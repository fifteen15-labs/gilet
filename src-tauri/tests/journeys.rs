//! End-to-end checks against a real save.
//!
//! These exercise the commands the UI calls, so the open → filter → export →
//! import path is covered even though the UI itself is driven by hand. They
//! skip rather than fail when no save is present, so the suite still passes on
//! a machine without Football Manager installed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use gilet_lib::commands;

/// The save these journeys run against: `Career.fm` in FM's own games folder,
/// or whatever `GILET_TEST_SAVE` points at. The override is how a machine
/// whose reference save is named something else — or is a career several
/// seasons in — still runs the suite rather than skipping it silently.
fn save_path() -> Option<std::path::PathBuf> {
    if let Some(named) = std::env::var_os("GILET_TEST_SAVE") {
        let p = std::path::PathBuf::from(named);
        return p.exists().then_some(p);
    }
    let home = std::env::var_os("HOME")?;
    let p = std::path::PathBuf::from(home)
        .join("Library/Application Support/Sports Interactive/Football Manager 26/games/Career.fm");
    p.exists().then_some(p)
}

fn today() -> Vec<u16> {
    vec![2026, 8, 1]
}

/// Loads the reference save once per test. Returns `None` when the machine has
/// no Football Manager install, so the suite still passes there.
fn load() -> Option<commands::SaveSummary> {
    let path = save_path()?;
    Some(commands::load_save(path.display().to_string(), &today(), |_| {}).unwrap())
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
fn opens_a_real_save_and_finds_people_and_clubs() {
    let summary = save_or_skip!();

    assert!(summary.players.len() > 5_000, "expected a populated database, got {}", summary.players.len());
    assert!(summary.clubs.len() > 1_000, "expected clubs, got {}", summary.clubs.len());
    assert!(summary.frames > 1_000);

    let haaland = summary
        .players
        .iter()
        .find(|p| p.name == "Erling Braut Haaland")
        .expect("Haaland should be in the database");
    assert_eq!(haaland.born, "2000-07-21");

    let city = summary
        .clubs
        .iter()
        .find(|c| c.name == "Manchester City")
        .expect("Manchester City should be in the database");
    assert_eq!(city.short_name, "Man City");
    assert_eq!(city.club_id, 1075);
}

#[test]
fn decodes_ability_and_separates_players_from_staff() {
    let summary = save_or_skip!();

    // FM's real ratings for a player whose quality is not in doubt.
    let haaland = summary
        .players
        .iter()
        .find(|p| p.name == "Erling Braut Haaland")
        .expect("Haaland should be in the database");
    assert_eq!(haaland.ability, Some(184), "Haaland's Current Ability");
    assert_eq!(haaland.potential, Some(195), "Haaland's Potential Ability");
    assert!(haaland.is_player);

    // Potential Ability is never below Current Ability, for every player.
    let players: Vec<_> = summary.players.iter().filter(|p| p.is_player).collect();
    assert!(players.len() > 1_000, "expected players, got {}", players.len());
    for p in &players {
        let (Some(ca), Some(pa)) = (p.ability, p.potential) else {
            panic!("a player must have both ability values: {}", p.name);
        };
        assert!(pa >= ca, "{}: PA {pa} below CA {ca}", p.name);
        assert!((1..=200).contains(&ca) && (1..=200).contains(&pa), "{}: out of range", p.name);
    }

    // Staff carry no attribute block, which is what separates them.
    let staff: Vec<_> = summary.players.iter().filter(|p| !p.is_player).collect();
    assert!(!staff.is_empty());
    assert!(staff.iter().all(|p| p.ability.is_none()));
    assert!(staff.iter().all(|p| p.attributes.is_empty()));
}

#[test]
fn decodes_attributes_and_the_goalkeeping_set() {
    let summary = save_or_skip!();
    let players: Vec<_> = summary.players.iter().filter(|p| p.is_player).collect();

    for p in players.iter().take(500) {
        assert_eq!(p.attributes.len(), 54, "{}: expected 54 attributes", p.name);
        assert!(
            p.attributes.iter().all(|a| (1..=20).contains(a)),
            "{}: attribute outside 1-20",
            p.name
        );
    }

    assert_eq!(summary.goalkeeping_indices.len(), 11);

    // Keepers score highly on the goalkeeping set while staying a minority.
    // This is the property the whole grouping rests on.
    let gk_mean = |p: &&&commands::PlayerRow| -> f64 {
        let total: u32 = summary
            .goalkeeping_indices
            .iter()
            .filter_map(|&i| p.attributes.get(i))
            .map(|&v| u32::from(v))
            .sum();
        f64::from(total) / 11.0
    };
    let keepers = players.iter().filter(|p| gk_mean(p) > 8.0).count();
    assert!(keepers > 50, "expected goalkeepers, found {keepers}");
    assert!(keepers * 3 < players.len(), "keepers should be a minority");
}

#[test]
fn named_attributes_match_the_players_they_describe() {
    let summary = save_or_skip!();

    let value = |name: &str, attribute: &str| -> u8 {
        let index = summary
            .attribute_names
            .iter()
            .position(|n| n == attribute)
            .unwrap_or_else(|| panic!("{attribute} is not a named attribute"));
        summary
            .players
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.attributes.get(index).copied())
            .unwrap_or_else(|| panic!("{name} has no attributes"))
    };

    // A striker finishes and does not mark.
    assert!(value("Erling Braut Haaland", "Finishing") >= 15);
    assert!(value("Erling Braut Haaland", "Marking") <= 8);

    // A centre-back marks and tackles but does not finish.
    assert!(value("Rúben Santos Gato Alves Dias", "Marking") >= 15);
    assert!(value("Rúben Santos Gato Alves Dias", "Tackling") >= 15);
    assert!(value("Rúben Santos Gato Alves Dias", "Finishing") <= 9);

    // A goalkeeper does none of the outfield skills, but reads high on
    // positioning and jumping — the split that separated Heading from
    // Jumping Reach in the first place.
    assert!(value("Alisson Ramsés Becker", "Crossing") <= 3);
    assert!(value("Alisson Ramsés Becker", "Tackling") <= 3);
    assert!(value("Alisson Ramsés Becker", "Jumping Reach") >= 12);
    assert!(value("Alisson Ramsés Becker", "Heading") <= 9);
}

#[test]
fn decodes_positions() {
    let summary = save_or_skip!();
    assert_eq!(summary.position_names.len(), 15);

    let position_of = |name: &str| -> Vec<String> {
        summary
            .players
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.positions.clone())
            .unwrap_or_default()
    };
    // These would be the first to fail if the slot order were wrong.
    assert_eq!(position_of("Erling Braut Haaland"), vec!["ST"], "Haaland is a striker");
    assert!(position_of("Bukayo Ayoyinka Saka").contains(&"AMR".to_owned()), "Saka plays right");
    assert!(position_of("Kylian Mbappé Lottin").contains(&"AML".to_owned()), "Mbappé plays left");

    // Centre-back is the most common position in any database, and sweeper is
    // nobody's in a modern one.
    let mut counts = std::collections::HashMap::new();
    for p in summary.players.iter().filter(|p| p.is_player) {
        if let Some(first) = p.positions.first() {
            *counts.entry(first.clone()).or_insert(0usize) += 1;
        }
    }
    let dc = counts.get("DC").copied().unwrap_or(0);
    assert!(dc > 200, "expected plenty of centre-backs, got {dc}");
    assert_eq!(counts.get("SW").copied().unwrap_or(0), 0, "sweeper should be unused");
}

#[test]
fn decodes_nationality() {
    let summary = save_or_skip!();

    let nation_of = |name: &str| -> String {
        summary
            .players
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.nation.clone())
            .unwrap_or_default()
    };
    assert_eq!(nation_of("Bukayo Ayoyinka Saka"), "England");
    assert_eq!(nation_of("Erling Braut Haaland"), "Norway");
    assert_eq!(nation_of("Kylian Mbappé Lottin"), "France");
    assert_eq!(nation_of("Florian Richard Wirtz"), "Germany");
    // From the 2026 naming pass over the small groups.
    assert_eq!(nation_of("Serhou Yadaly Guirassy"), "Guinea");
    assert_eq!(nation_of("Adam Marušić"), "Montenegro");

    // A save with the Welsh and English leagues loaded should be dominated by
    // those nations, which checks the mapping as a whole rather than one name.
    let mut by_nation = std::collections::HashMap::new();
    for p in &summary.players {
        if !p.nation.is_empty() {
            *by_nation.entry(p.nation.clone()).or_insert(0usize) += 1;
        }
    }
    let english = by_nation.get("England").copied().unwrap_or(0);
    assert!(english > 1_000, "expected many English players, got {english}");
    assert!(by_nation.contains_key("Wales"));
    assert!(by_nation.contains_key("Netherlands"));
}

#[test]
fn links_players_to_their_clubs() {
    let summary = save_or_skip!();

    let club_of = |name: &str| -> String {
        summary
            .players
            .iter()
            .find(|p| p.name == name)
            .map_or_else(|| panic!("{name} should be in the database"), |p| p.club.clone())
    };

    // The reference save's real squads, including 2025 summer transfers —
    // Walker left City for Burnley, so he is the negative case.
    assert_eq!(club_of("Erling Braut Haaland"), "Man City");
    assert_eq!(club_of("Bukayo Ayoyinka Saka"), "Arsenal");
    assert_eq!(club_of("Mohamed Salah Ghaly"), "Liverpool");
    assert_ne!(club_of("Kyle Andrew Walker"), "Man City");

    // People whose full name is exactly "forename surname" are stored with no
    // inline name at all; they were invisible to the parser before the string
    // table was decoded. Van Dijk is the canonical case.
    assert_eq!(club_of("Virgil van Dijk"), "Liverpool");
    assert_eq!(club_of("Florian Richard Wirtz"), "Liverpool");

    // A club filter needs squads to be squad-sized, not empty or thousands.
    // "Man City" covers two club entities — the men's squad of 33 and the
    // women's of 20, separate clubs in FM's database sharing a short name.
    //
    // Players only: the backroom binds to its club too, so the people at a
    // club are its squad *and* its staff, and folding the two together is what
    // would make a squad look three times the size it is.
    let city_squad = summary
        .players
        .iter()
        .filter(|p| p.club == "Man City" && p.is_player)
        .count();
    assert!(
        (15..=70).contains(&city_squad),
        "City's squads should be squad-sized, got {city_squad}"
    );
    let city_staff = summary
        .players
        .iter()
        .filter(|p| p.club == "Man City" && !p.is_player)
        .count();
    assert!(city_staff > 0, "City should employ backroom staff");

    // Most people in a database this size are unattached — retired legends,
    // newgens not yet placed, national staff. Attached people are a minority.
    let attached = summary.players.iter().filter(|p| !p.club.is_empty()).count();
    assert!(attached > 5_000, "expected thousands attached, got {attached}");
    assert!(attached < summary.players.len(), "not everyone has a club");
}

#[test]
fn a_shortlist_survives_export_and_reimport() {
    let Some(path) = save_path() else {
        eprintln!("skipped: no FM26 save on this machine");
        return;
    };

    let summary = commands::load_save(path.display().to_string(), &today(), |_| {}).unwrap();
    let picked: Vec<_> = summary.players.iter().take(25).cloned().collect();
    let names: Vec<String> = picked.iter().map(|p| p.name.clone()).collect();

    let out = std::env::temp_dir().join("gilet-roundtrip.csv");
    commands::export_csv(out.display().to_string(), picked).unwrap();

    let known: Vec<String> = summary.players.iter().map(|p| p.name.clone()).collect();
    let result = commands::import_csv(out.display().to_string(), known).unwrap();

    assert_eq!(result.matched, names, "every exported name should come back");
    assert!(result.unmatched.is_empty(), "unexpected unmatched: {:?}", result.unmatched);

    std::fs::remove_file(&out).ok();
}

#[test]
fn import_reports_names_that_are_not_in_the_save() {
    let out = std::env::temp_dir().join("gilet-unmatched.csv");
    std::fs::write(&out, "name\nErling Braut Haaland\nSomebody Who Does Not Exist\n").unwrap();

    let known = vec!["Erling Braut Haaland".to_owned()];
    let result = commands::import_csv(out.display().to_string(), known).unwrap();

    assert_eq!(result.matched, vec!["Erling Braut Haaland"]);
    assert_eq!(result.unmatched, vec!["Somebody Who Does Not Exist"]);

    std::fs::remove_file(&out).ok();
}

#[test]
fn opening_something_that_is_not_a_save_is_an_error_not_a_panic() {
    let out = std::env::temp_dir().join("gilet-not-a-save.fm");
    std::fs::write(&out, b"this is not a Football Manager save file at all").unwrap();

    let err = commands::load_save(out.display().to_string(), &today(), |_| {}).unwrap_err();
    assert!(err.to_string().contains("could not parse save"), "got: {err}");

    std::fs::remove_file(&out).ok();
}

#[test]
fn nationality_covers_the_database_not_just_a_handful() {
    let summary = save_or_skip!();

    let nation_of = |name: &str| -> String {
        summary
            .players
            .iter()
            .find(|p| p.name == name)
            .map_or_else(|| panic!("{name} should be in the database"), |p| p.nation.clone())
    };
    // Identified from their national squads rather than from a stored name.
    assert_eq!(nation_of("Kevin De Bruyne"), "Belgium");
    assert_eq!(nation_of("Robert Lewandowski"), "Poland");
    assert_eq!(nation_of("Luka Modrić"), "Croatia");
    assert_eq!(nation_of("Heung-Min Son"), "South Korea");
    assert_eq!(nation_of("Khvicha Kvaratskhelia"), "Georgia");

    // The table should cover the overwhelming majority of players, not the
    // few nations that were named first.
    let players: Vec<_> = summary.players.iter().filter(|p| p.is_player).collect();
    let named = players.iter().filter(|p| !p.nation.is_empty()).count();
    let share = named as f64 / players.len() as f64;
    assert!(share > 0.9, "only {:.1}% of players have a named nation", share * 100.0);
}

#[test]
fn club_squad_strength_ranks_the_divisions_correctly() {
    let summary = save_or_skip!();

    let club = |name: &str| -> &commands::ClubRow {
        summary
            .clubs
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} should be in the database"))
    };

    // Squad averages stand in for a league level while competitions are
    // undecoded, so the ordering across divisions has to hold.
    let city = club("Manchester City");
    assert!((15..=60).contains(&city.squad_size), "squad size {}", city.squad_size);
    let city_ca = city.average_ability.expect("City should have an average");

    // A League Two side must sit well below a Premier League one.
    let lower = club("Accrington Stanley").average_ability.expect("Accrington average");
    assert!(city_ca > lower + 30, "City {city_ca} vs Accrington {lower}");

    // Potential is never below current, in the aggregate as for one player.
    for c in summary.clubs.iter().filter(|c| c.squad_size > 0) {
        let (Some(ca), Some(pa)) = (c.average_ability, c.average_potential) else {
            panic!("{} has a squad but no averages", c.name);
        };
        assert!(pa >= ca, "{}: average PA {pa} below CA {ca}", c.name);
    }
}

#[test]
fn club_rows_carry_an_age_and_a_wage_bill() {
    let summary = save_or_skip!();

    let club = |name: &str| -> &commands::ClubRow {
        summary
            .clubs
            .iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} should be in the database"))
    };

    // A first-team squad's mean age sits inside the range a squad can be: a
    // figure outside it would mean staff or undated people are being folded in.
    let city = club("Manchester City");
    let age = city.average_age.expect("City should have a mean age");
    assert!((18.0..=32.0).contains(&age), "City mean age {age}");

    // The wage bill is the sum of the wages that decoded, so it is only ever a
    // floor — but a Premier League club's floor still has to dwarf a League Two
    // club's, and every counted wage has to belong to a counted player.
    let city_bill = city.wage_bill.expect("City should have a wage bill");
    let lower = club("Accrington Stanley");
    let lower_bill = lower.wage_bill.expect("Accrington should have a wage bill");
    assert!(city_bill > lower_bill * 5, "City {city_bill} vs Accrington {lower_bill}");

    for c in summary.clubs.iter().filter(|c| c.squad_size > 0) {
        assert!(
            c.wages_known <= c.squad_size,
            "{}: {} wages known across {} squad players",
            c.name,
            c.wages_known,
            c.squad_size
        );
        // No wage decoded means no bill at all, never a bill of zero.
        assert_eq!(c.wage_bill.is_some(), c.wages_known > 0, "{}", c.name);
    }
}

#[test]
fn in_save_shortlists_carry_the_entity_ids_of_their_members() {
    let summary = save_or_skip!();

    for list in &summary.game_shortlists {
        assert_eq!(
            list.players.len(),
            list.player_eids.len(),
            "{:?}: {} names against {} ids",
            list.name,
            list.players.len(),
            list.player_eids.len()
        );
        // Each id has to be the id of the person named beside it: the filter
        // that shows one shortlist keys on the id, and a mismatch there would
        // quietly show the wrong squad.
        for (eid, name) in list.player_eids.iter().zip(&list.players) {
            let person = summary
                .players
                .iter()
                .find(|p| p.eid == Some(*eid))
                .unwrap_or_else(|| panic!("shortlist member {eid} should be a loaded person"));
            assert_eq!(&person.name, name);
        }
    }
}
