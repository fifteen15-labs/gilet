//! End-to-end checks against a real save.
//!
//! These exercise the commands the UI calls, so the open → filter → export →
//! import path is covered even though the UI itself is driven by hand. They
//! skip rather than fail when no save is present, so the suite still passes on
//! a machine without Football Manager installed.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use anorak_lib::commands;

fn save_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let p = std::path::PathBuf::from(home)
        .join("Library/Application Support/Sports Interactive/Football Manager 26/games/Career.fm");
    p.exists().then_some(p)
}

fn today() -> Vec<u16> {
    vec![2026, 8, 1]
}

#[test]
fn opens_a_real_save_and_finds_people_and_clubs() {
    let Some(path) = save_path() else {
        eprintln!("skipped: no FM26 save on this machine");
        return;
    };

    let summary = commands::open_save(path.display().to_string(), today()).unwrap();

    assert!(summary.players.len() > 5_000, "expected a populated database, got {}", summary.players.len());
    assert!(summary.clubs.len() > 1_000, "expected clubs, got {}", summary.clubs.len());
    assert!(summary.frames > 1_000);

    // Known players, with the dates of birth the format decoding was verified against.
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

    // Ability is deliberately absent until it is located in the format.
    assert!(haaland.ability.is_none());
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

    let out = std::env::temp_dir().join("anorak-roundtrip.csv");
    commands::export_csv(out.display().to_string(), picked).unwrap();

    let known: Vec<String> = summary.players.iter().map(|p| p.name.clone()).collect();
    let result = commands::import_csv(out.display().to_string(), known).unwrap();

    assert_eq!(result.matched, names, "every exported name should come back");
    assert!(result.unmatched.is_empty(), "unexpected unmatched: {:?}", result.unmatched);

    std::fs::remove_file(&out).ok();
}

#[test]
fn import_reports_names_that_are_not_in_the_save() {
    let out = std::env::temp_dir().join("anorak-unmatched.csv");
    std::fs::write(&out, "name\nErling Braut Haaland\nSomebody Who Does Not Exist\n").unwrap();

    let known = vec!["Erling Braut Haaland".to_owned()];
    let result = commands::import_csv(out.display().to_string(), known).unwrap();

    assert_eq!(result.matched, vec!["Erling Braut Haaland"]);
    assert_eq!(result.unmatched, vec!["Somebody Who Does Not Exist"]);

    std::fs::remove_file(&out).ok();
}

#[test]
fn opening_something_that_is_not_a_save_is_an_error_not_a_panic() {
    let out = std::env::temp_dir().join("anorak-not-a-save.fm");
    std::fs::write(&out, b"this is not a Football Manager save file at all").unwrap();

    let err = commands::open_save(out.display().to_string(), today()).unwrap_err();
    assert!(err.to_string().contains("could not parse save"), "got: {err}");

    std::fs::remove_file(&out).ok();
}
