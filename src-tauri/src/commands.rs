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

/// FM keeps saves under Application Support on macOS and in Documents on
/// Windows. Only the directory that actually exists is offered, so the dialog
/// falls back to the system default rather than opening on a missing path.
fn saves_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let path = app.path();
    let candidates = if cfg!(target_os = "macos") {
        vec![path.data_dir().ok()?.join("Sports Interactive")]
    } else {
        vec![path.document_dir().ok()?.join("Sports Interactive")]
    };

    for base in candidates {
        if !base.is_dir() {
            continue;
        }
        // Prefer the newest Football Manager folder, so a machine with several
        // installed opens on the current one.
        let mut versions: Vec<_> = std::fs::read_dir(&base)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("games").is_dir())
            .collect();
        versions.sort();
        if let Some(newest) = versions.pop() {
            return Some(newest.join("games"));
        }
        return Some(base);
    }
    None
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
    pub name: String,
    pub born: String,
    pub age: u16,
    pub ability: Option<u8>,
    pub potential: Option<u8>,
    /// True when this person has ability data, i.e. is a player not staff.
    pub is_player: bool,
    /// The 54 attributes on FM's 1-20 scale. Empty for staff.
    pub attributes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubRow {
    pub id: usize,
    pub name: String,
    pub short_name: String,
    pub club_id: u32,
    pub nation_id: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveSummary {
    /// Which attribute indices are goalkeeping ones, so the UI can group them.
    pub goalkeeping_indices: Vec<usize>,
    pub path: String,
    pub players: Vec<PlayerRow>,
    pub clubs: Vec<ClubRow>,
    /// The save's in-game date, when it could be read. `None` means ages fall
    /// back to the system clock, which the UI says out loud.
    pub game_date: Option<String>,
    pub frames: usize,
    pub decompressed_bytes: usize,
    pub parse_millis: u64,
}

/// Reads and parses a save, returning every person in it.
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
pub fn open_save(path: String, today: Vec<u16>) -> Result<SaveSummary, CommandError> {
    let bytes = std::fs::read(&path).map_err(|e| CommandError::Read {
        path: path.clone(),
        message: e.to_string(),
    })?;

    let started = std::time::Instant::now();
    let save = fm_save::Save::parse(&bytes).map_err(|e| CommandError::Parse(e.to_string()))?;
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

    let players = save
        .people
        .iter()
        .map(|p| {
            let d = p.date_of_birth;
            PlayerRow {
                id: p.offset,
                name: p.full_name.clone(),
                born: format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
                age: d.age_on(now),
                ability: p.ability.as_ref().map(|a| a.current),
                potential: p.ability.as_ref().map(|a| a.potential),
                is_player: p.is_player(),
                attributes: p.ability.as_ref().map(|a| a.attributes.to_vec()).unwrap_or_default(),
            }
        })
        .collect();

    let clubs = save
        .clubs
        .iter()
        .map(|c| ClubRow {
            id: c.offset,
            name: c.name.clone(),
            short_name: c.short_name.clone(),
            club_id: c.club_id,
            nation_id: c.nation_id,
        })
        .collect();

    Ok(SaveSummary {
        goalkeeping_indices: fm_save::ability::GOALKEEPING_INDICES.to_vec(),
        path,
        players,
        clubs,
        game_date: dated_from_save.then(|| format!("{:04}-{:02}-{:02}", now.year, now.month, now.day)),
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

    let mut out = String::from("name,born,age,ability,potential\n");
    for r in &rows {
        // Writing into a String cannot fail, so the result is discarded.
        let _ = writeln!(
            out,
            "{},{},{},{},{}",
            csv_field(&r.name),
            r.born,
            r.age,
            r.ability.map_or(String::new(), |v| v.to_string()),
            r.potential.map_or(String::new(), |v| v.to_string()),
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
