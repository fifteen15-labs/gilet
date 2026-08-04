//! Lists every club whose name matches, with identity and parsed squad
//! summary — untangling clubs that exist as several entities (first team,
//! youth teams, B side) so the squad walk attaches players to the right one.
//!
//! ```text
//! cargo run --release --example clubteams -- <save.fm> "Name"
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (Some(path), Some(who)) = (std::env::args().nth(1), std::env::args().nth(2)) else {
        eprintln!("usage: clubteams <save.fm> <name>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let by_eid: std::collections::HashMap<u32, &fm_save::Person> =
        save.people.iter().filter_map(|p| Some((p.eid?, p))).collect();

    for c in save
        .clubs
        .iter()
        .filter(|c| c.name.contains(&who) || c.short_name.contains(&who))
    {
        let squad = c.eid.and_then(|eid| save.squads.iter().find(|s| s.club_eid == eid));
        println!(
            "club @0x{:x} eid {:?} uid {:?} club_id {:?} \"{}\" / \"{}\"  squad: {}",
            c.offset,
            c.eid,
            c.uid,
            c.club_id,
            c.name,
            c.short_name,
            squad.map_or(0, |s| s.player_eids.len())
        );
        if let Some(s) = squad {
            let mut names: Vec<String> = Vec::new();
            for e in s.player_eids.iter().take(6) {
                if let Some(p) = by_eid.get(e) {
                    let age = save
                        .game_date
                        .map_or(0, |t| p.date_of_birth.age_on(t));
                    names.push(format!("{} ({age})", p.full_name));
                }
            }
            let stubs = s
                .player_eids
                .iter()
                .filter(|e| save.stubs.iter().any(|st| st.eid == **e))
                .count();
            println!("    members: {}  [+{stubs} stub member(s)]", names.join(", "));
        }
    }
    Ok(())
}
