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

pub mod ability;
pub mod club;
pub mod container;
pub mod date;
pub mod error;
pub mod gamedate;
pub mod person;

pub use ability::Ability;
pub use club::Club;
pub use container::Frame;
pub use date::Date;
pub use error::{Error, Result};
pub use person::Person;

/// A parsed save.
#[derive(Debug, Clone)]
pub struct Save {
    pub people: Vec<Person>,
    pub clubs: Vec<Club>,
    /// The save's own in-game date, when it can be read. `None` for FM 26.2.0
    /// saves, which encode it differently.
    pub game_date: Option<Date>,
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
        let main = frames.iter().max_by_key(|f| f.data.len());
        let mut people = main.map(|f| person::scan_people(&f.data)).unwrap_or_default();
        let clubs = main.map(|f| club::scan_clubs(&f.data)).unwrap_or_default();

        // Attribute blocks sit ahead of the person they belong to, so they are
        // scanned separately and matched on afterwards.
        if let Some(frame) = main {
            let abilities = ability::scan_abilities(&frame.data);
            let offsets: Vec<usize> = people.iter().map(|p| p.offset).collect();
            for (ability, owner) in abilities
                .iter()
                .zip(ability::match_to_people(&abilities, &offsets))
            {
                if let Some(person) = owner.and_then(|i| people.get_mut(i)) {
                    person.ability = Some(ability.clone());
                }
            }
        }

        // The in-game date lives in the small header frame, not the database.
        let game_date = frames.first().and_then(|f| gamedate::find_game_date(&f.data));

        Ok(Self {
            people,
            clubs,
            game_date,
            frame_sizes,
        })
    }
}
