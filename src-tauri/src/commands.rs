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
    /// Short name of the club whose first-team squad lists this person, empty
    /// when unattached — free agents, national staff, unresolved records.
    pub club: String,
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
    /// True for a stub — a non-contract squad filler the save stores without
    /// a person record. Identity and club are known; name, age and attributes
    /// are not decoded, and the row says so rather than vanishing.
    pub stub: bool,
    /// The person's decoded place in a club's backroom — "Manager" from the
    /// roster table's seat, or "Coaching" / "Medical" / "Recruitment" from
    /// the department staff lists. `None` for players, the unemployed, and
    /// staff bound outside the department triple.
    pub staff_role: Option<String>,
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
) -> Result<SaveSummary, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Emitter as _;
        load_save(path, &today, |stage| {
            // A dropped progress event is not worth failing a parse over.
            let _ = app.emit(
                PARSE_PROGRESS_EVENT,
                ParseProgress {
                    fraction: stage.progress(),
                    label: stage.label().to_owned(),
                },
            );
        })
    })
    .await
    .map_err(|e| CommandError::Parse(format!("parsing did not finish: {e}")))?
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
        .filter_map(|p| Some((p.eid?, p.full_name.as_str())))
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
) -> PlayerRow {
    PlayerRow {
        id: p.offset,
        eid: p.eid,
        name: p.full_name.clone(),
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
    }
}

/// Rows for stub members: squad entries whose entity id has no person
/// record — generated non-contract signings. They appear as undecoded rows
/// under their club rather than silently missing from its squad.
fn stub_rows(
    save: &fm_save::Save,
    club_names: &std::collections::HashMap<u32, &str>,
) -> Vec<PlayerRow> {
    let club_by_member: std::collections::HashMap<u32, u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().map(|&e| (e, s.club_eid)))
        .collect();
    save.stubs
        .iter()
        .filter_map(|s| {
            let club_eid = club_by_member.get(&s.eid)?;
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
                club: club_names.get(club_eid).copied().unwrap_or_default().to_owned(),
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
                stub: true,
                staff_role: None,
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
/// bill here is a playing bill.
fn squad_strength(
    save: &fm_save::Save,
    now: fm_save::Date,
) -> std::collections::HashMap<u32, SquadTotals> {
    let mut strength: std::collections::HashMap<u32, SquadTotals> =
        std::collections::HashMap::new();
    for p in &save.people {
        let (Some(eid), Some(ability)) = (p.club_eid, p.ability.as_ref()) else {
            continue;
        };
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
    let mut players: Vec<PlayerRow> = save
        .people
        .iter()
        .filter(|p| !p.compact)
        .map(|p| person_row(p, now, &club_names))
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

    let mut out = String::from("name,born,age,ability,potential,club,wage,contract_until\n");
    for r in &rows {
        // Writing into a String cannot fail, so the result is discarded.
        let _ = writeln!(
            out,
            "{},{},{},{},{},{},{},{}",
            csv_field(&r.name),
            r.born,
            r.age.map_or(String::new(), |v| v.to_string()),
            r.ability.map_or(String::new(), |v| v.to_string()),
            r.potential.map_or(String::new(), |v| v.to_string()),
            csv_field(&r.club),
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
