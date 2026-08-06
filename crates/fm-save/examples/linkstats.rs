//! One line of linkage numbers per save, for before/after comparisons when a
//! scan is loosened or tightened: how many clubs parse, how many carry an
//! entity id, how many own a squad, and how many people end up linked to a
//! club. Also reports club entity ids claimed by more than one record, which
//! is what a false-positive club would look like from the squad table's side.
//!
//! ```text
//! cargo run --release --example linkstats -- <save.fm>...
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::collections::{HashMap, HashSet};

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");

        let with_eid = save.clubs.iter().filter(|c| c.eid.is_some()).count();
        let mut by_eid: HashMap<u32, usize> = HashMap::new();
        for club in save.clubs.iter().filter_map(|c| c.eid) {
            *by_eid.entry(club).or_default() += 1;
        }
        let shared = by_eid.values().filter(|&&n| n > 1).count();

        let squad_clubs: HashSet<u32> = save.squads.iter().map(|s| s.club_eid).collect();
        let players = save.people.iter().filter(|p| p.is_player()).count();
        let linked = save.people.iter().filter(|p| p.club_eid.is_some()).count();
        let linked_players = save
            .people
            .iter()
            .filter(|p| p.is_player() && p.club_eid.is_some())
            .count();

        let name = std::path::Path::new(&path)
            .file_stem()
            .map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
        println!(
            "{name:<20} clubs {:>6} ({with_eid} with eid, {shared} eids shared)  squads {:>5} \
             at {:>5} clubs  people {:>6}  linked {:>6}  players {:>6}  linked players {:>6}",
            save.clubs.len(),
            save.squads.len(),
            squad_clubs.len(),
            save.people.len(),
            linked,
            players,
            linked_players,
        );
    }
}
