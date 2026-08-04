//! Squad-table checks that need the raw parse rather than the command layer:
//! captains and entity ids are not part of what the UI consumes yet.
//!
//! Skips rather than fails when no save is present, like the journey tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

fn load_named(name: &str) -> Option<fm_save::Save> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home)
        .join("Library/Application Support/Sports Interactive/Football Manager 26/games")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    Some(fm_save::Save::parse(&bytes).unwrap())
}

fn load() -> Option<fm_save::Save> {
    load_named("Career.fm")
}

/// The decoded value of a named attribute.
fn value_of(ability: &fm_save::Ability, name: &str) -> u8 {
    let index = (0..fm_save::ability::ATTRIBUTE_COUNT)
        .find(|&i| fm_save::ability::attribute_name(i) == Some(name))
        .unwrap_or_else(|| panic!("{name} is not a named attribute"));
    ability.attributes[index]
}

/// Asserts a whole in-game report screen against a decoded block.
fn assert_attributes(ability: &fm_save::Ability, shown: &[(&str, u8)]) {
    for (name, expected) in shown {
        assert_eq!(value_of(ability, name), *expected, "{name}");
    }
}

macro_rules! save_or_skip {
    () => {
        match load() {
            Some(s) => s,
            None => {
                eprintln!("skipped: no FM26 save on this machine");
                return;
            }
        }
    };
}

#[test]
fn squad_records_name_the_captains() {
    let save = save_or_skip!();

    let club_eid = |name: &str| -> u32 {
        save.clubs
            .iter()
            .find(|c| c.name == name)
            .and_then(|c| c.eid)
            .unwrap_or_else(|| panic!("{name} should have an entity id"))
    };
    let squad_of = |name: &str| -> &fm_save::Squad {
        let eid = club_eid(name);
        save.squads
            .iter()
            .find(|s| s.club_eid == eid)
            .unwrap_or_else(|| panic!("{name} should have a squad record"))
    };
    let person_name = |eid: Option<u32>| -> String {
        save.people
            .iter()
            .find(|p| p.eid == eid)
            .map(|p| p.full_name.clone())
            .unwrap_or_default()
    };

    // The reference save's real captaincies at its in-game date, October 2025.
    let city = squad_of("Manchester City");
    assert_eq!(person_name(city.captain_eid), "Bernardo Mota Veiga de Carvalho e Silva");
    assert_eq!(person_name(city.vice_captain_eid), "Rúben Santos Gato Alves Dias");

    let liverpool = squad_of("Liverpool");
    assert_eq!(person_name(liverpool.captain_eid), "Virgil van Dijk");

    let arsenal = squad_of("Arsenal");
    assert_eq!(person_name(arsenal.captain_eid), "Martin Ødegaard");

    // Squad sizes across the table are squad-shaped.
    assert!(save.squads.len() > 1_000, "got {} squads", save.squads.len());
    assert!(save.squads.iter().all(|s| s.player_eids.len() <= 60));
}

#[test]
fn known_entity_ids_read_back_exactly() {
    let save = save_or_skip!();

    // Values confirmed by hand against the reference save's bytes.
    let by_name = |name: &str| save.clubs.iter().find(|c| c.name == name).unwrap();
    assert_eq!(by_name("Manchester City").eid, Some(369));
    assert_eq!(by_name("Manchester City").uid, Some(678));
    assert_eq!(by_name("Arsenal").eid, Some(293));
    assert_eq!(by_name("Liverpool").eid, Some(366));

    let haaland = save
        .people
        .iter()
        .find(|p| p.full_name == "Erling Braut Haaland")
        .unwrap();
    assert_eq!(haaland.eid, Some(10_241));
    assert_eq!(haaland.uid, Some(29_179_241));
}

#[test]
fn contracts_match_public_figures() {
    let save = save_or_skip!();
    let person = |name: &str| save.people.iter().find(|p| p.full_name == name).unwrap();

    // FM Scout lists Haaland at £450K a week to 30 June 2034 in FM26.
    let haaland = person("Erling Braut Haaland");
    assert_eq!(haaland.wage, Some(450_000));
    let until = haaland.contract_until.unwrap();
    assert_eq!((until.year, until.month, until.day), (2034, 6, 30));

    // Salah and van Dijk both signed to 2027.
    assert_eq!(person("Mohamed Salah Ghaly").contract_until.map(|d| d.year), Some(2027));
    assert_eq!(person("Virgil van Dijk").contract_until.map(|d| d.year), Some(2027));

    // Foreign contracts convert to the display currency, so Madrid wages are
    // not round numbers — the signature of a real read, not a guess.
    let mbappe = person("Kylian Mbappé Lottin");
    let wage = mbappe.wage.unwrap();
    assert!((300_000..700_000).contains(&wage), "Mbappé wage {wage}");
    assert!(wage % 1000 != 0, "a converted wage should not be round");

    // Wages across the database form a pyramid: semi-pro at the median,
    // thousands in the professional tail.
    let mut wages: Vec<u32> = save.people.iter().filter_map(|p| p.wage).collect();
    wages.sort_unstable();
    assert!(wages.len() > 15_000, "expected contracts, got {}", wages.len());
    let p50 = wages[wages.len() / 2];
    let p90 = wages[wages.len() * 9 / 10];
    assert!(p50 < 5_000, "median weekly wage should be modest, got {p50}");
    assert!(p90 > 5_000, "the top decile should be professional, got {p90}");
}

/// An aged save is the hard case: a decade of training moves attribute
/// internals off the multiples of five, and transfers shuffle every squad
/// list out of entity-id order. The values asserted here are read straight
/// from the in-game player report for Jamal Musiala on 28 May 2035.
#[test]
fn an_aged_save_decodes_against_the_in_game_report() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };

    let musiala = save
        .people
        .iter()
        .find(|p| p.full_name == "Jamal Musiala")
        .expect("Musiala should be in the database");
    let ability = musiala.ability.as_ref().expect("Musiala is a player");

    assert_eq!((ability.current, ability.potential), (180, 185));

    // The club link survives the shuffled squad lists.
    let liverpool = save
        .clubs
        .iter()
        .find(|c| c.name == "Liverpool")
        .and_then(|c| c.eid)
        .expect("Liverpool should have an entity id");
    assert_eq!(musiala.club_eid, Some(liverpool));

    // His whole report screen, attribute by attribute.
    let shown = [
        ("Crossing", 13), ("Dribbling", 18), ("Finishing", 16), ("First Touch", 17),
        ("Heading", 13), ("Long Shots", 14), ("Marking", 12), ("Passing", 17),
        ("Tackling", 11), ("Technique", 18), ("Corners", 10), ("Free Kick Taking", 11),
        ("Long Throws", 5), ("Penalty Taking", 14), ("Aggression", 12), ("Anticipation", 16),
        ("Bravery", 13), ("Composure", 18), ("Concentration", 13), ("Decisions", 16),
        ("Determination", 16), ("Flair", 19), ("Leadership", 9), ("Off the Ball", 16),
        ("Positioning", 11), ("Teamwork", 14), ("Vision", 17), ("Work Rate", 15),
        ("Acceleration", 14), ("Agility", 18), ("Balance", 16), ("Jumping Reach", 12),
        ("Natural Fitness", 14), ("Pace", 15), ("Stamina", 14), ("Strength", 12),
    ];
    assert_attributes(ability, &shown);

    // Right-footed, and the game shows his left as "Fairly Strong".
    assert_eq!(value_of(ability, "Right Foot"), 20);
    assert_eq!(value_of(ability, "Left Foot"), 12);

    // His position ratings: natural AM(C), accomplished across the AM strip.
    assert_eq!(ability.natural_positions().first().copied(), Some("AMC"));

    // His header strip: a wage inside the scouted £350K-£425K band, contract
    // to 30 June 2037.
    assert_eq!(musiala.wage, Some(392_499));
    let until = musiala.contract_until.expect("contract expiry");
    assert_eq!((until.year, until.month, until.day), (2037, 6, 30));
}

/// The goalkeeping attributes are invisible on an outfielder's report, so a
/// keeper is the only way to check them. Values from Lucas Chevalier's
/// in-game report in the same 2035 save.
#[test]
fn a_goalkeepers_report_confirms_the_goalkeeping_set() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };

    let keeper = save
        .people
        .iter()
        .find(|p| p.full_name == "Lucas Chevalier")
        .expect("Chevalier should be in the database");
    let ability = keeper.ability.as_ref().expect("Chevalier is a player");
    assert_eq!(ability.natural_positions(), vec!["GK"]);

    assert_attributes(
        ability,
        &[
            // The goalkeeping set his screen shows in place of the technical one.
            ("Command of Area", 14),
            ("Kicking", 13),
            ("One on Ones", 18),
            ("Throwing", 16),
            ("Eccentricity", 14),
            ("Rushing Out Tendency", 15),
            ("Punching Tendency", 5),
            // Shared attributes, which must read the same for a keeper.
            ("First Touch", 14),
            ("Passing", 15),
            ("Free Kick Taking", 4),
            ("Penalty Taking", 5),
            ("Technique", 13),
            ("Anticipation", 16),
            ("Concentration", 13),
            ("Determination", 16),
            ("Leadership", 15),
            ("Positioning", 14),
            ("Jumping Reach", 15),
            ("Acceleration", 11),
            ("Pace", 12),
            ("Agility", 15),
            ("Strength", 12),
        ],
    );

    // His screen shows Aerial Reach, Communication, Handling and Reflexes
    // all at 15 — which is why this report alone could not separate them.
    // The split came from Donnarumma's published block instead (see
    // `published_keeper_attributes_split_the_last_four`); this report still
    // pins all four values.
    for name in ["Handling", "Aerial Reach", "Communication", "Reflexes"] {
        assert_eq!(value_of(ability, name), 15, "{name}");
    }

    // Outfielders score near nothing on the goalkeeping set — the property
    // the whole grouping was found by.
    let musiala = save
        .people
        .iter()
        .find(|p| p.full_name == "Jamal Musiala")
        .and_then(|p| p.ability.as_ref())
        .expect("Musiala");
    for index in fm_save::ability::GOALKEEPING_INDICES {
        assert!(musiala.attributes[index] <= 4, "outfielder at goalkeeping index {index}");
        assert!(ability.attributes[index] >= 5, "keeper at goalkeeping index {index}");
    }
}

/// FM 26.2.0 moved the header date; the main frame's week stamp stands in.
/// Without it, ages on an aged save compute against the real-world clock and
/// come out years wrong.
#[test]
fn an_aged_save_reads_its_own_date() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };
    let date = save.game_date.expect("26.2.0 save should read a date");
    // The stamp tracks the weekly rollover, so it sits within days of the
    // true 28 May 2035.
    assert_eq!(date.year, 2035);
    assert_eq!(date.month, 5);
}

/// The Afan Lido career regressed two ways at once: its week stamp carries
/// junk in the high bits (0x1A9F, not a day of year until masked), and the
/// men's Tottenham club record reads flags 0x12, which dropped the club and
/// left its whole squad showing no club.
#[test]
fn a_262_career_reads_its_date_and_flagged_clubs() {
    let Some(save) = load_named("Paul Dolden - Afan Lido.fm") else {
        eprintln!("skipped: no Afan Lido save on this machine");
        return;
    };

    // Day 159 of 2026, confirmed by the current-date stamps repeated through
    // the save's competition frames.
    let date = save.game_date.expect("masked week stamp should read");
    assert_eq!(date, fm_save::Date { year: 2026, month: 6, day: 8 });

    // The men's Tottenham record (eid 418) must exist alongside the women's
    // (eid 15924), and its squad must resolve — Pape Matar Sarr was one of
    // the contracted players showing no club.
    let spurs: Vec<_> = save
        .clubs
        .iter()
        .filter(|c| c.name == "Tottenham Hotspur")
        .collect();
    assert!(spurs.iter().any(|c| c.eid == Some(418)), "men's Spurs record missing");
    let squad = save
        .squads
        .iter()
        .find(|s| s.club_eid == 418)
        .expect("men's Spurs squad missing");
    let sarr = save
        .people
        .iter()
        .find(|p| p.full_name == "Pape Matar Sarr")
        .expect("Sarr missing");
    assert!(squad.player_eids.contains(&sarr.eid.unwrap()));
    assert_eq!(sarr.club_eid, Some(418));

    // Chelsea also reads flags 0x12 in this save; it must be present.
    assert!(save.clubs.iter().any(|c| c.name == "Chelsea" && c.eid.is_some()));
}

/// A club playing in one nation's pyramid from a ground in another kept no
/// entity id, because the club head's third u32 was read as a repeat of the
/// nation and required to match. It is not a repeat — it is where the club
/// sits, and the two differ for every cross-border club in the database.
///
/// The New Saints play in Wales (175) from Oswestry, England (139). Without
/// the entity id no squad record could reference them, so their entire first
/// team showed no club. The same head shape covers Cardiff, Swansea, Wrexham
/// and Newport County in the English pyramid, Derry City in the Irish one and
/// Berwick Rangers in the Scottish one.
#[test]
fn a_cross_border_club_resolves_its_squad() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let tns: Vec<_> = save.clubs.iter().filter(|c| c.name == "The New Saints").collect();
    assert!(!tns.is_empty(), "The New Saints missing entirely");
    assert!(
        tns.iter().all(|c| c.eid.is_some()),
        "a cross-border club must still carry an entity id"
    );
    // The club's own nation stays the pyramid it plays in, not where it sits.
    assert!(tns.iter().all(|c| c.nation_id == 175), "TNS should read as Welsh");

    // A day-one squad member rather than one signed later in a career, so the
    // anchor holds against the static save.
    let keeper = save
        .people
        .iter()
        .find(|p| p.full_name == "Nathan Shepperd")
        .expect("Nathan Shepperd missing from the people table");
    let club_eid = keeper.club_eid.expect("Shepperd should be linked to a club");
    assert!(
        tns.iter().any(|c| c.eid == Some(club_eid)),
        "Shepperd should be linked to The New Saints, got club eid {club_eid}"
    );

    // The men's squad resolves whole — the failure being pinned here was every
    // member showing no club, not a few going missing.
    let squad = save
        .squads
        .iter()
        .find(|s| s.club_eid == 1220)
        .expect("men's TNS squad missing");
    assert!(squad.player_eids.len() >= 20, "squad list unexpectedly short");
    let resolved = squad
        .player_eids
        .iter()
        .filter(|eid| save.people.iter().any(|p| p.eid == Some(**eid)))
        .count();
    assert_eq!(resolved, squad.player_eids.len(), "every member should resolve to a person");
}

/// The last four goalkeeping indices were separated by published FM 26 data
/// rather than an in-game screen: fminside.net serves the 26.2 database with
/// display×5 values (the same source class as the FM Scout wage check), and
/// Donnarumma's page splits all four — Handling 80, Aerial Reach 75,
/// Communication 70, Reflexes 90 — where the one in-game keeper report read
/// 15 across the board. His decoded block matches that split (16/15/14/18),
/// and Alisson's is consistent with his page (85/70/70/85 → 17/14/14/17).
#[test]
fn published_keeper_attributes_split_the_last_four() {
    let save = save_or_skip!();

    let block = |name: &str| -> &fm_save::Ability {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .and_then(|p| p.ability.as_ref())
            .unwrap_or_else(|| panic!("{name} should be a player"))
    };

    let donnarumma = block("Gianluigi Donnarumma");
    assert_eq!(value_of(donnarumma, "Handling"), 16);
    assert_eq!(value_of(donnarumma, "Aerial Reach"), 15);
    assert_eq!(value_of(donnarumma, "Communication"), 14);
    assert_eq!(value_of(donnarumma, "Reflexes"), 18);

    let alisson = block("Alisson Ramsés Becker");
    assert_eq!(value_of(alisson, "Handling"), 17);
    assert_eq!(value_of(alisson, "Aerial Reach"), 14);
    assert_eq!(value_of(alisson, "Communication"), 14);
    assert_eq!(value_of(alisson, "Reflexes"), 17);
}

/// The eight-byte run after the nation marker is the hidden personality set.
/// Slot names verified where a screen shows the value or the personality
/// label pins it: Adaptability is visible on staff reports, and the
/// Model Professional / Model Citizen labels constrain Professionalism.
#[test]
fn hidden_personality_matches_the_in_game_labels() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };
    let person = |name: &str| {
        save.people
            .iter()
            .find(|p| p.full_name.contains(name))
            .unwrap_or_else(|| panic!("{name} missing"))
    };

    // Adaptability is visible on staff screens: Leckie Elite, Ottley
    // Outstanding, Emery Good.
    assert_eq!(person("Isaac Leckie").adaptability(), Some(20));
    assert_eq!(person("Reece Thomas Ottley").adaptability(), Some(19));
    assert_eq!(person("Unai Emery").adaptability(), Some(13));

    // Emery is a Model Professional; Musiala a Model Citizen.
    assert_eq!(person("Unai Emery").professionalism(), Some(20));
    assert_eq!(person("Jamal Musiala").professionalism(), Some(16));
    assert_eq!(person("Jamal Musiala").personality, Some([18, 19, 15, 14, 16, 16, 18, 4]));

    // The run parses for nearly every adult. The misses are the simulated
    // children an aged save accumulates (born in-game, a different record
    // layout, junk nation ids) plus human-manager avatars.
    let adults: Vec<_> = save
        .people
        .iter()
        .filter(|p| p.nation_id <= 250 && p.date_of_birth.year < 2020)
        .collect();
    let with = adults.iter().filter(|p| p.personality.is_some()).count();
    assert!(
        with * 10 > adults.len() * 9,
        "only {with} of {} adults have personality",
        adults.len()
    );
}

/// The probe save was made after creating shortlists whose contents are
/// known exactly — the ground truth that unlocked `scout_man.dat`.
#[test]
fn probe_save_reads_the_in_game_shortlists() {
    let Some(save) = load_named("Probe.fm") else {
        eprintln!("skipped: no Probe.fm on this machine");
        return;
    };

    let names_in = |list: &fm_save::GameShortlist| -> Vec<String> {
        list.person_eids
            .iter()
            .map(|&eid| {
                save.people
                    .iter()
                    .find(|p| p.eid == Some(eid))
                    .unwrap_or_else(|| panic!("shortlist eid {eid} resolves to nobody"))
                    .full_name
                    .clone()
            })
            .collect()
    };

    let list = |name: &str| -> &fm_save::GameShortlist {
        save.shortlists
            .iter()
            .find(|s| s.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no shortlist named {name}"))
    };

    // Created in FM as ZZPROBE with exactly these three, in this order.
    assert_eq!(
        names_in(list("ZZPROBE")),
        vec!["Virgil van Dijk", "Florian Richard Wirtz", "Mohamed Salah Ghaly"]
    );
    // WirtzNew was imported back into the career from its .fmf export.
    assert_eq!(
        names_in(list("WirtzNew")),
        vec!["Florian Richard Wirtz", "Roberto Firmino Barbosa de Oliveira", "Mohamed Salah Ghaly"]
    );
    // The career's unnamed default list exists and resolves too.
    let unnamed = save.shortlists.iter().find(|s| s.name.is_none());
    assert!(unnamed.is_some(), "the unnamed default shortlist should parse");
}

/// The writer's safety proof, against the real probe save: taking the file
/// apart and reassembling it unchanged must reproduce it byte for byte, and
/// an in-memory shortlist edit must survive a full reparse. No file on disk
/// is touched.
#[test]
fn probe_save_survives_reassembly_and_a_shortlist_edit() {
    let Some(bytes) = ({
        std::env::var_os("HOME").and_then(|home| {
            std::fs::read(
                std::path::PathBuf::from(home)
                    .join("Library/Application Support/Sports Interactive/Football Manager 26/games")
                    .join("Probe.fm"),
            )
            .ok()
        })
    }) else {
        eprintln!("skipped: no Probe.fm on this machine");
        return;
    };

    // Identity: decompose + assemble with nothing changed is the same file.
    let d = fm_save::archive::decompose(&bytes).unwrap();
    let rebuilt =
        fm_save::archive::assemble(d.header, d.body, d.inner_header, d.manifest_frame).unwrap();
    assert_eq!(rebuilt, bytes, "identity reassembly must be byte-identical");

    // Edit: put Haaland on ZZPROBE, rebuild, reparse the whole save.
    let save = fm_save::Save::parse(&bytes).unwrap();
    let haaland = save
        .people
        .iter()
        .find(|p| p.full_name == "Erling Braut Haaland")
        .and_then(|p| p.eid)
        .expect("Haaland should have an entity id");

    let scout = fm_save::archive::member_plaintext(&bytes, "scout_man.dat").unwrap();
    // Date-added bytes observed in this save's own entries — the low half of
    // the field is undecoded, so nothing is invented here.
    let date = [0x9F, 0x1A, 0xEA, 0x07];
    let edited = fm_save::shortlist::add_entry(&scout, Some("ZZPROBE"), haaland, date).unwrap();
    let written = fm_save::archive::replace_member(&bytes, "scout_man.dat", &edited).unwrap();

    let reparsed = fm_save::Save::parse(&written).unwrap();
    let zzprobe = reparsed
        .shortlists
        .iter()
        .find(|s| s.name.as_deref() == Some("ZZPROBE"))
        .expect("ZZPROBE should still parse after the rewrite");
    assert_eq!(zzprobe.person_eids.len(), 4);
    assert_eq!(zzprobe.person_eids.last().copied(), Some(haaland));

    // The rest of the save is untouched by the edit: same people, same
    // captain, same other lists.
    assert_eq!(reparsed.people.len(), save.people.len());
    let wirtznew = |s: &fm_save::Save| {
        s.shortlists
            .iter()
            .find(|l| l.name.as_deref() == Some("WirtzNew"))
            .map(|l| l.person_eids.clone())
    };
    assert_eq!(wirtznew(&reparsed), wirtznew(&save));
}

/// The non-player sheet reads back exactly what the pre-game editor shows.
///
/// Both people are checked against their editor "All Attributes" page, and
/// both matter for a different reason. Nikolić is a manager with a full
/// database row whose object carries a preamble, so a fixed stride to the five
/// u16s reads garbage. Fradley is a player-analyst: his own record holds no
/// sheet at all, and reading the block off the object sharing his eid gives
/// the next person's numbers — the mistake that defeated every earlier attempt
/// (`docs/OPEN_PROBLEMS.md` §3b).
#[test]
fn staff_sheets_match_the_editor() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipping: no Day One.fm");
        return;
    };

    let sheet = |name: &str| {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .unwrap_or_else(|| panic!("{name} is not in the save"))
            .staff
            .clone()
            .unwrap_or_else(|| panic!("{name} has no non-player sheet"))
    };

    // Editor: Current Ability 130, Potential 145, Current Reputation 130,
    // World 90, Home 125.
    let nikolic = sheet("Marko Nikolić");
    assert_eq!(nikolic.eid, 5155, "the sheet is one entity id below the person");
    assert_eq!((nikolic.current_ability, nikolic.potential_ability), (130, 145));
    assert_eq!(
        (
            nikolic.home_reputation,
            nikolic.current_reputation,
            nikolic.world_reputation
        ),
        (125, 130, 90),
        "the triple is home, current, world — not the editor's print order"
    );
    for (name, expected) in [
        ("Attacking", 9),
        ("Directness", 14),
        ("Authority", 17),
        ("Trigger Press", 16),
        ("Working With Youngsters", 11),
        ("Buying Players", 10),
        ("Mind Games", 12),
        ("Squad Rotation", 8),
        ("Judging Player Ability", 12),
        ("Judging Player Potential", 10),
        ("People Management", 10),
        ("Motivating", 16),
        ("Tactical Knowledge", 13),
        ("Coaching Attacking", 9),
        ("Coaching Defending", 16),
        ("Coaching Fitness", 11),
        ("Coaching Possession", 13),
        ("Coaching Technical", 13),
        ("Coaching Tactical", 14),
        ("Coaching Set Pieces", 12),
    ] {
        assert_eq!(nikolic.get(name), Some(expected), "Nikolić {name}");
    }

    // Editor: CA 140, PA 150, Current Reputation 140, World 110, Home 127.
    let fradley = sheet("Daniel Fradley");
    assert_eq!(fradley.eid, 20129);
    assert_eq!((fradley.current_ability, fradley.potential_ability), (140, 150));
    assert_eq!(
        (
            fradley.home_reputation,
            fradley.current_reputation,
            fradley.world_reputation
        ),
        (127, 140, 110)
    );
    for (name, expected) in [
        ("Working With Youngsters", 12),
        ("Judging Player Ability", 12),
        ("Judging Player Potential", 14),
        ("Judging Player Data", 16),
        ("People Management", 12),
        ("Motivating", 10),
        ("Tactical Knowledge", 16),
        ("Coaching Attacking", 5),
        ("Coaching Defending", 5),
        ("Coaching Fitness", 1),
        ("Coaching Possession", 6),
        ("Coaching Technical", 16),
        ("Coaching Tactical", 16),
        ("Coaching Set Pieces", 12),
    ] {
        assert_eq!(fradley.get(name), Some(expected), "Fradley {name}");
    }
}
