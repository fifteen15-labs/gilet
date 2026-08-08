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
pub mod archive;
pub mod backroom;
pub mod club;
pub mod container;
pub mod date;
pub mod error;
pub mod gamedate;
pub mod humans;
pub mod manifest;
pub mod person;
pub mod shortlist;
pub mod squad;
pub mod staff;
pub mod strings;
pub mod stub;
pub mod tactic;

pub use ability::Ability;
pub use club::Club;
pub use container::Frame;
pub use date::Date;
pub use error::{Error, Result};
pub use person::Person;
pub use shortlist::GameShortlist;
pub use squad::Squad;

/// A parsed save.
#[derive(Debug, Clone)]
pub struct Save {
    pub people: Vec<Person>,
    pub clubs: Vec<Club>,
    /// One record per club that fields a first-team squad, referencing people
    /// by entity id. `Person::club_eid` is the same link from the other side.
    pub squads: Vec<Squad>,
    /// B-team and youth squads, read from the same table's 0x13/0x15-typed
    /// rows. They bind `Person::club_eid` for players no first-team squad
    /// claims — the players outside the loaded leagues — but stay out of
    /// [`Save::squads`] so a club's squad size, wage bill and audit keep
    /// meaning first team.
    pub team_squads: Vec<Squad>,
    /// Squad fillers with no person record — generated non-contract players
    /// stored as entity stubs. Only stubs whose entity id no parsed person
    /// claims are kept, so a squad list can show every member it references.
    pub stubs: Vec<stub::Stub>,
    /// The save's own in-game date, when it can be read: the header pair on
    /// 26.0.0, the main frame's masked week stamp on 26.2.0 (at most a week
    /// stale). `None` only when neither source decodes.
    pub game_date: Option<Date>,
    /// The human manager's in-game shortlists, from the `scout_man.dat`
    /// member. Empty when the save's manifest or that member cannot be read.
    pub shortlists: Vec<GameShortlist>,
    /// The human manager's person entity id, from the `humans.dat` member.
    /// Their club is that person's `club_eid`; `None` there means genuinely
    /// unemployed — an unemployed career's member reads exactly this way.
    pub human_eid: Option<u32>,
    /// The human's active tactic, from the `tactics_man.dat` member. `None`
    /// on a career that has not set one up yet — the member says so itself.
    pub tactic: Option<tactic::Tactic>,
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

        let main = main_frame(&frames);

        let mut people = Vec::new();
        let mut clubs = Vec::new();
        let mut squads = Vec::new();
        let mut team_squads = Vec::new();
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

                link_backroom(&frame.data, &mut people, &clubs, &club_ids);

                // B and youth squads bind last, and only people neither a
                // first-team list nor the backroom claimed — a player-coach
                // in a youth list keeps the staff role and club the backroom
                // pass gave them. A player appearing in the lists of two
                // *different* clubs stays unbound: an ambiguous read is not
                // a link.
                team_squads = squad::scan_team_squads(&frame.data, &club_ids);
                link_team_members(&mut people, &team_squads, &chain);

                // Gender falls out of the squad structure: squads are
                // single-gender and the female forename pool is the tail of
                // the name table, so the split derives from the save itself.
                let boundary = person::female_forename_boundary(&people, &squads);
                person::bind_gender(&mut people, boundary);
            }

            // Non-player sheets sit on the entity object one eid below the
            // person's own, so they are matched by id rather than by position.
            // The same walk collects the tag-02 player lines, which bind
            // below — after abilities, whose CA/PA pair is their entry test.
            let (sheets, lines) = staff::scan_objects(&frame.data);
            let mut staff_by_eid: std::collections::HashMap<u32, staff::Staff> = sheets
                .into_iter()
                .map(|s| (s.eid.saturating_add(1), s))
                .collect();
            for person in &mut people {
                if let Some(sheet) = person.eid.and_then(|e| staff_by_eid.remove(&e)) {
                    person.staff = Some(sheet);
                }
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

            // Player reputation lines use the sheets' one-eid-below key, and
            // must also repeat the person's parsed CA/PA exactly — the pair
            // works as a checksum, so a chance triple or a neighbour's line
            // has no way to bind. A line that matches nobody stays unbound;
            // a person without one keeps `None`, which the UI must show as
            // undecoded rather than unknown-and-lowly.
            let mut lines_by_eid: std::collections::HashMap<u32, staff::PlayerLine> =
                lines.into_iter().map(|l| (l.eid.saturating_add(1), l)).collect();
            for person in &mut people {
                let Some(line) = person.eid.and_then(|e| lines_by_eid.get(&e)) else {
                    continue;
                };
                let matches = person.ability.as_ref().is_some_and(|a| {
                    u16::from(a.current) == line.current_ability
                        && u16::from(a.potential) == line.potential_ability
                });
                if matches {
                    person.reputation = Some(person::Reputation {
                        home: line.home_reputation,
                        current: line.current_reputation,
                        world: line.world_reputation,
                    });
                    if let Some(eid) = person.eid {
                        lines_by_eid.remove(&eid);
                    }
                }
            }

            // Compact entries — people aged saves fold down to a name and an
            // identity (person.rs, `scan_compact`) — join last, after every
            // offset-based pass: their offsets sit inside other people's
            // records, and letting them into the earlier passes would shift
            // record boundaries and hand them a neighbour's contract. An eid
            // a full record already claims stays with the record.
            if let Some(table) = &table {
                let claimed: std::collections::HashSet<u32> =
                    people.iter().filter_map(|p| p.eid).collect();
                people.extend(
                    person::scan_compact(&frame.data, table, table.end_offset)
                        .into_iter()
                        .filter(|p| p.eid.is_some_and(|e| !claimed.contains(&e))),
                );
            }
        }

        // The in-game date is the database frame's week stamp — at most a week
        // stale, against years wrong from a system-clock fallback. The header
        // frame's own stamp is the real-world time the file was written and is
        // never used for ages: it read as the in-game date only on a save
        // played within weeks of its own calendar.
        let game_date = main.and_then(|f| gamedate::find_main_frame_date(&f.data));

        let shortlists = named_frame(&frames, "scout_man.dat")
            .map(|f| shortlist::scan_shortlists(&f.data))
            .unwrap_or_default();

        // The human's identity only counts when it resolves to a parsed
        // person — an eid pointing at nobody reads as no reading.
        let human_eid = named_frame(&frames, "humans.dat")
            .and_then(|f| humans::scan_human(&f.data))
            .filter(|eid| people.iter().any(|p| p.eid == Some(*eid)));

        let tactic = named_frame(&frames, "tactics_man.dat")
            .and_then(|f| tactic::scan_tactic(&f.data));

        // Stub people are only worth keeping where a squad references an
        // entity id that no parsed person answers to — those members would
        // otherwise vanish from squad lists without a trace.
        let mut stubs = Vec::new();
        if let Some(frame) = main {
            let bound: std::collections::HashSet<u32> =
                people.iter().filter_map(|p| p.eid).collect();
            let referenced: std::collections::HashSet<u32> = squads
                .iter()
                .flat_map(|s| s.player_eids.iter().copied())
                .collect();
            stubs = stub::scan_stubs(&frame.data)
                .into_iter()
                .filter(|s| referenced.contains(&s.eid) && !bound.contains(&s.eid))
                .collect();
        }

        on_stage(Stage::Done);
        Ok(Self {
            people,
            clubs,
            squads,
            team_squads,
            stubs,
            game_date,
            shortlists,
            human_eid,
            tactic,
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
/// The database frame, where every record scan runs. `game_db.dat` through
/// the manifest; the single-largest-frame heuristic — right on every save
/// until a long career grew `player_stats_hist_dt.cmt` past the database
/// (372 MB against 351 MB) and the whole save read as empty — stays as the
/// fallback for a save whose manifest does not parse.
fn main_frame(frames: &[Frame]) -> Option<&Frame> {
    named_frame(frames, "game_db.dat")
        .or_else(|| frames.iter().max_by_key(|f| f.data.len()))
}

/// The decompressed frame of the named member, located through the manifest
/// in the final frame. The manifest's sorted position is the frame index, and
/// the member's plaintext length must match the frame byte-for-byte — a
/// mismatch means the mapping is wrong, and no data is better than data read
/// out of someone else's bytes.
fn named_frame<'a>(frames: &'a [Frame], name: &str) -> Option<&'a Frame> {
    let members = manifest::read_manifest(&frames.last()?.data)?;
    let index = manifest::frame_index_of(&members, name)?;
    let frame = frames.get(index)?;
    let plain = members.get(index).map(|m| m.plain)?;
    (frame.data.len() as u64 == plain).then_some(frame)
}

/// Sets `Person::club_eid` for the backroom: managers from the roster
/// table's slot, then the staff lists in each club record's body.
fn link_backroom(frame: &[u8], people: &mut [Person], clubs: &[Club], club_ids: &[(u32, u32)]) {
    link_managers(frame, people, club_ids);
    link_staff(frame, people, clubs);
}

/// Sets `Person::club_eid` for managers, from the roster table's manager
/// slot. Managers are never in the squad table.
fn link_managers(frame: &[u8], people: &mut [Person], club_ids: &[(u32, u32)]) {
    let mut by_eid: std::collections::HashMap<u32, usize> = people
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some((p.eid?, i)))
        .collect();
    for m in backroom::scan_managers(frame, club_ids) {
        if let Some(person) = by_eid.remove(&m.manager_eid).and_then(|i| people.get_mut(i)) {
            if person.club_eid.is_none() {
                person.club_eid = Some(m.club_eid);
                person.staff_role = Some(backroom::Role::Manager);
            }
        }
    }
}

/// Sets `Person::club_eid` for the rest of the backroom, from the staff
/// lists inside each club record's body.
///
/// Each list is gated on its own members: at least four in five must
/// resolve to non-player people, or the whole list is dropped — an
/// ascending run of some other entity kind must not hand people a club.
/// Individual members only bind when they are themselves non-players with
/// no club yet, so a player eid colliding with a list never loses their
/// squad club.
fn link_staff(frame: &[u8], people: &mut [Person], clubs: &[Club]) {
    let spans: Vec<(usize, u32)> = clubs.iter().filter_map(|c| Some((c.offset, c.eid?))).collect();
    let by_eid: std::collections::HashMap<u32, usize> = people
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some((p.eid?, i)))
        .collect();
    for list in backroom::scan_staff_lists(frame, &spans) {
        let members: Vec<usize> = list.eids.iter().filter_map(|e| by_eid.get(e).copied()).collect();
        let non_players = members
            .iter()
            .filter(|&&i| people.get(i).is_some_and(|p| p.ability.is_none()))
            .count();
        if non_players * 5 < list.eids.len() * 4 {
            continue;
        }
        for i in members {
            if let Some(person) = people.get_mut(i) {
                if person.ability.is_none() && person.club_eid.is_none() {
                    person.club_eid = Some(list.club_eid);
                    person.staff_role = list.department.map(backroom::Role::from);
                }
            }
        }
    }
}

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

/// Sets `Person::club_eid` from the B, youth and out-of-league squad lists,
/// for people the first-team pass left unbound.
///
/// A person can sit in several such lists — a B squad and a youth squad of
/// the same club is fine and binds; lists of two different clubs is a
/// contradiction, and the person stays unbound rather than get a guessed
/// club. Measured on a 2035 save that is single digits of conflicted people
/// against tens of thousands bound.
///
/// An out-of-league list carries one extra check before it may bind: when
/// three or more of its members already have a first-team club and their
/// majority is *not* this list's club, the whole list is refused. A real
/// out-of-league squad's members are precisely the people no loaded squad
/// knows — a list full of already-placed players pointing elsewhere is
/// noise wearing a club's entity id. B and youth lists are exempt: their
/// known members are loanees, and loanees genuinely point elsewhere.
fn link_team_members(people: &mut [Person], team_squads: &[squad::Squad], chain: &[person::Identity]) {
    let offsets: Vec<usize> = people.iter().map(|p| p.offset).collect();
    let mut eid_to_person: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for id in chain {
        let idx = offsets.partition_point(|&o| o <= id.offset);
        if let Some(i) = idx.checked_sub(1) {
            eid_to_person.entry(id.eid).or_insert(i);
        }
    }

    let mut club_of: std::collections::HashMap<u32, Option<u32>> = std::collections::HashMap::new();
    for s in team_squads {
        if s.kind == squad::SquadKind::OutOfLeague {
            let mut counts: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
            let mut known = 0usize;
            for member in &s.player_eids {
                let bound = eid_to_person
                    .get(member)
                    .and_then(|&i| people.get(i))
                    .and_then(|p| p.club_eid);
                if let Some(c) = bound {
                    *counts.entry(c).or_default() += 1;
                    known += 1;
                }
            }
            let majority = counts.iter().max_by_key(|(_, &n)| n).map(|(&c, _)| c);
            if known >= 3 && majority != Some(s.club_eid) {
                continue;
            }
        }
        for member in &s.player_eids {
            club_of
                .entry(*member)
                .and_modify(|c| {
                    if *c != Some(s.club_eid) {
                        *c = None;
                    }
                })
                .or_insert(Some(s.club_eid));
        }
    }

    for (member, club) in club_of {
        let Some(club) = club else { continue };
        if let Some(person) = eid_to_person.get(&member).and_then(|&i| people.get_mut(i)) {
            if person.club_eid.is_none() {
                person.club_eid = Some(club);
            }
        }
    }
}
