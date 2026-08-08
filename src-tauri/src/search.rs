//! Filtering, scoring and sorting over the loaded rows — the hot path.
//!
//! This used to live in the frontend (`src/lib/utils/filter.ts` and friends),
//! which meant every keystroke filtered a quarter of a million rows in
//! JavaScript on the UI thread, and every save load shipped all of them over
//! IPC. The rows now stay in Rust and the frontend asks for pages; the rules
//! here are a line-for-line port of the TypeScript ones, and every "an
//! unknown is not a low value" decision is kept: a missing reading fails a
//! bound rather than passing it at zero.

use serde::{Deserialize, Serialize};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization as _;

use crate::commands::PlayerRow;
use crate::shortlist::{ProfileKind, ScoringProfile};

// Attribute indices, mirrored from `crates/fm-save/src/ability.rs` naming —
// the same constants the frontend's `attributes.ts` carries.
const PENALTY_TAKING: usize = 8;
const CORNERS: usize = 27;
const LONG_THROWS: usize = 30;
const FREE_KICK_TAKING: usize = 35;
const DIRTINESS: usize = 41;
const CONSISTENCY: usize = 44;
const IMPORTANT_MATCHES: usize = 47;
const INJURY_PRONENESS: usize = 48;
const VERSATILITY: usize = 49;
const DETERMINATION: usize = 51;

/// The tendency half of the non-player sheet ends here; an aged save rewrites
/// it onto an undecoded scale, so a staff profile skips those slots when any
/// value has left 1-20.
const STAFF_COACHING_FROM: usize = 26;

/// Position-rating thresholds: a natural position, and one they can fill.
const NATURAL: u8 = 15;
const ACCOMPLISHED: u8 = 10;

/// Weights below this are treated as absent — a slider dragged to zero
/// removes the attribute rather than dragging the average down.
const MIN_WEIGHT: f64 = 0.01;

/// The filter bar, exactly as the frontend holds it (camelCase over IPC).
/// Every field's absence rule matches the TypeScript original.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Filters {
    pub query: String,
    pub max_age: Option<u16>,
    pub min_ability: Option<u16>,
    pub max_ability: Option<u16>,
    pub min_potential: Option<u16>,
    pub max_potential: Option<u16>,
    pub kind: Option<String>,
    pub staff_role: Option<String>,
    pub position: Option<String>,
    pub position_tier: Option<String>,
    pub shortlist: Option<String>,
    pub nation_id: Option<u16>,
    pub gender: Option<String>,
    pub contract: Option<String>,
    pub min_score: Option<f64>,
    pub expiry_cutoff: Option<String>,
    pub min_headroom: Option<i32>,
    pub risk: Option<String>,
    pub max_wage: Option<u32>,
    pub min_versatility: Option<u8>,
    pub min_positions: Option<usize>,
    pub set_piece: Option<String>,
    pub min_set_piece: Option<u8>,
    pub min_reputation: Option<u16>,
    pub min_professionalism: Option<u8>,
    pub min_ambition: Option<u8>,
    /// Positions the active tactic asks for. A row passes when it plays any of
    /// them, at the tier `position_tier` sets — the "fit my tactic" search.
    pub tactic_positions: Option<Vec<String>>,
}

/// One page of results: the true total, the rows the table renders, and each
/// row's score under the active profile (null when unscoreable).
#[derive(Debug, Serialize)]
pub struct SearchPage {
    pub total: usize,
    pub rows: Vec<PlayerRow>,
    pub scores: Vec<Option<f64>>,
}

/// A matching player's identity, for writing results into a shortlist
/// without shipping the rows.
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub eid: u32,
    pub name: String,
}

/// What a search needs beyond the rows: the members of the shortlist being
/// filtered to, and the save's own position slot ordering.
pub struct Context {
    pub shortlist_eids: std::collections::HashSet<u32>,
    pub position_slots: std::collections::HashMap<String, usize>,
}

/// Case- and accent-insensitive fold, the same rule as the frontend's
/// `normalise`: NFD, drop combining marks, lowercase — "mbappe" finds
/// "Mbappé".
fn normalise(value: &str) -> String {
    value
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// The row's Current Ability: a player's own, or the non-player CA from the
/// staff sheet. `None` when neither is decoded, which is not a low one.
fn ability_of(row: &PlayerRow) -> Option<u16> {
    row.ability
        .map(u16::from)
        .or_else(|| row.staff.as_ref().map(|s| s.current_ability))
}

fn potential_of(row: &PlayerRow) -> Option<u16> {
    row.potential
        .map(u16::from)
        .or_else(|| row.staff.as_ref().map(|s| s.potential_ability))
}

/// Room left to grow. `None` when either end is undecoded.
fn headroom(row: &PlayerRow) -> Option<i32> {
    Some(i32::from(potential_of(row)?) - i32::from(ability_of(row)?))
}

/// Whether this row can be judged for flags at all — staff and stubs carry
/// no attribute block, and no reading is not a clean sheet.
fn has_flag_data(row: &PlayerRow) -> bool {
    !row.attributes.is_empty() || row.professionalism.is_some()
}

/// One flag rule: the value read off the row, the direction, and the line it
/// has to cross. Mirrors `flags.ts` — risks only, since the filter counts
/// risks; the strength chips never gate a search.
fn risk_count(row: &PlayerRow) -> usize {
    let attribute = |i: usize| row.attributes.get(i).copied();
    let low = |v: Option<u8>, limit: u8| v.is_some_and(|v| v <= limit);
    let high = |v: Option<u8>, limit: u8| v.is_some_and(|v| v >= limit);
    [
        high(attribute(INJURY_PRONENESS), 15),
        low(attribute(CONSISTENCY), 8),
        low(attribute(IMPORTANT_MATCHES), 8),
        low(attribute(DETERMINATION), 8),
        high(attribute(DIRTINESS), 15),
        low(row.professionalism, 7),
        low(row.temperament, 6),
        high(row.controversy, 15),
        low(row.pressure, 6),
        low(row.ambition, 6),
    ]
    .iter()
    .filter(|fired| **fired)
    .count()
}

/// The weighted mean of the person's attributes under a profile, on the same
/// 1-20 scale as the attributes. `None` when the person does not carry the
/// sheet the profile weights — an unscoreable person is not a zero.
#[must_use]
pub fn score(row: &PlayerRow, profile: &ScoringProfile) -> Option<f64> {
    let values: &[u8] = match profile.kind {
        ProfileKind::Staff => &row.staff.as_ref()?.attributes,
        ProfileKind::Player => {
            if row.attributes.is_empty() {
                return None;
            }
            &row.attributes
        }
    };
    if values.is_empty() {
        return None;
    }
    // A number on an unknown scale must not enter a 1-20 weighted mean.
    let tendencies_decoded = profile.kind != ProfileKind::Staff
        || values
            .get(..STAFF_COACHING_FROM.min(values.len()))
            .is_some_and(|half| half.iter().all(|v| *v <= 20));

    let mut total = 0.0;
    let mut weight = 0.0;
    for (key, w) in &profile.weights {
        if *w < MIN_WEIGHT {
            continue;
        }
        let Ok(index) = key.parse::<usize>() else {
            continue;
        };
        if profile.kind == ProfileKind::Staff && index < STAFF_COACHING_FROM && !tendencies_decoded
        {
            continue;
        }
        let Some(value) = values.get(index) else {
            continue;
        };
        if *value > 20 {
            continue;
        }
        total += f64::from(*value) * w;
        weight += w;
    }
    if weight == 0.0 {
        return None;
    }
    Some((total / weight * 10.0).round() / 10.0)
}

/// The set-piece attribute index for a key, `None` for an unknown key — a
/// preset from another build must not silently filter on index 0.
fn set_piece_index(key: &str) -> Option<usize> {
    match key {
        "corners" => Some(CORNERS),
        "freekicks" => Some(FREE_KICK_TAKING),
        "penalties" => Some(PENALTY_TAKING),
        "longthrows" => Some(LONG_THROWS),
        _ => None,
    }
}

fn tier_threshold(tier: Option<&str>) -> u8 {
    if tier == Some("accomplished") {
        ACCOMPLISHED
    } else {
        NATURAL
    }
}

/// Whether a row passes the bar. A line-for-line port of `matches` in
/// `filter.ts`; the comments there carry the reasoning and are not repeated.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn matches(
    row: &PlayerRow,
    filters: &Filters,
    row_score: Option<f64>,
    context: &Context,
) -> bool {
    if filters.shortlist.is_some() {
        let Some(eid) = row.eid else { return false };
        if !context.shortlist_eids.contains(&eid) {
            return false;
        }
    }
    if let Some(min) = filters.min_score {
        match row_score {
            Some(s) if s >= min => {}
            _ => return false,
        }
    }
    let kind = filters.kind.as_deref().unwrap_or("all");
    if kind == "players" && !row.is_player {
        return false;
    }
    if kind == "staff" && row.is_player {
        return false;
    }
    if kind == "staff" {
        let role = filters.staff_role.as_deref().unwrap_or("any");
        if role != "any" && row.staff_role.as_deref() != Some(role) {
            return false;
        }
    }
    if let Some(position) = filters.position.as_deref() {
        if filters.position_tier.as_deref() == Some("accomplished") {
            let Some(&slot) = context.position_slots.get(position) else {
                return false;
            };
            let Some(rating) = row.position_ratings.get(slot) else {
                return false;
            };
            if *rating < ACCOMPLISHED {
                return false;
            }
        } else if !row.positions.iter().any(|p| p == position) {
            return false;
        }
    }
    if let Some(wanted) = filters.tactic_positions.as_deref() {
        // A player fits when they play any position the tactic asks for. At the
        // "can cover" tier a 10+ rating in the slot counts; otherwise it must be
        // one of their natural positions — an unread rating fails the bound, it
        // does not pass it at zero.
        let accomplished = filters.position_tier.as_deref() == Some("accomplished");
        let fits = wanted.iter().any(|position| {
            if accomplished {
                context
                    .position_slots
                    .get(position)
                    .and_then(|&slot| row.position_ratings.get(slot))
                    .is_some_and(|rating| *rating >= ACCOMPLISHED)
            } else {
                row.positions.iter().any(|p| p == position)
            }
        });
        if !fits {
            return false;
        }
    }
    if let Some(nation) = filters.nation_id {
        if row.nation_id != Some(nation) {
            return false;
        }
    }
    match filters.gender.as_deref() {
        Some("men") if row.female != Some(false) => return false,
        Some("women") if row.female != Some(true) => return false,
        _ => {}
    }
    match filters.contract.as_deref() {
        Some("free") => {
            if row.wage.is_some() || !row.club.is_empty() || !row.contract_until.is_empty() {
                return false;
            }
        }
        Some("expiring") => {
            if row.contract_until.is_empty() {
                return false;
            }
            if let Some(cutoff) = filters.expiry_cutoff.as_deref() {
                if row.contract_until.as_str() > cutoff {
                    return false;
                }
            }
        }
        _ => {}
    }
    if let Some(max_age) = filters.max_age {
        match row.age {
            Some(age) if age <= max_age => {}
            _ => return false,
        }
    }
    if filters.min_ability.is_some() || filters.max_ability.is_some() {
        let Some(current) = ability_of(row) else {
            return false;
        };
        if filters.min_ability.is_some_and(|min| current < min) {
            return false;
        }
        if filters.max_ability.is_some_and(|max| current > max) {
            return false;
        }
    }
    if filters.min_potential.is_some() || filters.max_potential.is_some() {
        let Some(potential) = potential_of(row) else {
            return false;
        };
        if filters.min_potential.is_some_and(|min| potential < min) {
            return false;
        }
        if filters.max_potential.is_some_and(|max| potential > max) {
            return false;
        }
    }
    if let Some(max_wage) = filters.max_wage {
        match row.wage {
            Some(wage) if wage <= max_wage => {}
            _ => return false,
        }
    }
    if let Some(min) = filters.min_versatility {
        match row.attributes.get(VERSATILITY) {
            Some(v) if *v >= min => {}
            _ => return false,
        }
    }
    if let Some(min) = filters.min_positions {
        if row.position_ratings.is_empty() {
            return false;
        }
        let threshold = tier_threshold(filters.position_tier.as_deref());
        let covered = row.position_ratings.iter().filter(|r| **r >= threshold).count();
        if covered < min {
            return false;
        }
    }
    if let Some(min) = filters.min_reputation {
        match world_reputation(row) {
            Some(rep) if rep >= min => {}
            _ => return false,
        }
    }
    if let Some(min) = filters.min_professionalism {
        match row.professionalism {
            Some(v) if v >= min => {}
            _ => return false,
        }
    }
    if let Some(min) = filters.min_ambition {
        match row.ambition {
            Some(v) if v >= min => {}
            _ => return false,
        }
    }
    if let (Some(piece), Some(min)) = (filters.set_piece.as_deref(), filters.min_set_piece) {
        let Some(index) = set_piece_index(piece) else {
            return false;
        };
        match row.attributes.get(index) {
            Some(v) if *v >= min => {}
            _ => return false,
        }
    }
    if let Some(min) = filters.min_headroom {
        match headroom(row) {
            Some(room) if room >= min => {}
            _ => return false,
        }
    }
    match filters.risk.as_deref() {
        Some("clean") if !has_flag_data(row) || risk_count(row) > 0 => return false,
        Some("flagged") if !has_flag_data(row) || risk_count(row) == 0 => return false,
        _ => {}
    }
    let query = filters.query.trim();
    if query.is_empty() {
        return true;
    }
    let needle = normalise(query);
    normalise(&row.name).contains(&needle) || normalise(&row.club).contains(&needle)
}

/// The number a sort key stands for, or `None` when this person has none of
/// it — unknowns sort last in both directions.
fn sort_value(row: &PlayerRow, key: &str, row_score: Option<f64>) -> Option<f64> {
    match key {
        "score" => row_score,
        "headroom" => headroom(row).map(f64::from),
        "ability" => ability_of(row).map(f64::from),
        "potential" => potential_of(row).map(f64::from),
        "reputation" => world_reputation(row).map(f64::from),
        "age" => row.age.map(f64::from),
        _ => None,
    }
}

/// Worldwide reputation, 0-200, for whichever kind of row this is: the
/// non-player sheet for staff, or the player line — the two never both carry
/// one, so there is no kind to prefer over the other.
fn world_reputation(row: &PlayerRow) -> Option<u16> {
    row.staff
        .as_ref()
        .map(|s| s.world_reputation)
        .or_else(|| row.reputation.as_ref().map(|r| r.world))
}

/// One matching row with its precomputed sort keys, so the comparator does
/// no work per comparison and nothing is indexed.
struct Hit<'a> {
    row: &'a PlayerRow,
    score: Option<f64>,
    sort: Option<f64>,
    name: String,
}

/// Filters, scores and sorts the loaded rows, returning one page. Scores are
/// computed once per row: the filter, the sort and the returned page all
/// read the same number.
#[must_use]
pub fn run(
    players: &[PlayerRow],
    filters: &Filters,
    sort_key: &str,
    sort_direction: &str,
    profile: Option<&ScoringProfile>,
    context: &Context,
    limit: usize,
) -> SearchPage {
    let mut hits: Vec<Hit> = players
        .iter()
        .filter_map(|row| {
            let row_score = profile.and_then(|p| score(row, p));
            if !matches(row, filters, row_score, context) {
                return None;
            }
            Some(Hit {
                row,
                score: row_score,
                sort: sort_value(row, sort_key, row_score),
                name: normalise(&row.name),
            })
        })
        .collect();

    let descending = sort_direction != "asc";
    if sort_key == "name" {
        // Stubs have no name, and an empty string must not lead the
        // alphabet: the nameless sink to the bottom in both directions.
        hits.sort_by(|a, b| {
            match (a.row.name.is_empty(), b.row.name.is_empty()) {
                (true, false) => return std::cmp::Ordering::Greater,
                (false, true) => return std::cmp::Ordering::Less,
                _ => {}
            }
            let ordered = a.name.cmp(&b.name);
            if descending { ordered.reverse() } else { ordered }
        });
    } else {
        hits.sort_by(|a, b| match (a.sort, b.sort) {
            (None, None) => a.name.cmp(&b.name),
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(left), Some(right)) => {
                let ordered = left.partial_cmp(&right).unwrap_or(std::cmp::Ordering::Equal);
                if descending { ordered.reverse() } else { ordered }
            }
        });
    }

    let total = hits.len();
    SearchPage {
        total,
        rows: hits.iter().take(limit).map(|h| h.row.clone()).collect(),
        scores: hits.iter().take(limit).map(|h| h.score).collect(),
    }
}

/// Every matching player's identity, for "add these results to a shortlist".
/// Staff rows and rows without an entity id are skipped — FM's in-save
/// shortlists hold players.
#[must_use]
pub fn matching_eids(
    players: &[PlayerRow],
    filters: &Filters,
    profile: Option<&ScoringProfile>,
    context: &Context,
) -> Vec<SearchHit> {
    players
        .iter()
        .filter(|row| {
            let row_score = profile.and_then(|p| score(row, p));
            matches(row, filters, row_score, context)
        })
        .filter(|row| row.is_player || row.staff.is_none())
        .filter_map(|row| {
            Some(SearchHit {
                eid: row.eid?,
                name: row.name.clone(),
            })
        })
        .collect()
}
