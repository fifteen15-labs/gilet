//! Breaks the club-less population down by what else the record carries, to
//! separate parser gaps from genuine free agents, and reports the gender
//! coverage: how many people have no verdict, and how the boundary's verdict
//! disagrees with squad membership.
//!
//! ```text
//! cargo run --release --example gapstats -- <save.fm>...
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");

        let name = std::path::Path::new(&path)
            .file_stem()
            .map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());

        let players: Vec<_> = save.people.iter().filter(|p| p.is_player()).collect();
        let clubless: Vec<_> = players.iter().filter(|p| p.club_eid.is_none()).collect();
        let with_contract = clubless.iter().filter(|p| p.contract_until.is_some()).count();
        let with_wage = clubless
            .iter()
            .filter(|p| p.wage.is_some_and(|w| w > 0))
            .count();
        let no_contract = clubless.len() - with_contract;
        let compact = clubless.iter().filter(|p| p.compact).count();
        let no_eid = clubless.iter().filter(|p| p.eid.is_none()).count();

        println!("{name}");
        println!(
            "  players {:>6}  clubless {:>6} ({:.1}%)",
            players.len(),
            clubless.len(),
            100.0 * clubless.len() as f64 / players.len() as f64
        );
        println!(
            "  clubless with contract {with_contract:>6}  (clubless wage>0 {with_wage:>5})  <- parser gap, employed somewhere"
        );
        println!("  clubless no contract   {no_contract:>6}  (compact {compact:>5}, no eid {no_eid:>5})");

        // Gender coverage over everyone, and players specifically.
        let all = save.people.len();
        let unknown = save.people.iter().filter(|p| p.female.is_none()).count();
        let p_unknown = players.iter().filter(|p| p.female.is_none()).count();
        let women = save
            .people
            .iter()
            .filter(|p| p.female == Some(true))
            .count();
        println!(
            "  gender: people {all}  unknown {unknown}  players unknown {p_unknown}  women {women}"
        );

        // Staff: clubless staff with a staff sheet (employed staff should carry a club).
        let staff: Vec<_> = save
            .people
            .iter()
            .filter(|p| !p.is_player() && !p.compact)
            .collect();
        let staff_clubless = staff.iter().filter(|p| p.club_eid.is_none()).count();
        println!("  staff (non-player, non-compact) {:>6}  clubless {:>6}", staff.len(), staff_clubless);
    }
}
