//! Shortlist persistence.
//!
//! Shortlists are the user's own work, so they live on disk as readable JSON in
//! Football Manager's own shortlists folder rather than inside a database only
//! this app can open.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager as _;

use crate::CommandError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortlist {
    pub name: String,
    /// Player names rather than record offsets: offsets are only stable within
    /// a single save file, and a shortlist should survive the next rollover.
    pub players: Vec<String>,
}

/// A named filter configuration, so a recurring search survives the session.
///
/// The filter body is opaque JSON: its shape belongs to the frontend, and
/// mirroring it here would mean changing two files for every new filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedFilter {
    pub name: String,
    pub filters: serde_json::Value,
}

/// Which attribute sheet a profile weights. Profiles saved before staff
/// scoring existed carry no kind and default to player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProfileKind {
    #[default]
    Player,
    Staff,
}

/// A user-defined weighting over attribute indices, used to rank players by
/// what the user is actually looking for.
///
/// The weights are the user's own. Gilet ships none: FM's role ratings are
/// computed from weights SI does not publish, and inventing a table of them
/// would be inventing numbers. Keyed by attribute index as a string, because
/// that is what JSON object keys are. A `staff` profile's keys index the
/// 52-item non-player sheet instead of the player block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringProfile {
    pub name: String,
    pub weights: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub kind: ProfileKind,
}

/// Gilet's files live in Football Manager's own `shortlists` folder, next to
/// the `.slf` files FM writes there, because that is where someone already
/// looks for a shortlist. Ours stay readable JSON rather than FM's proprietary
/// binary, so FM will not open them — the filename keeps the two apart.
///
/// Falls back to the app data directory Tauri resolves from the app identifier
/// when FM is not installed, which is the only case where there is no better
/// answer.
fn store_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    crate::commands::fm_dir(app)
        .map(|d| d.join("shortlists"))
        .or_else(|| app.path().app_data_dir().ok())
}

/// The pre-FM-folder location, kept only to migrate away from.
fn legacy_path(app: &tauri::AppHandle, file: &str) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join(file))
}

/// Copies a file left in the old app data directory across to FM's shortlists
/// folder the first time it is read, so moving the store does not read as data
/// loss. The original is copied rather than moved: an upgrade should never be
/// the thing that deletes someone's shortlists.
fn migrate(app: &tauri::AppHandle, file: &str) {
    let Some(current) = store_dir(app).map(|d| d.join(file)) else {
        return;
    };
    if current.exists() {
        return;
    }
    let Some(legacy) = legacy_path(app, file) else {
        return;
    };
    if !legacy.exists() || legacy == current {
        return;
    }
    let Some(dir) = current.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    // A failed migration is not fatal — the read below simply finds nothing and
    // the user starts fresh, with the legacy file still on disk.
    let _ = std::fs::copy(&legacy, &current);
}

fn store_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    migrate(app, "shortlists.json");
    store_dir(app).map(|d| d.join("shortlists.json"))
}

fn filters_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    migrate(app, "filters.json");
    store_dir(app).map(|d| d.join("filters.json"))
}

fn profiles_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    migrate(app, "profiles.json");
    store_dir(app).map(|d| d.join("profiles.json"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: Option<PathBuf>) -> Result<Vec<T>, CommandError> {
    let Some(path) = path else { return Ok(Vec::new()) };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| CommandError::Read {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    serde_json::from_str(&text).map_err(|e| CommandError::Parse(e.to_string()))
}

fn write_json<T: Serialize>(path: Option<PathBuf>, value: &[T]) -> Result<(), CommandError> {
    let Some(path) = path else { return Ok(()) };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| CommandError::Write {
            path: dir.display().to_string(),
            message: e.to_string(),
        })?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| CommandError::Parse(e.to_string()))?;
    std::fs::write(&path, text).map_err(|e| CommandError::Write {
        path: path.display().to_string(),
        message: e.to_string(),
    })
}

/// Reads saved filter presets, empty when none exist yet.
///
/// # Errors
/// Fails only if the file exists but cannot be read or parsed.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn load_filters(app: tauri::AppHandle) -> Result<Vec<SavedFilter>, CommandError> {
    read_json(filters_path(&app))
}

/// Writes filter presets to disk.
///
/// # Errors
/// Fails if the directory or file cannot be written.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn save_filters(app: tauri::AppHandle, filters: Vec<SavedFilter>) -> Result<(), CommandError> {
    write_json(filters_path(&app), &filters)
}

/// Reads saved scoring profiles, empty when none exist yet.
///
/// # Errors
/// Fails only if the file exists but cannot be read or parsed.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn load_profiles(app: tauri::AppHandle) -> Result<Vec<ScoringProfile>, CommandError> {
    read_json(profiles_path(&app))
}

/// Writes scoring profiles to disk.
///
/// # Errors
/// Fails if the directory or file cannot be written.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn save_profiles(
    app: tauri::AppHandle,
    profiles: Vec<ScoringProfile>,
) -> Result<(), CommandError> {
    write_json(profiles_path(&app), &profiles)
}

/// Reads saved shortlists, returning an empty list when none exist yet.
///
/// # Errors
/// Fails only if the file exists but cannot be read or parsed.
// Tauri injects the handle by value; the signature is fixed by the macro.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn load_shortlists(app: tauri::AppHandle) -> Result<Vec<Shortlist>, CommandError> {
    let Some(path) = store_path(&app) else {
        return Ok(Vec::new());
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| CommandError::Read {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    serde_json::from_str(&text).map_err(|e| CommandError::Parse(e.to_string()))
}

/// Writes shortlists to disk, creating the directory on first save.
///
/// # Errors
/// Fails if the directory or file cannot be written.
// Commands receive owned values deserialised from the frontend payload.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn save_shortlists(app: tauri::AppHandle, lists: Vec<Shortlist>) -> Result<(), CommandError> {
    let Some(path) = store_path(&app) else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| CommandError::Write {
            path: dir.display().to_string(),
            message: e.to_string(),
        })?;
    }
    let text = serde_json::to_string_pretty(&lists).map_err(|e| CommandError::Parse(e.to_string()))?;
    std::fs::write(&path, text).map_err(|e| CommandError::Write {
        path: path.display().to_string(),
        message: e.to_string(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_saved_before_kinds_existed_loads_as_a_player_profile() {
        let old = r#"{"name":"Wingers","weights":{"34":5}}"#;
        let profile: ScoringProfile = serde_json::from_str(old).unwrap();
        assert_eq!(profile.kind, ProfileKind::Player);
    }

    #[test]
    fn a_staff_profile_round_trips() {
        let profile = ScoringProfile {
            name: "Scouts".to_owned(),
            weights: std::collections::BTreeMap::from([("28".to_owned(), 5.0)]),
            kind: ProfileKind::Staff,
        };
        let text = serde_json::to_string(&profile).unwrap();
        assert!(text.contains(r#""kind":"staff""#));
        let back: ScoringProfile = serde_json::from_str(&text).unwrap();
        assert_eq!(back.kind, ProfileKind::Staff);
    }
}
