//! Commands exposed to the frontend.
//!
//! These live in their own module because `tauri::generate_handler!` imports
//! each command into the module that invokes it, which collides if the handler
//! list sits alongside the definitions.

use serde::{Deserialize, Serialize};
use tauri::Manager as _;

use crate::CommandError;

/// Where the file dialogs should start.
///
/// Resolved in Rust so the paths follow each platform's own conventions rather
/// than the frontend guessing at them.
#[derive(Debug, Clone, Serialize)]
pub struct Locations {
    /// Football Manager's own saves folder, when it exists.
    pub saves: Option<String>,
    /// Where a CSV should be written by default.
    pub documents: Option<String>,
}

/// Football Manager's own data folder for the newest installed version, e.g.
/// `…/Sports Interactive/Football Manager 26`.
///
/// FM keeps this under Application Support on macOS and in Documents on
/// Windows. `None` when FM is not installed, so callers fall back rather than
/// inventing a path that does not exist.
pub(crate) fn fm_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let path = app.path();
    let base = if cfg!(target_os = "macos") {
        path.data_dir().ok()?.join("Sports Interactive")
    } else {
        path.document_dir().ok()?.join("Sports Interactive")
    };
    if !base.is_dir() {
        return None;
    }
    // Prefer the newest Football Manager folder, so a machine with several
    // installed opens on the current one. A version folder is only a real one
    // if it holds saves, which rules out leftovers from an uninstall.
    let mut versions: Vec<_> = std::fs::read_dir(&base)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("games").is_dir())
        .collect();
    versions.sort();
    versions.pop().or(Some(base))
}

/// Where the open dialog should start. Only a directory that actually exists is
/// offered, so the dialog falls back to the system default rather than opening
/// on a missing path.
fn saves_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let dir = fm_dir(app)?;
    let games = dir.join("games");
    if games.is_dir() {
        return Some(games);
    }
    Some(dir)
}

/// Returns the directories the open and save dialogs should default to.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn default_locations(app: tauri::AppHandle) -> Locations {
    Locations {
        saves: saves_dir(&app).map(|p| p.display().to_string()),
        documents: app
            .path()
            .document_dir()
            .ok()
            .map(|p| p.display().to_string()),
    }
}

/// A person as the table renders them.
///
/// `ability` and `potential` are `Option` because only players carry an
/// attribute block; staff have none, which is what distinguishes the two.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a serialization row: each bool is an independent decoded fact, not a state machine"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRow {
    /// Byte offset of the record, stable within one save and used as the row key.
    pub id: usize,
    /// Person entity id, the save's own identifier — what shortlist edits key
    /// on. `None` for the few records whose identity block did not resolve.
    pub eid: Option<u32>,
    pub name: String,
    pub born: String,
    /// Age on the save's own date. `None` for stubs and compact entries,
    /// whose birth date the save does not carry — an unknown age is not a
    /// young one.
    pub age: Option<u16>,
    pub ability: Option<u8>,
    pub potential: Option<u8>,
    /// True when this person has ability data, i.e. is a player not staff.
    pub is_player: bool,
    /// The 54 attributes on FM's 1-20 scale. Empty for staff.
    pub attributes: Vec<u8>,
    /// Nation identifier, shared with the club records. `None` when the save
    /// carries none — stubs and compact entries.
    pub nation_id: Option<u16>,
    /// Nation name where the identifier is confirmed, otherwise empty.
    pub nation: String,
    /// Positions the player is comfortable in, strongest first. Empty for staff.
    pub positions: Vec<String>,
    /// Rating 1-20 for each of the 15 position slots. Empty for staff.
    pub position_ratings: Vec<u8>,
    /// Short name of the club this person belongs to, empty when unattached —
    /// free agents, national staff, unresolved records. Since the team-squad
    /// pass this covers B and youth players too, not only the first team.
    pub club: String,
    /// The club's entity id, alongside `club`. Two clubs can share a short
    /// name (a men's and women's side, most often), so a club *filter* keys
    /// on this rather than on the label — `None` alongside an empty `club`
    /// for the unattached, and also for the rare person whose club is a
    /// record with no validated entity head.
    pub club_eid: Option<u32>,
    /// Whether a first-team squad list carries this person. False for B and
    /// youth players, staff and the unattached — the flag that keeps a squad
    /// audit meaning the team that plays, not everyone wearing the badge.
    pub first_team: bool,
    /// Which of `club`'s own squad lists placed this person there — "First
    /// Team", "B Team", "Youth", or "Out of League" for a club outside the
    /// loaded leagues. `None` for staff, the unattached, and anyone whose club
    /// came from the backroom lists rather than a squad one.
    pub squad_level: Option<String>,
    /// Whether this person is a woman, inferred from the forename pool.
    /// `None` when the save has no women's football to derive the split from.
    pub female: Option<bool>,
    /// Weekly wage in the save's display currency. `None` when no contract
    /// was found — the unemployed and the retired.
    pub wage: Option<u32>,
    /// Contract expiry as `YYYY-MM-DD`, empty when unknown.
    pub contract_until: String,
    /// Hidden Adaptability, 1-20. `None` when the personality run is absent.
    pub adaptability: Option<u8>,
    /// Hidden Ambition, 1-20.
    pub ambition: Option<u8>,
    /// Hidden Loyalty, 1-20.
    pub loyalty: Option<u8>,
    /// Hidden Pressure, 1-20.
    pub pressure: Option<u8>,
    /// Hidden Professionalism, 1-20 — the development driver.
    pub professionalism: Option<u8>,
    /// Hidden Sportsmanship, 1-20.
    pub sportsmanship: Option<u8>,
    /// Hidden Temperament, 1-20.
    pub temperament: Option<u8>,
    /// Hidden Controversy, 1-20.
    pub controversy: Option<u8>,
    /// Non-player attributes — the pre-game editor's "All Attributes" sheet —
    /// read from the entity object one id below this person's own. `None` for
    /// anyone the save gives no such object, which is most players.
    pub staff: Option<StaffSheet>,
    /// Game reputation on the editor's 0-200 scale, bound only where the
    /// save's player line repeats this person's own CA/PA — `None` is an
    /// undecoded reading, never a nobody.
    pub reputation: Option<ReputationRow>,
    /// True for a stub — a non-contract squad filler the save stores without
    /// a person record. Identity and club are known; name, age and attributes
    /// are not decoded, and the row says so rather than vanishing.
    pub stub: bool,
    /// The person's decoded place in a club's backroom — "Manager" from the
    /// roster table's seat, "Director of Football" or "Board" from the
    /// boardroom run, or "Coaching" / "Medical" / "Recruitment" from the
    /// department staff lists. `None` for players, the unemployed, and
    /// staff bound outside the department triple.
    pub staff_role: Option<String>,
    /// Whether a national side's squad list names this person. False is
    /// "no decoded selection names them", not proof of being uncapped —
    /// the save only materialises the selections it has needed.
    pub in_national_squad: bool,
}

/// A person's three game reputations, 0-200 as the editor stores them.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReputationRow {
    /// Standing in their home nation.
    pub home: u16,
    /// Standing where they currently play.
    pub current: u16,
    /// Worldwide standing — the reputation that decides who takes your call.
    pub world: u16,
}

impl From<fm_save::person::Reputation> for ReputationRow {
    fn from(r: fm_save::person::Reputation) -> Self {
        Self { home: r.home, current: r.current, world: r.world }
    }
}

/// A person's non-player sheet, as `fm-save` decodes it.
///
/// The names are not sent per row — they are the same 52 for everyone, and
/// `staff_attribute_names` serves them once.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffSheet {
    /// The 52 attributes on FM's 1-20 scale, in the editor's own order.
    pub attributes: Vec<u8>,
    /// Reputation in their home nation, 0-200.
    pub home_reputation: u16,
    /// Reputation where they currently work, 0-200.
    pub current_reputation: u16,
    /// Worldwide reputation, 0-200.
    pub world_reputation: u16,
    /// Non-player Current Ability, 0-200.
    pub current_ability: u16,
    /// Non-player Potential Ability, 0-200.
    pub potential_ability: u16,
}

/// The editor's name for each of the 52 non-player attributes, in storage
/// order. Two are empty: the pre-game editor leaves those rows blank itself,
/// and a guessed name would be an invented one.
#[tauri::command]
#[must_use]
pub fn staff_attribute_names() -> Vec<String> {
    (0..fm_save::staff::ATTRIBUTE_COUNT)
        .map(|i| fm_save::staff::attribute_name(i).unwrap_or_default().to_owned())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubRow {
    pub id: usize,
    /// The club's entity id — what squad and roster records reference, and what
    /// the "my club" shortcuts match a row on. `None` for a record whose head
    /// did not validate.
    pub eid: Option<u32>,
    pub name: String,
    pub short_name: String,
    pub club_id: u32,
    pub nation_id: u32,
    /// How many players the squad table lists for this club.
    pub squad_size: usize,
    /// Mean Current Ability across the squad, rounded. `None` when the club
    /// fields no squad whose ability is decoded.
    pub average_ability: Option<u16>,
    /// Mean Potential Ability across the squad, rounded.
    pub average_potential: Option<u16>,
    /// Mean age of the squad players whose birth date decoded, to one decimal
    /// place. `None` when none did — an unknown age never enters the mean.
    pub average_age: Option<f32>,
    /// Weekly wage bill: the sum of the squad wages that decoded, in the
    /// save's display currency. `None` when no squad wage decoded.
    pub wage_bill: Option<u64>,
    /// How many of the squad had a decoded wage. The bill is a floor, not a
    /// total, whenever this is below `squad_size`, and the UI says so rather
    /// than presenting a partial sum as the club's outgoings.
    pub wages_known: usize,
    /// The club's reputation, 0-10000, from the roster table. `None` when the
    /// club has no roster row — undecoded, not a reputation of zero.
    pub reputation: Option<u16>,
}

/// The club the human manager runs, resolved from `humans.dat` through the
/// manager's own record. `None` means the human is unemployed — an
/// unemployed career's member reads exactly this way — or the save carries
/// no human at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyClub {
    /// The club's entity id — what squad and roster records key on, and what
    /// the frontend matches rows against.
    pub eid: u32,
    pub name: String,
    pub short_name: String,
    /// The manager's own person eid, so the UI can point at their record.
    pub manager_eid: u32,
    /// The manager's name, for the sidebar header.
    pub manager_name: String,
}

/// The human's active tactic, as `fm-save` reads it from `tactics_man.dat`.
///
/// Positions are the eleven slot names in team-sheet order; `starter_eids` is
/// index-aligned with them when a selection is stored. Roles and duties are
/// not decoded, so a tactic-fit search keys on positions and says so.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TacticRow {
    pub name: String,
    /// The style name ("Custom Gegenpress"), when one is set.
    pub style: Option<String>,
    /// Eleven position names in slot order.
    pub positions: Vec<String>,
    /// The starting XI's person entity ids, index-aligned with `positions`.
    /// Empty when the save stores no selection yet.
    pub starter_eids: Vec<u32>,
}

/// An in-game shortlist, with entity ids resolved to the same names the
/// player table uses, so importing one is a name-for-name copy.
#[derive(Debug, Clone, Serialize)]
pub struct GameShortlistRow {
    /// The name the user gave it in FM; `None` for the unnamed default list.
    pub name: Option<String>,
    pub players: Vec<String>,
    /// The same members as entity ids, in the same order — what the "show
    /// only this shortlist" filter keys on. Names collide (a save holds
    /// several Danny Wards); entity ids do not.
    pub player_eids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveSummary {
    /// Which attribute indices are goalkeeping ones, so the UI can group them.
    pub goalkeeping_indices: Vec<usize>,
    /// Inferred name per attribute index, empty string where unknown.
    pub attribute_names: Vec<String>,
    /// The 15 position slot names, in slot order.
    pub position_names: Vec<String>,
    pub path: String,
    pub players: Vec<PlayerRow>,
    pub clubs: Vec<ClubRow>,
    /// The save's in-game date, when it could be read. `None` means ages fall
    /// back to the system clock, which the UI says out loud.
    pub game_date: Option<String>,
    /// The human manager's shortlists as FM stores them in the save.
    pub game_shortlists: Vec<GameShortlistRow>,
    pub frames: usize,
    pub decompressed_bytes: usize,
    pub parse_millis: u64,
    /// The club the human runs, when a save records one.
    pub my_club: Option<MyClub>,
    /// The human's active tactic, when one is set.
    pub tactic: Option<TacticRow>,
}

/// The loaded save, kept in the backend so searches run here rather than
/// shipping a quarter of a million rows to the frontend and filtering them
/// in JavaScript on the UI thread.
#[derive(Default)]
pub struct LoadedSave(pub std::sync::Mutex<Option<std::sync::Arc<SaveSummary>>>);

/// A nation present in the save, for the filter dropdowns. `name` is empty
/// for identifiers the format work has not named — they still group and
/// filter, they just cannot say which flag they are.
#[derive(Debug, Clone, Serialize)]
pub struct NationOption {
    pub id: u16,
    pub name: String,
}

/// What the frontend receives on open: everything in [`SaveSummary`] except
/// the player rows, plus the derived facts the filter bar used to compute by
/// scanning them. The rows stay in [`LoadedSave`] and are queried in pages.
#[derive(Debug, Clone, Serialize)]
pub struct SaveOverview {
    pub goalkeeping_indices: Vec<usize>,
    pub attribute_names: Vec<String>,
    pub position_names: Vec<String>,
    pub path: String,
    pub clubs: Vec<ClubRow>,
    pub game_date: Option<String>,
    pub game_shortlists: Vec<GameShortlistRow>,
    pub frames: usize,
    pub decompressed_bytes: usize,
    pub parse_millis: u64,
    /// How many people the table can show — the header's census figure.
    pub people_count: usize,
    /// Whether any row carries a decoded ability, gender or trait reading —
    /// what decides if those filters are offered at all.
    pub ability_known: bool,
    pub gender_known: bool,
    pub flags_known: bool,
    /// Distinct nations, named ones alphabetical and the unnamed tail by
    /// identifier — the same ordering the frontend used to derive per load.
    pub nations: Vec<NationOption>,
    /// The club the human runs, when a save records one.
    pub my_club: Option<MyClub>,
    /// The human's active tactic, when one is set.
    pub tactic: Option<TacticRow>,
}

impl SaveOverview {
    fn of(summary: &SaveSummary) -> Self {
        let players = &summary.players;
        let mut seen: std::collections::HashMap<u16, &str> = std::collections::HashMap::new();
        for p in players {
            if let Some(id) = p.nation_id {
                seen.entry(id).or_insert(p.nation.as_str());
            }
        }
        // Unnamed identifiers still group and filter; they read as "#98" at
        // the bottom of the dropdown rather than as blank rows.
        let mut nations: Vec<NationOption> = seen
            .into_iter()
            .map(|(id, name)| NationOption {
                id,
                name: if name.is_empty() { format!("#{id}") } else { name.to_owned() },
            })
            .collect();
        nations.sort_by(|a, b| match (a.name.starts_with('#'), b.name.starts_with('#')) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (true, true) => a.id.cmp(&b.id),
            (false, false) => a.name.cmp(&b.name),
        });
        Self {
            goalkeeping_indices: summary.goalkeeping_indices.clone(),
            attribute_names: summary.attribute_names.clone(),
            position_names: summary.position_names.clone(),
            path: summary.path.clone(),
            clubs: summary.clubs.clone(),
            game_date: summary.game_date.clone(),
            game_shortlists: summary.game_shortlists.clone(),
            frames: summary.frames,
            decompressed_bytes: summary.decompressed_bytes,
            parse_millis: summary.parse_millis,
            my_club: summary.my_club.clone(),
            tactic: summary.tactic.clone(),
            people_count: players.len(),
            ability_known: players.iter().any(|p| p.ability.is_some()),
            gender_known: players.iter().any(|p| p.female.is_some()),
            flags_known: players
                .iter()
                .any(|p| !p.attributes.is_empty() || p.professionalism.is_some()),
            nations,
        }
    }
}

/// How far through parsing the backend is, emitted as it goes.
#[derive(Debug, Clone, Serialize)]
pub struct ParseProgress {
    /// 0.0 to 1.0.
    pub fraction: f32,
    /// What the backend is doing, for the user to read.
    pub label: String,
}

/// The event a parse emits as each stage begins.
pub const PARSE_PROGRESS_EVENT: &str = "parse-progress";

/// Reads and parses a save on a worker thread, returning every person in it.
///
/// Parsing a 190 MB save takes seconds, and a synchronous Tauri command runs
/// on the main thread — which freezes the window and shows the macOS spinning
/// wheel, indistinguishable from a crash. The work therefore happens inside
/// `spawn_blocking`, and each [`fm_save::Stage`] is emitted as a
/// `parse-progress` event so the UI can show real progress rather than a
/// spinner that might mean anything.
///
/// `today` is `[year, month, day]` from the frontend so ages match the user's
/// clock rather than the build machine's.
///
/// # Errors
/// Fails if the file cannot be read or is not a Football Manager save.
// Commands receive owned values deserialised from the frontend payload, so
// borrowing here is not an option.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn open_save(
    app: tauri::AppHandle,
    path: String,
    today: Vec<u16>,
) -> Result<SaveOverview, CommandError> {
    use tauri::Manager as _;

    let emitter = app.clone();
    let summary = tauri::async_runtime::spawn_blocking(move || {
        use tauri::Emitter as _;
        load_save(path, &today, |stage| {
            // A dropped progress event is not worth failing a parse over.
            let _ = emitter.emit(
                PARSE_PROGRESS_EVENT,
                ParseProgress {
                    fraction: stage.progress(),
                    label: stage.label().to_owned(),
                },
            );
        })
    })
    .await
    .map_err(|e| CommandError::Parse(format!("parsing did not finish: {e}")))??;

    let overview = SaveOverview::of(&summary);
    let state = app.state::<LoadedSave>();
    if let Ok(mut slot) = state.0.lock() {
        *slot = Some(std::sync::Arc::new(summary));
    }
    Ok(overview)
}

/// The loaded rows, or the error every query command shares when no save is
/// open — the frontend never calls these before `open_save`, so seeing this
/// message means the state was dropped, not that the user did anything wrong.
fn loaded(state: &tauri::State<'_, LoadedSave>) -> Result<std::sync::Arc<SaveSummary>, CommandError> {
    state
        .0
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .ok_or_else(|| CommandError::Parse("no save is loaded".to_owned()))
}

/// The search context a filter set needs: the named shortlist's members and
/// the save's own position slot ordering.
fn search_context(summary: &SaveSummary, filters: &crate::search::Filters) -> crate::search::Context {
    let shortlist_eids: std::collections::HashSet<u32> = match filters.shortlist.as_deref() {
        Some(name) => summary
            .game_shortlists
            .iter()
            .find(|l| l.name.as_deref().unwrap_or("") == name)
            .map(|l| l.player_eids.iter().copied().collect())
            .unwrap_or_default(),
        None => std::collections::HashSet::new(),
    };
    let position_slots: std::collections::HashMap<String, usize> = summary
        .position_names
        .iter()
        .enumerate()
        .filter(|(_, name)| !name.is_empty())
        .map(|(index, name)| (name.clone(), index))
        .collect();
    crate::search::Context { shortlist_eids, position_slots }
}

/// Filters, scores and sorts the loaded rows, returning one page and the
/// true total. The heavy lifting the frontend used to do per keystroke.
///
/// # Errors
/// Fails when no save is loaded.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn search_players(
    state: tauri::State<'_, LoadedSave>,
    filters: crate::search::Filters,
    sort_key: String,
    sort_direction: String,
    profile: Option<crate::shortlist::ScoringProfile>,
    limit: usize,
) -> Result<crate::search::SearchPage, CommandError> {
    let summary = loaded(&state)?;
    let context = search_context(&summary, &filters);
    Ok(crate::search::run(
        &summary.players,
        &filters,
        &sort_key,
        &sort_direction,
        profile.as_ref(),
        &context,
        limit,
    ))
}

/// Every matching player's entity id and name, for writing a whole result
/// set into an in-save shortlist without shipping the rows.
///
/// # Errors
/// Fails when no save is loaded.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn search_eids(
    state: tauri::State<'_, LoadedSave>,
    filters: crate::search::Filters,
    profile: Option<crate::shortlist::ScoringProfile>,
) -> Result<Vec<crate::search::SearchHit>, CommandError> {
    let summary = loaded(&state)?;
    let context = search_context(&summary, &filters);
    Ok(crate::search::matching_eids(
        &summary.players,
        &filters,
        profile.as_ref(),
        &context,
    ))
}

/// One row by exact name — how the sidebar opens a shortlist member in the
/// detail panel.
///
/// # Errors
/// Fails when no save is loaded.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn player_by_name(
    state: tauri::State<'_, LoadedSave>,
    name: String,
) -> Result<Option<PlayerRow>, CommandError> {
    let summary = loaded(&state)?;
    Ok(summary.players.iter().find(|p| p.name == name).cloned())
}

/// Everyone at one club — squad, backroom, B and youth players together; the
/// audits split them by `first_team`, `squad_level` and `is_player` on their
/// side. Tens of rows, not thousands.
///
/// Matched by entity id when the club record's head validated one, not by the
/// short name shown: two club entities can share a short name — most often a
/// men's and women's side — and a name match would pool them into one squad.
/// `eid` is `None` only for the rare club whose record head did not resolve,
/// where the name is the only link there ever was.
///
/// # Errors
/// Fails when no save is loaded.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn club_people(
    state: tauri::State<'_, LoadedSave>,
    short_name: String,
    eid: Option<u32>,
) -> Result<Vec<PlayerRow>, CommandError> {
    let summary = loaded(&state)?;
    Ok(summary
        .players
        .iter()
        .filter(|p| match eid {
            Some(eid) => p.club_eid == Some(eid),
            None => p.club == short_name,
        })
        .cloned()
        .collect())
}

/// Edits one in-game shortlist inside the save file itself.
///
/// The write policy comes from `LEGAL_NOTES.md` (amendment of 2 August 2026):
/// the user's own saves only, and never without a backup — the first write to
/// a save puts the untouched original at `<path>.gilet.bak`, and that file is
/// never overwritten afterwards.
///
/// `list` is the shortlist's FM name, or `None` for the unnamed default list.
/// `date` is `[year, month, day]` — the save's own current date, which the
/// frontend already holds; it becomes the new entry's date-added field.
///
/// # Errors
/// Fails when the file cannot be read or written, the save cannot be safely
/// rewritten (`fm_save::archive` validates before touching anything), or no
/// shortlist has that name.
// Commands receive owned values deserialised from the frontend payload.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn edit_game_shortlist(
    path: String,
    list: Option<String>,
    eid: u32,
    add: bool,
    date: Vec<u16>,
) -> Result<usize, CommandError> {
    if add {
        add_players_to_game_shortlist(path, list, vec![eid], date)
    } else {
        write_shortlist_edit(&path, |scout| {
            fm_save::shortlist::remove_entry(scout, list.as_deref(), eid).ok_or_else(|| {
                missing_list(list.as_deref())
            })
        })
        .map(usize::from)
    }
}

/// Adds many players to one in-save shortlist in a single rewrite — the
/// "filter, then send the results into the game" flow. Per-player rewrites
/// would copy the whole save once per name; this copies it once.
///
/// Returns how many players were actually added; ones already on the list
/// count zero and cost nothing.
///
/// # Errors
/// As [`edit_game_shortlist`].
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn add_players_to_game_shortlist(
    path: String,
    list: Option<String>,
    eids: Vec<u32>,
    date: Vec<u16>,
) -> Result<usize, CommandError> {
    let when = fm_save::Date {
        year: date.first().copied().unwrap_or(2026),
        month: date.get(1).copied().unwrap_or(1) as u8,
        day: date.get(2).copied().unwrap_or(1) as u8,
    };
    let stamp = fm_save::shortlist::date_added_bytes(when);

    let mut added = 0usize;
    write_shortlist_edit(&path, |scout| {
        let mut frame = scout.to_vec();
        for &eid in &eids {
            let next = fm_save::shortlist::add_entry(&frame, list.as_deref(), eid, stamp)
                .ok_or_else(|| missing_list(list.as_deref()))?;
            if next.len() != frame.len() {
                added += 1;
            }
            frame = next;
        }
        Ok(frame)
    })?;
    Ok(added)
}

/// Empties one in-save shortlist, leaving the list itself in place. Returns
/// whether anything was removed.
///
/// # Errors
/// As [`edit_game_shortlist`].
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn clear_game_shortlist(path: String, list: Option<String>) -> Result<bool, CommandError> {
    write_shortlist_edit(&path, |scout| {
        fm_save::shortlist::clear_list(scout, list.as_deref())
            .ok_or_else(|| missing_list(list.as_deref()))
    })
}

fn missing_list(list: Option<&str>) -> CommandError {
    CommandError::Parse(format!("no shortlist named {}", list.unwrap_or("(unnamed)")))
}

/// The shared write path: read the save, let `edit` produce a new
/// `scout_man.dat`, and rewrite the file around it — after parking the
/// untouched original at `<path>.gilet.bak` the first time. Returns whether
/// anything changed; an edit that produces identical bytes touches nothing.
fn write_shortlist_edit(
    path: &str,
    edit: impl FnOnce(&[u8]) -> Result<Vec<u8>, CommandError>,
) -> Result<bool, CommandError> {
    let bytes = std::fs::read(path).map_err(|e| CommandError::Read {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    let scout = fm_save::archive::member_plaintext(&bytes, "scout_man.dat")
        .map_err(|e| CommandError::Parse(e.to_string()))?;

    let edited = edit(&scout)?;
    if edited == scout {
        return Ok(false);
    }

    let written = fm_save::archive::replace_member(&bytes, "scout_man.dat", &edited)
        .map_err(|e| CommandError::Parse(e.to_string()))?;

    // The backup preserves the save as it was before Gilet ever wrote to it,
    // so it is created once and never replaced.
    let backup = format!("{path}.gilet.bak");
    if !std::path::Path::new(&backup).exists() {
        std::fs::write(&backup, &bytes).map_err(|e| CommandError::Write {
            path: backup.clone(),
            message: e.to_string(),
        })?;
    }
    std::fs::write(path, written).map_err(|e| CommandError::Write {
        path: path.to_owned(),
        message: e.to_string(),
    })?;
    Ok(true)
}

/// Resolves the save's in-game shortlists to the same names the player table
/// uses. An eid that resolves to nobody is dropped rather than guessed at; a
/// list that ends up nameless and empty is noise, not data.
fn resolve_game_shortlists(save: &fm_save::Save) -> Vec<GameShortlistRow> {
    let people_by_eid: std::collections::HashMap<u32, &str> = save
        .people
        .iter()
        .filter_map(|p| Some((p.eid?, p.display_name())))
        .collect();
    save.shortlists
        .iter()
        .map(|s| {
            let resolved: Vec<(u32, String)> = s
                .person_eids
                .iter()
                .filter_map(|&eid| people_by_eid.get(&eid).map(|&n| (eid, n.to_owned())))
                .collect();
            GameShortlistRow {
                name: s.name.clone(),
                player_eids: resolved.iter().map(|(eid, _)| *eid).collect(),
                players: resolved.into_iter().map(|(_, name)| name).collect(),
            }
        })
        .filter(|s| s.name.is_some() || !s.players.is_empty())
        .collect()
}

/// One table row for a parsed person, aged against the save's own date.
fn person_row(
    p: &fm_save::Person,
    now: fm_save::Date,
    club_names: &std::collections::HashMap<u32, &str>,
    first_team: &std::collections::HashSet<u32>,
) -> PlayerRow {
    PlayerRow {
        first_team: p.eid.is_some_and(|e| first_team.contains(&e)),
        id: p.offset,
        eid: p.eid,
        reputation: p.reputation.map(ReputationRow::from),
        name: p.display_name().to_owned(),
        born: p
            .date_of_birth
            .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
            .unwrap_or_default(),
        age: p.date_of_birth.map(|d| d.age_on(now)),
        ability: p.ability.as_ref().map(|a| a.current),
        potential: p.ability.as_ref().map(|a| a.potential),
        is_player: p.is_player(),
        attributes: p.ability.as_ref().map(|a| a.attributes.to_vec()).unwrap_or_default(),
        nation_id: p.nation_id,
        nation: p.nation().unwrap_or_default().to_owned(),
        positions: p
            .ability
            .as_ref()
            .map(|a| a.natural_positions().iter().map(|s| (*s).to_owned()).collect())
            .unwrap_or_default(),
        position_ratings: p.ability.as_ref().map(|a| a.positions.to_vec()).unwrap_or_default(),
        club: p
            .club_eid
            .and_then(|eid| club_names.get(&eid).copied())
            .unwrap_or_default()
            .to_owned(),
        club_eid: p.club_eid,
        squad_level: p.squad_level.map(|k| k.name().to_owned()),
        staff: p.staff.as_ref().map(|s| StaffSheet {
            attributes: s.attributes.to_vec(),
            home_reputation: s.home_reputation,
            current_reputation: s.current_reputation,
            world_reputation: s.world_reputation,
            current_ability: s.current_ability,
            potential_ability: s.potential_ability,
        }),
        female: p.female,
        wage: p.wage,
        contract_until: p
            .contract_until
            .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
            .unwrap_or_default(),
        adaptability: p.adaptability(),
        ambition: p.ambition(),
        loyalty: p.loyalty(),
        pressure: p.pressure(),
        professionalism: p.professionalism(),
        sportsmanship: p.sportsmanship(),
        temperament: p.temperament(),
        controversy: p.controversy(),
        stub: false,
        staff_role: p.staff_role.map(|r| r.name().to_owned()),
        in_national_squad: p.in_national_squad,
    }
}

/// Rows for stub members: squad entries whose entity id has no person
/// record — generated non-contract signings. They appear as undecoded rows
/// under their club rather than silently missing from its squad.
///
/// A stub is just as often a B, youth or out-of-league filler as a first-team
/// one — a squad with no budget for real signings leans on them harder, if
/// anything — so both squad tables are read, first team taking priority on
/// the rare eid a stub-shaped id turns up in more than one list.
fn stub_rows(
    save: &fm_save::Save,
    club_names: &std::collections::HashMap<u32, &str>,
) -> Vec<PlayerRow> {
    let mut club_of: std::collections::HashMap<u32, (u32, fm_save::squad::SquadKind)> =
        std::collections::HashMap::new();
    for s in &save.squads {
        for &eid in &s.player_eids {
            club_of.entry(eid).or_insert((s.club_eid, fm_save::squad::SquadKind::FirstTeam));
        }
    }
    for s in &save.team_squads {
        for &eid in &s.player_eids {
            club_of.entry(eid).or_insert((s.club_eid, s.kind));
        }
    }
    save.stubs
        .iter()
        .filter_map(|s| {
            let &(club_eid, kind) = club_of.get(&s.eid)?;
            Some(PlayerRow {
                id: s.offset,
                eid: Some(s.eid),
                name: String::new(),
                born: String::new(),
                age: None,
                ability: None,
                potential: None,
                is_player: true,
                attributes: Vec::new(),
                nation_id: None,
                nation: String::new(),
                positions: Vec::new(),
                position_ratings: Vec::new(),
                club: club_names.get(&club_eid).copied().unwrap_or_default().to_owned(),
                club_eid: Some(club_eid),
                squad_level: Some(kind.name().to_owned()),
                female: None,
                wage: None,
                contract_until: String::new(),
                adaptability: None,
                ambition: None,
                loyalty: None,
                pressure: None,
                professionalism: None,
                sportsmanship: None,
                temperament: None,
                controversy: None,
                // A stub is a presence, not a person: no sheet to read.
                staff: None,
                reputation: None,
                stub: true,
                staff_role: None,
                in_national_squad: false,
                first_team: kind == fm_save::squad::SquadKind::FirstTeam,
            })
        })
        .collect()
}

/// What one club's squad adds up to, gathered in a single pass over the
/// people. Every total carries its own count, because a field that only some
/// of the squad have decoded — an age, a wage — must average over the ones it
/// has rather than over the whole squad.
#[derive(Debug, Clone, Copy, Default)]
struct SquadTotals {
    ability: u32,
    potential: u32,
    /// Players counted: everyone at this club with a decoded ability block.
    players: usize,
    age_years: u32,
    aged: usize,
    wages: u64,
    waged: usize,
}

impl SquadTotals {
    /// The mean of a total over its own count, rounded. `None` when nothing
    /// was counted — an empty average is not zero.
    fn mean(sum: u32, n: usize) -> Option<u16> {
        (n > 0).then(|| u16::try_from(sum as usize / n).unwrap_or(u16::MAX))
    }

    /// Mean age to one decimal place. Ages are small integers and counts are
    /// squad-sized, both exact in an f32, so the cast loses nothing.
    #[allow(clippy::cast_precision_loss)]
    fn average_age(self) -> Option<f32> {
        (self.aged > 0).then(|| {
            let mean = self.age_years as f32 / self.aged as f32;
            (mean * 10.0).round() / 10.0
        })
    }
}

/// Totals every club's squad: ability, age and the wage bill.
///
/// Squad strength is the closest thing to a league level while competitions
/// are undecoded: a club is as strong as the players it fields. Staff carry no
/// ability block and so never count towards a squad — their wages are not
/// decoded at all (`OPEN_PROBLEMS.md` §3c), which is the other reason a wage
/// bill here is a playing bill. Only first-team squad members count:
/// `club_eid` alone would fold the B and youth lists in (they bind it too,
/// since the team-squad pass), and a first-team age profile diluted by
/// sixteen-year-old intakes reads younger than the team that plays.
fn squad_strength(
    save: &fm_save::Save,
    now: fm_save::Date,
) -> std::collections::HashMap<u32, SquadTotals> {
    let first_team: std::collections::HashSet<u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().copied())
        .collect();
    let mut strength: std::collections::HashMap<u32, SquadTotals> =
        std::collections::HashMap::new();
    for p in &save.people {
        let (Some(eid), Some(ability)) = (p.club_eid, p.ability.as_ref()) else {
            continue;
        };
        if !p.eid.is_some_and(|e| first_team.contains(&e)) {
            continue;
        }
        let entry = strength.entry(eid).or_default();
        entry.ability += u32::from(ability.current);
        entry.potential += u32::from(ability.potential);
        entry.players += 1;
        if let Some(born) = p.date_of_birth {
            entry.age_years += u32::from(born.age_on(now));
            entry.aged += 1;
        }
        if let Some(wage) = p.wage {
            entry.wages += u64::from(wage);
            entry.waged += 1;
        }
    }
    strength
}

/// The body of [`open_save`], without the Tauri handle, so the whole
/// open → decode → summarise path is testable outside an app.
///
/// # Errors
/// Fails if the file cannot be read or is not a Football Manager save.
pub fn load_save(
    path: String,
    today: &[u16],
    on_stage: impl FnMut(fm_save::Stage),
) -> Result<SaveSummary, CommandError> {
    let bytes = std::fs::read(&path).map_err(|e| CommandError::Read {
        path: path.clone(),
        message: e.to_string(),
    })?;

    let started = std::time::Instant::now();
    let save = fm_save::Save::parse_with_progress(&bytes, on_stage)
        .map_err(|e| CommandError::Parse(e.to_string()))?;
    let parse_millis = started.elapsed().as_millis() as u64;

    // Ages are relative to the save's own date. Using the system clock instead
    // reports everyone a year too old on a save left alone for a season.
    let system_today = fm_save::Date {
        year: today.first().copied().unwrap_or(2026),
        month: today.get(1).copied().unwrap_or(1) as u8,
        day: today.get(2).copied().unwrap_or(1) as u8,
    };
    let now = save.game_date.unwrap_or(system_today);
    let dated_from_save = save.game_date.is_some();

    // Club short names by entity id, to label each person with their club.
    let club_names: std::collections::HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();

    // Compact people — the name-and-identity entries aged saves fold
    // departed people down to (SAVE_FORMAT.md §6d-ter) — are parsed so the
    // census is honest, but not shown: a row with no age, club, attributes
    // or contract answers no scouting question and clogs the table.
    // Owner's call, 4 August 2026.
    let first_team: std::collections::HashSet<u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().copied())
        .collect();
    let mut players: Vec<PlayerRow> = save
        .people
        .iter()
        .filter(|p| !p.compact)
        .map(|p| person_row(p, now, &club_names, &first_team))
        .collect();
    players.extend(stub_rows(&save, &club_names));

    let strength = squad_strength(&save, now);
    let clubs = save
        .clubs
        .iter()
        .map(|c| {
            let totals = c.eid.and_then(|eid| strength.get(&eid)).copied();
            ClubRow {
                id: c.offset,
                eid: c.eid,
                name: c.name.clone(),
                short_name: c.short_name.clone(),
                club_id: c.club_id,
                nation_id: c.nation_id,
                squad_size: totals.map_or(0, |t| t.players),
                average_ability: totals.and_then(|t| SquadTotals::mean(t.ability, t.players)),
                average_potential: totals.and_then(|t| SquadTotals::mean(t.potential, t.players)),
                average_age: totals.and_then(SquadTotals::average_age),
                // A wage bill of nothing is not a wage bill of zero: a club
                // whose wages all failed to decode reports none at all.
                wage_bill: totals.filter(|t| t.waged > 0).map(|t| t.wages),
                wages_known: totals.map_or(0, |t| t.waged),
                reputation: c.reputation,
            }
        })
        .collect();

    Ok(SaveSummary {
        goalkeeping_indices: fm_save::ability::GOALKEEPING_INDICES.to_vec(),
        position_names: fm_save::ability::POSITION_NAMES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        attribute_names: (0..fm_save::ability::ATTRIBUTE_COUNT)
            .map(|i| fm_save::ability::attribute_name(i).unwrap_or_default().to_owned())
            .collect(),
        path,
        players,
        clubs,
        game_date: dated_from_save.then(|| format!("{:04}-{:02}-{:02}", now.year, now.month, now.day)),
        game_shortlists: resolve_game_shortlists(&save),
        frames: save.frame_sizes.len(),
        decompressed_bytes: save.frame_sizes.iter().sum(),
        parse_millis,
        my_club: resolve_my_club(&save),
        tactic: resolve_tactic(&save),
    })
}

/// Resolves the human's club from `Save::human_eid` through their record.
/// `None` when the save has no human, the human's record did not resolve, or
/// they are unemployed — a missing club here is not a parser gap to paper
/// over, it is the honest "between jobs" state.
fn resolve_my_club(save: &fm_save::Save) -> Option<MyClub> {
    let human_eid = save.human_eid?;
    let manager = save.people.iter().find(|p| p.eid == Some(human_eid))?;
    let club_eid = manager.club_eid?;
    let club = save.clubs.iter().find(|c| c.eid == Some(club_eid))?;
    Some(MyClub {
        eid: club_eid,
        name: club.name.clone(),
        short_name: club.short_name.clone(),
        manager_eid: human_eid,
        manager_name: manager.display_name().to_owned(),
    })
}

/// Maps the parsed tactic to its IPC shape.
fn resolve_tactic(save: &fm_save::Save) -> Option<TacticRow> {
    let t = save.tactic.as_ref()?;
    Some(TacticRow {
        name: t.name.clone(),
        style: t.style.clone(),
        positions: t.positions.clone(),
        starter_eids: t.starters.clone(),
    })
}

/// Reads player names out of a CSV so a shortlist can be brought in from a
/// spreadsheet or another tool.
///
/// Takes the `name` column when there is a header, otherwise the first column.
/// Names that are not in the loaded save are returned separately rather than
/// dropped, so the user finds out rather than wondering where they went.
///
/// # Errors
/// Fails if the file cannot be read.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn import_csv(path: String, known: Vec<String>) -> Result<ImportResult, CommandError> {
    let text = std::fs::read_to_string(&path).map_err(|e| CommandError::Read {
        path: path.clone(),
        message: e.to_string(),
    })?;

    let known: std::collections::HashSet<&str> = known.iter().map(String::as_str).collect();
    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let Some(first) = split_csv_row(line).into_iter().next() else {
            continue;
        };
        let name = first.trim();
        if name.is_empty() {
            continue;
        }
        // Skip a header row rather than importing a player called "name".
        if index == 0 && name.eq_ignore_ascii_case("name") {
            continue;
        }
        if known.contains(name) {
            if !matched.iter().any(|m| m == name) {
                matched.push(name.to_owned());
            }
        } else {
            unmatched.push(name.to_owned());
        }
    }

    Ok(ImportResult { matched, unmatched })
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub matched: Vec<String>,
    pub unmatched: Vec<String>,
}

/// Splits one CSV row, honouring double quotes so a quoted name containing a
/// comma stays intact — the same quoting `export_csv` writes.
fn split_csv_row(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

/// Writes rows to a CSV file.
///
/// # Errors
/// Fails if the destination cannot be written.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn export_csv(path: String, rows: Vec<PlayerRow>) -> Result<(), CommandError> {
    use std::fmt::Write as _;

    let mut out = String::from("name,born,age,ability,potential,club,squad_level,wage,contract_until\n");
    for r in &rows {
        // Writing into a String cannot fail, so the result is discarded.
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{},{}",
            csv_field(&r.name),
            r.born,
            r.age.map_or(String::new(), |v| v.to_string()),
            r.ability.map_or(String::new(), |v| v.to_string()),
            r.potential.map_or(String::new(), |v| v.to_string()),
            csv_field(&r.club),
            csv_field(r.squad_level.as_deref().unwrap_or_default()),
            r.wage.map_or(String::new(), |v| v.to_string()),
            r.contract_until,
        );
    }
    std::fs::write(&path, out).map_err(|e| CommandError::Write {
        path,
        message: e.to_string(),
    })
}

/// Quotes a CSV field. Player names contain commas and apostrophes, so this is
/// not optional.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{csv_field, split_csv_row};

    #[test]
    fn quotes_fields_containing_commas() {
        assert_eq!(csv_field("Haaland"), "Haaland");
        assert_eq!(csv_field("de Boer, Franciscus"), "\"de Boer, Franciscus\"");
    }

    #[test]
    fn escapes_embedded_quotes() {
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn leaves_accented_names_alone() {
        assert_eq!(csv_field("Kylian Mbappé Lottin"), "Kylian Mbappé Lottin");
    }

    #[test]
    fn splits_a_plain_row() {
        assert_eq!(split_csv_row("a,b,c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn keeps_a_quoted_comma_together() {
        // Round-trips what csv_field writes for a name containing a comma.
        assert_eq!(split_csv_row("\"de Boer, Franciscus\",1970"), vec!["de Boer, Franciscus", "1970"]);
    }

    #[test]
    fn unescapes_doubled_quotes() {
        assert_eq!(split_csv_row("\"a\"\"b\",x"), vec!["a\"b", "x"]);
    }
}
