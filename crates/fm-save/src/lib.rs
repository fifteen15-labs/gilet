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
pub mod squad;
pub mod strings;

pub use ability::Ability;
pub use club::Club;
pub use container::Frame;
pub use date::Date;
pub use error::{Error, Result};
pub use person::Person;
pub use squad::Squad;

/// A parsed save.
#[derive(Debug, Clone)]
pub struct Save {
    pub people: Vec<Person>,
    pub clubs: Vec<Club>,
    /// One record per club that fields a first-team squad, referencing people
    /// by entity id. `Person::club_eid` is the same link from the other side.
    pub squads: Vec<Squad>,
    /// The save's own in-game date, when it can be read. `None` for FM 26.2.0
    /// saves, which encode it differently.
    pub game_date: Option<Date>,
    /// Decompressed size of every frame, kept so the UI can report what was
    /// read without holding 187 MB of frame payloads alive.
    pub frame_sizes: Vec<usize>,
}

/// A stage of parsing, reported so a caller can show progress.
///
/// Reading a 190 MB save takes long enough that a UI must say what it is
/// doing; the variants are in the order they occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Decompressing,
    ReadingNames,
    ReadingPeople,
    ReadingIdentities,
    ReadingContracts,
    ReadingClubs,
    ReadingSquads,
    ReadingAbility,
    Done,
}

impl Stage {
    /// Every stage, in order.
    pub const ALL: [Self; 9] = [
        Self::Decompressing,
        Self::ReadingNames,
        Self::ReadingPeople,
        Self::ReadingIdentities,
        Self::ReadingContracts,
        Self::ReadingClubs,
        Self::ReadingSquads,
        Self::ReadingAbility,
        Self::Done,
    ];

    /// How far through parsing this stage is, 0.0 to 1.0.
    ///
    /// Weighted by measured cost rather than spread evenly: decompression and
    /// the ability scan dominate, and a bar that jumps in even steps for
    /// wildly uneven work reads as broken.
    #[must_use]
    pub fn progress(self) -> f32 {
        match self {
            Self::Decompressing => 0.05,
            Self::ReadingNames => 0.35,
            Self::ReadingPeople => 0.45,
            Self::ReadingIdentities => 0.55,
            Self::ReadingContracts => 0.65,
            Self::ReadingClubs => 0.70,
            Self::ReadingSquads => 0.78,
            Self::ReadingAbility => 0.88,
            Self::Done => 1.0,
        }
    }

    /// What to show the user while this stage runs.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Decompressing => "Decompressing frames",
            Self::ReadingNames => "Reading the name table",
            Self::ReadingPeople => "Reading players and staff",
            Self::ReadingIdentities => "Matching identities",
            Self::ReadingContracts => "Reading contracts",
            Self::ReadingClubs => "Reading clubs",
            Self::ReadingSquads => "Linking squads",
            Self::ReadingAbility => "Reading ability and attributes",
            Self::Done => "Done",
        }
    }
}

impl Save {
    /// Parses a save from its raw bytes.
    ///
    /// # Errors
    /// Propagates [`Error`] when the file is not a save or a frame is corrupt.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        Self::parse_with_progress(bytes, |_| {})
    }

    /// Parses a save, reporting each [`Stage`] as it begins.
    ///
    /// # Errors
    /// Propagates [`Error`] when the file is not a save or a frame is corrupt.
    pub fn parse_with_progress(bytes: &[u8], mut on_stage: impl FnMut(Stage)) -> Result<Self> {
        on_stage(Stage::Decompressing);
        let frames = container::read_frames(bytes)?;
        let frame_sizes = frames.iter().map(|f| f.data.len()).collect();

        // Records live in the single largest frame — 105 MB of the 187 MB total
        // in the reference save. Scanning every frame costs a lot for nothing.
        let main = frames.iter().max_by_key(|f| f.data.len());

        let mut people = Vec::new();
        let mut clubs = Vec::new();
        let mut squads = Vec::new();
        if let Some(frame) = main {
            // People sit after the string table their names reference; both
            // scans need it, so it is parsed once and dropped when done.
            on_stage(Stage::ReadingNames);
            let table = strings::scan_strings(&frame.data);

            let mut chain = Vec::new();
            if let Some(table) = &table {
                on_stage(Stage::ReadingPeople);
                people = person::scan_people(&frame.data, table);
                on_stage(Stage::ReadingIdentities);
                chain = person::bind_identities(&frame.data, &mut people, table.end_offset);
                on_stage(Stage::ReadingContracts);
                person::bind_contracts(&frame.data, &mut people);
            }

            on_stage(Stage::ReadingClubs);
            clubs = club::scan_clubs(&frame.data);

            if table.is_some() {
                on_stage(Stage::ReadingSquads);
                let club_ids: Vec<(u32, u32)> = clubs
                    .iter()
                    .filter_map(|c| Some((c.eid?, c.uid?)))
                    .collect();
                squads = squad::scan_squads(&frame.data, &club_ids);
                link_members(&mut people, &squads, &chain);

                // Gender falls out of the squad structure: squads are
                // single-gender and the female forename pool is the tail of
                // the name table, so the split derives from the save itself.
                let boundary = person::female_forename_boundary(&people, &squads);
                person::bind_gender(&mut people, boundary);
            }

            // Attribute blocks sit ahead of the person they belong to, so they
            // are scanned separately and matched on afterwards.
            on_stage(Stage::ReadingAbility);
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

        // The in-game date lives in the small header frame on 26.0.0; on
        // 26.2.0 it moved, and the main frame's week stamp stands in — at
        // most a week stale, against years wrong from a system-clock
        // fallback.
        let game_date = frames
            .first()
            .and_then(|f| gamedate::find_game_date(&f.data))
            .or_else(|| main.and_then(|f| gamedate::find_main_frame_date(&f.data)));

        on_stage(Stage::Done);
        Ok(Self {
            people,
            clubs,
            squads,
            game_date,
            frame_sizes,
        })
    }
}

/// Sets `Person::club_eid` from the squad table.
///
/// Squad members are resolved through the identity chain rather than only
/// each person's own entity id: a record can contain a second identity block
/// whose prefix went undetected, and resolving through the chain still lands
/// on the right record. Resolution is by containing record — the person whose
/// prefix is the last one before the block.
fn link_members(people: &mut [Person], squads: &[squad::Squad], chain: &[person::Identity]) {
    let offsets: Vec<usize> = people.iter().map(|p| p.offset).collect();
    let mut eid_to_person: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for id in chain {
        let idx = offsets.partition_point(|&o| o <= id.offset);
        if let Some(i) = idx.checked_sub(1) {
            eid_to_person.entry(id.eid).or_insert(i);
        }
    }
    for s in squads {
        for member in &s.player_eids {
            if let Some(person) = eid_to_person.get(member).and_then(|&i| people.get_mut(i)) {
                if person.club_eid.is_none() {
                    person.club_eid = Some(s.club_eid);
                }
            }
        }
    }
}
