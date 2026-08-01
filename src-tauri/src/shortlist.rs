//! Shortlist persistence.
//!
//! Shortlists are the user's own work, so they live on disk as readable JSON in
//! the app data directory rather than inside a database only this app can open.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::CommandError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortlist {
    pub name: String,
    /// Player names rather than record offsets: offsets are only stable within
    /// a single save file, and a shortlist should survive the next rollover.
    pub players: Vec<String>,
}

fn store_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/anorak/shortlists.json"))
}

/// Reads saved shortlists, returning an empty list when none exist yet.
///
/// # Errors
/// Fails only if the file exists but cannot be read or parsed.
#[tauri::command]
pub fn load_shortlists() -> Result<Vec<Shortlist>, CommandError> {
    let Some(path) = store_path() else {
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
pub fn save_shortlists(lists: Vec<Shortlist>) -> Result<(), CommandError> {
    let Some(path) = store_path() else {
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
