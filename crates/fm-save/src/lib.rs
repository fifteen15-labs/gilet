//! Reader for Football Manager 26 save files.
//!
//! The format is undocumented; `docs/SAVE_FORMAT.md` records what has been
//! confirmed against real saves and what is still unknown. This crate is pure —
//! it takes bytes and returns data, with no Tauri or filesystem dependency — so
//! the parsing can be tested directly.
//!
//! ```no_run
//! let bytes = std::fs::read("Career.fm")?;
//! let save = fm_save::Save::parse(&bytes)?;
//! println!("{} people", save.people.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod container;
pub mod date;
pub mod error;
pub mod person;

pub use container::Frame;
pub use date::Date;
pub use error::{Error, Result};
pub use person::Person;

/// A parsed save.
#[derive(Debug, Clone)]
pub struct Save {
    pub people: Vec<Person>,
    /// Decompressed size of every frame, kept so the UI can report what was
    /// read without holding 187 MB of frame payloads alive.
    pub frame_sizes: Vec<usize>,
}

impl Save {
    /// Parses a save from its raw bytes.
    ///
    /// # Errors
    /// Propagates [`Error`] when the file is not a save or a frame is corrupt.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let frames = container::read_frames(bytes)?;
        let frame_sizes = frames.iter().map(|f| f.data.len()).collect();

        // Records live in the single largest frame — 105 MB of the 187 MB total
        // in the reference save. Scanning every frame costs a lot for nothing.
        let people = frames
            .iter()
            .max_by_key(|f| f.data.len())
            .map(|f| person::scan_people(&f.data))
            .unwrap_or_default();

        Ok(Self { people, frame_sizes })
    }
}
