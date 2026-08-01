//! Commands exposed to the frontend.
//!
//! These live in their own module because `tauri::generate_handler!` imports
//! each command into the module that invokes it, which collides if the handler
//! list sits alongside the definitions.

use serde::{Deserialize, Serialize};

use crate::CommandError;

/// A player as the table renders them.
///
/// `ability` and `potential` are `Option` because Current and Potential Ability
/// are not yet located in the save format — see `docs/SAVE_FORMAT.md`. They
/// serialise as `null` so the UI can show an undecoded state rather than a
/// fabricated number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRow {
    /// Byte offset of the record, stable within one save and used as the row key.
    pub id: usize,
    pub name: String,
    pub born: String,
    pub age: u16,
    pub ability: Option<u8>,
    pub potential: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveSummary {
    pub path: String,
    pub players: Vec<PlayerRow>,
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

    let now = fm_save::Date {
        year: today.first().copied().unwrap_or(2026),
        month: today.get(1).copied().unwrap_or(1) as u8,
        day: today.get(2).copied().unwrap_or(1) as u8,
    };

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
                ability: None,
                potential: None,
            }
        })
        .collect();

    Ok(SaveSummary {
        path,
        players,
        frames: save.frame_sizes.len(),
        decompressed_bytes: save.frame_sizes.iter().sum(),
        parse_millis,
    })
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
    use super::csv_field;

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
}
