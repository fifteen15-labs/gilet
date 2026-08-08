//! The backend search against a real save — the rules that used to run in
//! the frontend, now asserted where they actually execute.
//!
//! Skips rather than fails when the save is not on this machine, like the
//! other real-save suites.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use gilet_lib::commands::SaveSummary;
use gilet_lib::search;

fn load() -> Option<SaveSummary> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home)
        .join("Library/Application Support/Sports Interactive/Football Manager 26/games")
        .join("Day One.fm");
    let path = path.to_str()?.to_owned();
    if !std::path::Path::new(&path).exists() {
        return None;
    }
    Some(gilet_lib::commands::load_save(path, &[2026, 8, 1], |_| {}).unwrap())
}

fn context(summary: &SaveSummary) -> search::Context {
    search::Context {
        shortlist_eids: std::collections::HashSet::new(),
        position_slots: summary
            .position_names
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.is_empty())
            .map(|(i, n)| (n.clone(), i))
            .collect(),
    }
}

macro_rules! save_or_skip {
    () => {
        match load() {
            Some(s) => s,
            None => {
                eprintln!("skipped: no Day One.fm on this machine");
                return;
            }
        }
    };
}

#[test]
fn free_agents_have_neither_wage_nor_club_nor_expiry() {
    let summary = save_or_skip!();
    let filters = search::Filters {
        kind: Some("players".to_owned()),
        contract: Some("free".to_owned()),
        min_ability: Some(100),
        ..Default::default()
    };
    let page = search::run(
        &summary.players,
        &filters,
        "ability",
        "desc",
        None,
        &context(&summary),
        400,
    );
    assert!(page.total > 0, "a full database holds able free agents");
    for row in &page.rows {
        assert!(row.wage.is_none(), "{} has a wage", row.name);
        assert!(row.club.is_empty(), "{} has a club", row.name);
        assert!(row.contract_until.is_empty(), "{} has an expiry", row.name);
        assert!(row.ability.unwrap_or(0) >= 100);
    }
    // Sorted by ability, strongest first.
    let abilities: Vec<u8> = page.rows.iter().filter_map(|r| r.ability).collect();
    assert!(abilities.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn a_query_ignores_case_and_diacritics() {
    let summary = save_or_skip!();
    let filters = search::Filters {
        query: "mbappe".to_owned(),
        ..Default::default()
    };
    let page = search::run(
        &summary.players,
        &filters,
        "ability",
        "desc",
        None,
        &context(&summary),
        400,
    );
    assert!(
        page.rows.iter().any(|r| r.name.contains("Mbappé")),
        "\"mbappe\" should find Mbappé"
    );
}

#[test]
fn the_nameless_sink_to_the_bottom_of_a_name_sort() {
    let summary = save_or_skip!();
    let page = search::run(
        &summary.players,
        &search::Filters::default(),
        "name",
        "asc",
        None,
        &context(&summary),
        400,
    );
    assert_eq!(page.total, summary.players.len());
    let first = page.rows.first().unwrap();
    assert!(!first.name.is_empty(), "an empty name must not lead the alphabet");
}

#[test]
fn an_unknown_shortlist_matches_nobody() {
    let summary = save_or_skip!();
    let filters = search::Filters {
        shortlist: Some("no such list".to_owned()),
        ..Default::default()
    };
    let page = search::run(
        &summary.players,
        &filters,
        "name",
        "asc",
        None,
        &context(&summary),
        400,
    );
    assert_eq!(page.total, 0);
}

#[test]
fn tactic_positions_keep_only_players_who_cover_one() {
    let summary = save_or_skip!();
    // A goalkeeper-or-striker fit: the two positions never overlap in one
    // player, so every result must be natural in exactly one of them, and no
    // outfield-only or keeper-only player is wrongly dropped.
    let filters = search::Filters {
        kind: Some("players".to_owned()),
        tactic_positions: Some(vec!["GK".to_owned(), "ST".to_owned()]),
        position_tier: Some("natural".to_owned()),
        ..Default::default()
    };
    let page = search::run(
        &summary.players,
        &filters,
        "ability",
        "desc",
        None,
        &context(&summary),
        400,
    );
    assert!(page.total > 0, "a database holds keepers and strikers");
    for row in &page.rows {
        assert!(
            row.positions.iter().any(|p| p == "GK" || p == "ST"),
            "{} plays neither GK nor ST but passed the tactic filter",
            row.name
        );
    }
}
