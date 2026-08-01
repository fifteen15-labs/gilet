//! Tauri shell over the `fm-save` parser.
//!
//! Everything here is I/O and serialisation. Format knowledge belongs in
//! `fm-save`, which is pure and testable on its own.

pub mod commands;
pub mod shortlist;

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("could not read {path}: {message}")]
    Read { path: String, message: String },
    #[error("could not parse save: {0}")]
    Parse(String),
    #[error("could not write {path}: {message}")]
    Write { path: String, message: String },
}

// Tauri requires command errors to be serialisable; the message is what the UI
// shows, so flatten to a string rather than leaking the variant shape.
impl Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::default_locations,
            commands::open_save,
            commands::export_csv,
            commands::import_csv,
            shortlist::load_shortlists,
            shortlist::save_shortlists,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("gilet failed to start: {e}");
        std::process::exit(1);
    }
}
