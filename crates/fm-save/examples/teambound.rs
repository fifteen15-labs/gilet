//! Reports what the B/youth squad pass binds on a save: counts, and named
//! samples of players whose club link came from a team squad rather than a
//! first-team list — for picking real-save test anchors.
//!
//! ```text
//! cargo run --release --example teambound -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: teambound <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let first_team: std::collections::HashSet<u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().copied())
        .collect();
    let clubs: std::collections::HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();

    println!("team squads: {}", save.team_squads.len());
    let mut bound = 0usize;
    let mut shown = 0usize;
    for p in &save.people {
        let (Some(eid), Some(club)) = (p.eid, p.club_eid) else {
            continue;
        };
        if first_team.contains(&eid) || p.ability.is_none() {
            continue;
        }
        bound += 1;
        if shown < 15 {
            shown += 1;
            println!(
                "  {} -> {}  (eid {eid})",
                p.full_name,
                clubs.get(&club).copied().unwrap_or("?")
            );
        }
    }
    println!("players bound outside first-team lists: {bound}");
    Ok(())
}
