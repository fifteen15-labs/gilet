//! Player attributes, Current Ability and Potential Ability.
//!
//! These are not in the person record. FM stores a separate attribute block —
//! 54 bytes, each an attribute on the 1-20 scale multiplied by 5 — and CA/PA
//! sit just before it. The block precedes its person's name by roughly
//! 930-1100 bytes, so the block is the anchor and the person is matched to it,
//! not the other way round.
//!
//! Only players have a block; staff do not. That absence is what separates the
//! two, which nothing in the person record does.

/// Attributes per block. FM's full set across technical, mental, physical and
/// goalkeeping.
pub const ATTRIBUTE_COUNT: usize = 54;

/// Indices of the goalkeeping attributes.
///
/// Found empirically rather than assumed: at each of these, ~90% of players sit
/// at 3 or below while a minority score highly — the signature of an attribute
/// only keepers have. There are exactly 11, matching FM's goalkeeping set.
/// The remaining 43 are outfield attributes whose individual names are not yet
/// mapped, so they are presented by index rather than guessed at.
pub const GOALKEEPING_INDICES: [usize; 11] = [11, 12, 13, 14, 15, 16, 19, 21, 31, 32, 33];

/// Whether an attribute index belongs to the goalkeeping set.
#[must_use]
pub fn is_goalkeeping(index: usize) -> bool {
    GOALKEEPING_INDICES.contains(&index)
}

/// Names inferred for some attribute indices.
///
/// The format does not label its attributes. Two generations of evidence:
///
/// **Statistical** (the first fourteen): which well-known players top each
/// index, and how the mean varies by the player's strongest position — an
/// index was only named when both agreed. That method separated Heading (3)
/// from Jumping Reach (39) via goalkeepers, and gave Marking (5) against
/// Tackling (9) directionally.
///
/// **Ground truth** (the rest, plus one correction): the in-game player
/// report for Jamal Musiala in an aged save, checked against his decoded
/// block. All thirteen statistically-named outfield indices matched his
/// screen exactly, which validates both eras at once. Indices whose displayed
/// value appeared exactly once in his block are named outright:
///
/// - 26 **Flair** (his 19), 27 **Corners** (10), 35 **Free Kick Taking** (11)
/// - 40 **Leadership** (9) — *correcting* the earlier "Aggression" label;
///   Otamendi, Ronaldo and Freuler topping it fits captains as well as
///   aggressors, and the ground truth is unambiguous
/// - the long-stuck 34/38 pair split: his screen shows Acceleration 14 and
///   Pace 15, the pair reads {14, 15}, so **34 = Acceleration, 38 = Pace**,
///   and Work Rate's 15 lands on **29** as the only remaining candidate
///
/// Marking (5) at 12 and Tackling (9) at 11 matched his screen too, so the
/// old "if this is wrong they are swapped" caveat is closed.
///
/// Still ambiguous — several indices share a displayed value on his screen:
/// {7, 22} is First Touch / Vision, {43, 53} is Bravery / Concentration,
/// {24, 45} holds Aggression and a hidden attribute, and the 18/16/14 pools
/// hold Dribbling, Composure, Agility, Anticipation, Decisions,
/// Determination, Balance, Long Shots, Teamwork, Natural Fitness and Stamina
/// among hidden attributes. A report for one more player with different
/// values would break most of those ties.
///
/// These are inferences, not values read from the file.
#[must_use]
pub fn attribute_name(index: usize) -> Option<&'static str> {
    match index {
        0 => Some("Crossing"),
        2 => Some("Finishing"),
        3 => Some("Heading"),
        5 => Some("Marking"),
        6 => Some("Off the Ball"),
        8 => Some("Penalty Taking"),
        9 => Some("Tackling"),
        10 => Some("Passing"),
        20 => Some("Positioning"),
        23 => Some("Technique"),
        26 => Some("Flair"),
        27 => Some("Corners"),
        29 => Some("Work Rate"),
        30 => Some("Long Throws"),
        34 => Some("Acceleration"),
        35 => Some("Free Kick Taking"),
        36 => Some("Strength"),
        38 => Some("Pace"),
        39 => Some("Jumping Reach"),
        40 => Some("Leadership"),
        _ => None,
    }
}

/// Attributes are stored on an internal 1-100 scale; the displayed 1-20 value
/// is the internal one divided by five, rounded to nearest. On a freshly
/// started save every internal value is an exact multiple of five (display
/// times five), but training and decline move them off the multiples — an
/// aged save has none left, which is why the byte test cannot require
/// divisibility.
const SCALE: u8 = 5;
const MIN_ATTR: u8 = 1;
const MAX_ATTR: u8 = 100;

/// Position ratings stay on the raw 1-20 scale.
const MAX_POSITION: u8 = 20;

/// Bytes back from the start of the block.
const CA_BACK: usize = 39;
const PA_BACK: usize = 37;

/// Position ratings sit in the 15 bytes immediately before the attribute
/// block, each 1-20 for how naturally the player plays there.
pub const POSITION_COUNT: usize = 15;

/// Slot order, established from players whose real position is unambiguous:
/// Alisson tops slot 0, Rúben Dias slot 3, Kyle Walker slot 4 (and 14),
/// Bruno Fernandes slot 10, Saka slot 11, Haaland and Kane slot 12.
///
/// The population backs it up — slot 3 is the most common strongest position
/// at 19.7% (centre-backs), then striker at 16.4% and centre-mid at 14.3%,
/// while sweeper is nobody's best position, as it should be in a modern
/// database.
pub const POSITION_NAMES: [&str; POSITION_COUNT] = [
    "GK", "SW", "DL", "DC", "DR", "DM", "ML", "MC", "MR", "AML", "AMC", "AMR", "ST", "WBL", "WBR",
];

/// A rating of 15 or better is a position the player is genuinely comfortable
/// in, which is the threshold used to describe them.
const NATURAL: u8 = 15;

/// The furthest a block sits before the person it belongs to.
///
/// A block always precedes its owner, so the owner is simply the next person
/// record — the bound only rejects an orphan block with no person after it.
/// The distance is far more variable than it first appears: the median is
/// about 1,200 bytes but the 99th percentile is near 29,000, so a tight bound
/// silently drops thousands of players.
const MAX_BLOCK_TO_NAME: usize = 50_000;

/// Ability ratings are on a 1-200 scale, with 200 the hard ceiling for
/// Potential Ability.
const MAX_ABILITY: u8 = 200;

/// One player's ability data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ability {
    /// Offset of the attribute block within the frame.
    pub block_offset: usize,
    /// Current Ability, 1-200.
    pub current: u8,
    /// Potential Ability, 1-200. Never below `current`.
    pub potential: u8,
    /// The 54 attributes, converted back to the 1-20 scale FM displays.
    pub attributes: [u8; ATTRIBUTE_COUNT],
    /// How well the player plays each of the 15 positions, 1-20.
    pub positions: [u8; POSITION_COUNT],
}

impl Ability {
    /// Positions the player is genuinely comfortable in, strongest first.
    /// Empty when they have no natural position.
    #[must_use]
    pub fn natural_positions(&self) -> Vec<&'static str> {
        let mut rated: Vec<(u8, &'static str)> = self
            .positions
            .iter()
            .zip(POSITION_NAMES)
            .filter(|(v, _)| **v >= NATURAL)
            .map(|(v, name)| (*v, name))
            .collect();
        rated.sort_by_key(|(v, _)| std::cmp::Reverse(*v));
        rated.into_iter().map(|(_, name)| name).collect()
    }
}

fn is_attribute_byte(b: u8) -> bool {
    (MIN_ATTR..=MAX_ATTR).contains(&b)
}

/// Finds every attribute block in a frame and reads the ability values in
/// front of it.
///
/// A bare run of 1-100 bytes is no signature at all, so the block is
/// recognised by its whole surrounding structure at once: fifteen position
/// bytes each 1-20 immediately before it, Current and Potential Ability at
/// fixed offsets in front with `1 <= CA <= PA <= 200`, and all 54 attribute
/// bytes in 1-100. Matches step forward a full block so one block is never
/// reported twice at overlapping offsets.
#[must_use]
pub fn scan_abilities(frame: &[u8]) -> Vec<Ability> {
    let mut out = Vec::new();
    let mut at = CA_BACK;

    while at + ATTRIBUTE_COUNT <= frame.len() {
        match read_block(frame, at) {
            Some(ability) => {
                out.push(ability);
                at += ATTRIBUTE_COUNT;
            }
            None => at += 1,
        }
    }

    out
}

fn read_block(frame: &[u8], start: usize) -> Option<Ability> {
    // Position ratings come first in the test: the fifteen bytes before the
    // block are each 1-20, which rejects almost every offset within a byte
    // or two and keeps the whole-frame scan cheap.
    let raw_positions = frame.get(start.checked_sub(POSITION_COUNT)?..start)?;
    if !raw_positions.iter().all(|&b| (1..=MAX_POSITION).contains(&b)) {
        return None;
    }

    let current = *frame.get(start.checked_sub(CA_BACK)?)?;
    let potential = *frame.get(start.checked_sub(PA_BACK)?)?;

    // Both are 1-200 and a player can never exceed their own ceiling. A block
    // failing this is a false positive, not a player with odd data.
    if current == 0 || current > MAX_ABILITY || potential > MAX_ABILITY || potential < current {
        return None;
    }

    let raw = frame.get(start..start.checked_add(ATTRIBUTE_COUNT)?)?;
    if !raw.iter().all(|&b| is_attribute_byte(b)) {
        return None;
    }
    let mut attributes = [0u8; ATTRIBUTE_COUNT];
    for (slot, &b) in attributes.iter_mut().zip(raw) {
        // Round to nearest: 66 displays as 13, 68 as 14. Exact for the
        // multiples of five a fresh save holds. The display floor is 1 —
        // an internal 1 or 2 would otherwise round to a nonexistent 0.
        *slot = ((b + SCALE / 2) / SCALE).max(1);
    }

    let mut positions = [0u8; POSITION_COUNT];
    for (slot, &b) in positions.iter_mut().zip(raw_positions) {
        *slot = b;
    }

    Some(Ability {
        block_offset: start,
        current,
        potential,
        attributes,
        positions,
    })
}

/// Matches each attribute block to the person it belongs to.
///
/// `people_offsets` must be the sorted record offsets of the people in the same
/// frame. Returns, for each ability, the index into that slice — the first
/// person starting after the block.
///
/// More than one block can resolve to the same person, because the person scan
/// does not find every record. Where that happens the nearest block wins:
/// letting a distant one overwrite a close one reassigns ability between
/// players, which is silently wrong rather than obviously broken.
#[must_use]
pub fn match_to_people(abilities: &[Ability], people_offsets: &[usize]) -> Vec<Option<usize>> {
    // Best (smallest) distance seen so far for each person index.
    let mut claimed: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut proposals: Vec<Option<(usize, usize)>> = Vec::with_capacity(abilities.len());

    for a in abilities {
        let idx = people_offsets.partition_point(|&o| o <= a.block_offset);
        let proposal = people_offsets.get(idx).and_then(|offset| {
            let distance = offset.saturating_sub(a.block_offset);
            (distance <= MAX_BLOCK_TO_NAME).then_some((idx, distance))
        });
        if let Some((person, distance)) = proposal {
            claimed
                .entry(person)
                .and_modify(|best| *best = (*best).min(distance))
                .or_insert(distance);
        }
        proposals.push(proposal);
    }

    proposals
        .into_iter()
        .map(|p| {
            let (person, distance) = p?;
            (claimed.get(&person) == Some(&distance)).then_some(person)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Builds a block with the ability bytes in front of it, matching the
    /// on-disk layout.
    fn block(current: u8, potential: u8, attrs: [u8; ATTRIBUTE_COUNT]) -> Vec<u8> {
        block_at(current, potential, attrs, [1u8; POSITION_COUNT])
    }

    fn block_at(
        current: u8,
        potential: u8,
        attrs: [u8; ATTRIBUTE_COUNT],
        positions: [u8; POSITION_COUNT],
    ) -> Vec<u8> {
        let mut v = vec![0u8; 64];
        let start = v.len();
        *v.get_mut(start - CA_BACK).unwrap() = current;
        *v.get_mut(start - PA_BACK).unwrap() = potential;
        // Positions sit immediately before the attribute block.
        for (i, p) in positions.iter().enumerate() {
            *v.get_mut(start - POSITION_COUNT + i).unwrap() = *p;
        }
        v.extend(attrs.iter().map(|a| a * SCALE));
        // Terminate the run so the scanner sees its end.
        v.push(0xFF);
        v
    }

    fn flat(value: u8) -> [u8; ATTRIBUTE_COUNT] {
        [value; ATTRIBUTE_COUNT]
    }

    #[test]
    fn reads_current_and_potential_ability() {
        // Haaland's real values in the reference save.
        let buf = block(184, 195, flat(14));
        let found = scan_abilities(&buf);
        assert_eq!(found.len(), 1);
        let a = found.first().unwrap();
        assert_eq!((a.current, a.potential), (184, 195));
    }

    #[test]
    fn converts_attributes_back_to_the_twenty_point_scale() {
        let mut attrs = flat(10);
        attrs[0] = 20;
        attrs[1] = 1;
        let found = scan_abilities(&block(150, 170, attrs));
        let a = found.first().unwrap();
        assert_eq!(a.attributes[0], 20, "100 on disk is 20 displayed");
        assert_eq!(a.attributes[1], 1, "5 on disk is 1 displayed");
        assert_eq!(a.attributes.len(), ATTRIBUTE_COUNT);
    }

    #[test]
    fn rejects_a_block_whose_potential_is_below_its_current() {
        // Impossible in FM, so the run is not a real attribute block.
        assert!(scan_abilities(&block(180, 120, flat(12))).is_empty());
    }

    #[test]
    fn rejects_ability_above_the_two_hundred_ceiling() {
        assert!(scan_abilities(&block(210, 220, flat(12))).is_empty());
    }

    #[test]
    fn ignores_runs_that_are_not_exactly_the_attribute_count() {
        // Staff carry shorter runs; treating them as blocks would read ability
        // from the wrong offsets.
        let mut v = vec![0u8; 64];
        v.extend(std::iter::repeat_n(50u8, ATTRIBUTE_COUNT - 10));
        v.push(0xFF);
        assert!(scan_abilities(&v).is_empty());
    }

    #[test]
    fn matches_a_block_to_the_person_that_follows_it() {
        let abilities = scan_abilities(&block(184, 195, flat(14)));
        let a = abilities.first().unwrap();
        // One person just after the block, one far away.
        let offsets = vec![a.block_offset + 950, a.block_offset + 900_000];
        assert_eq!(match_to_people(&abilities, &offsets), vec![Some(0)]);
    }

    #[test]
    fn leaves_a_block_unmatched_when_no_person_follows_it() {
        let abilities = scan_abilities(&block(184, 195, flat(14)));
        let a = abilities.first().unwrap();
        // Far beyond any plausible record, and nothing after it.
        let offsets = vec![a.block_offset + MAX_BLOCK_TO_NAME + 1];
        assert_eq!(match_to_people(&abilities, &offsets), vec![None]);
    }

    #[test]
    fn the_nearest_block_wins_when_two_resolve_to_one_person() {
        // Two blocks, one person after both. Letting the far block overwrite
        // the near one would attribute the wrong player's ability.
        let mut buf = block(120, 140, flat(9));
        let far_offset = scan_abilities(&buf).first().unwrap().block_offset;
        buf.extend(std::iter::repeat_n(0u8, 400));
        buf.extend(block(184, 195, flat(14)));

        let abilities = scan_abilities(&buf);
        assert_eq!(abilities.len(), 2, "expected two blocks");
        let near = abilities.get(1).unwrap().block_offset;
        assert!(near > far_offset);

        let person = near + 900;
        let matched = match_to_people(&abilities, &[person]);
        assert_eq!(matched, vec![None, Some(0)], "the closer block should own the person");
    }

    #[test]
    fn reads_position_ratings() {
        // A keeper: 20 in slot 0, nothing anywhere else.
        let mut keeper = [1u8; POSITION_COUNT];
        keeper[0] = 20;
        let found = scan_abilities(&block_at(180, 182, flat(11), keeper));
        let a = found.first().unwrap();
        assert_eq!(a.positions[0], 20);
        assert_eq!(a.natural_positions(), vec!["GK"]);
    }

    #[test]
    fn lists_natural_positions_strongest_first() {
        // Walker's shape: right-back first, then centre-back and wing-back.
        let mut walker = [1u8; POSITION_COUNT];
        walker[4] = 20;
        walker[3] = 18;
        walker[14] = 16;
        walker[2] = 10; // below the threshold, so not "natural"
        let found = scan_abilities(&block_at(147, 164, flat(12), walker));
        assert_eq!(found.first().unwrap().natural_positions(), vec!["DR", "DC", "WBR"]);
    }

    #[test]
    fn a_player_with_no_strong_position_lists_none() {
        let found = scan_abilities(&block(100, 120, flat(10)));
        assert!(found.first().unwrap().natural_positions().is_empty());
    }

    #[test]
    fn names_only_the_attributes_the_evidence_supports() {
        assert_eq!(attribute_name(2), Some("Finishing"));
        assert_eq!(attribute_name(30), Some("Long Throws"));
        // Goalkeepers separate heading from jumping: they jump constantly and
        // head almost never.
        assert_eq!(attribute_name(3), Some("Heading"));
        assert_eq!(attribute_name(39), Some("Jumping Reach"));
        // Ground truth from Musiala's in-game report split the 34/38 pair:
        // his screen showed Acceleration 14 and Pace 15 and the pair read
        // {14, 15}.
        assert_eq!(attribute_name(34), Some("Acceleration"));
        assert_eq!(attribute_name(38), Some("Pace"));
        // 40 was "Aggression" until the same report read 9 there, matching
        // his Leadership exactly.
        assert_eq!(attribute_name(40), Some("Leadership"));
        // First Touch and Vision both showed 17, so 7 and 22 stay unnamed.
        for ambiguous in [7, 22] {
            assert_eq!(attribute_name(ambiguous), None, "index {ambiguous} is not separable");
        }
    }

    #[test]
    fn rounds_aged_save_internals_to_the_nearest_display_value() {
        // Training moves internals off the multiples of five a fresh save
        // holds: Musiala's Crossing is stored as 66 and displayed 13, his
        // Penalty Taking as 68 and displayed 14.
        let mut buf = vec![0u8; 64];
        let start = buf.len();
        *buf.get_mut(start - CA_BACK).unwrap() = 180;
        *buf.get_mut(start - PA_BACK).unwrap() = 185;
        for i in 0..POSITION_COUNT {
            *buf.get_mut(start - POSITION_COUNT + i).unwrap() = 1;
        }
        let mut attrs = [50u8; ATTRIBUTE_COUNT];
        attrs[0] = 66;
        attrs[1] = 68;
        attrs[2] = 1;
        buf.extend_from_slice(&attrs);
        buf.push(0xFF);

        let found = scan_abilities(&buf);
        assert_eq!(found.len(), 1);
        let a = found.first().unwrap();
        assert_eq!(a.attributes[0], 13, "66 rounds down to 13");
        assert_eq!(a.attributes[1], 14, "68 rounds up to 14");
        assert_eq!(a.attributes[2], 1, "1 displays as 1, not 0");
    }

    #[test]
    fn index_six_is_not_heading() {
        // Guards a corrected mistake: centre-backs score 8.5 here against
        // strikers' 12.0, which rules Heading out.
        assert_ne!(attribute_name(6), Some("Heading"));
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let full = block(184, 195, flat(14));
        for cut in 0..full.len() {
            let _ = scan_abilities(full.get(..cut).unwrap());
        }
    }
}
