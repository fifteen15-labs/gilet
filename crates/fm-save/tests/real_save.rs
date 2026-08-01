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

    // Every named attribute, exactly as the report screen shows it.
    let shown = [
        (0, 13, "Crossing"),
        (2, 16, "Finishing"),
        (3, 13, "Heading"),
        (5, 12, "Marking"),
        (6, 16, "Off the Ball"),
        (8, 14, "Penalty Taking"),
        (9, 11, "Tackling"),
        (10, 17, "Passing"),
        (20, 11, "Positioning"),
        (23, 18, "Technique"),
        (26, 19, "Flair"),
        (27, 10, "Corners"),
        (29, 15, "Work Rate"),
        (30, 5, "Long Throws"),
        (34, 14, "Acceleration"),
        (35, 11, "Free Kick Taking"),
        (36, 12, "Strength"),
        (38, 15, "Pace"),
        (39, 12, "Jumping Reach"),
        (40, 9, "Leadership"),
    ];
    for (index, value, name) in shown {
        assert_eq!(fm_save::ability::attribute_name(index), Some(name));
        assert_eq!(
            ability.attributes.get(index).copied(),
            Some(value),
            "{name} should read {value}"
        );
    }

    // His position ratings: natural AM(C), accomplished across the AM strip.
    assert_eq!(ability.natural_positions().first().copied(), Some("AMC"));

    // His header strip: a wage inside the scouted £350K-£425K band, contract
    // to 30 June 2037.
    assert_eq!(musiala.wage, Some(392_499));
    let until = musiala.contract_until.expect("contract expiry");
    assert_eq!((until.year, until.month, until.day), (2037, 6, 30));
}
