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

    // The four unseparated goalkeeping indices all read 15 on his screen —
    // Aerial Reach, Communication, Handling and Reflexes in some order.
    for index in [11, 12, 14, 21] {
        assert_eq!(fm_save::ability::attribute_name(index), None);
        assert_eq!(ability.attributes[index], 15, "index {index}");
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
