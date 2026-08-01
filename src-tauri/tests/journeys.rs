//! End-to-end checks against a real save.
//!
//! These exercise the commands the UI calls, so the open → filter → export →
//! import path is covered even though the UI itself is driven by hand. They
//! skip rather than fail when no save is present, so the suite still passes on
//! a machine without Football Manager installed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gilet_lib::commands;

fn save_path() -> Option<std::path::PathBuf> {
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
    Some(commands::open_save(path.display().to_string(), today()).unwrap())
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
fn a_shortlist_survives_export_and_reimport() {
    let Some(path) = save_path() else {
        eprintln!("skipped: no FM26 save on this machine");
        return;
    };

    let summary = commands::open_save(path.display().to_string(), today()).unwrap();
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

    let err = commands::open_save(out.display().to_string(), today()).unwrap_err();
    assert!(err.to_string().contains("could not parse save"), "got: {err}");

    std::fs::remove_file(&out).ok();
}
