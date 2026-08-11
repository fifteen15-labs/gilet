//! Prints how many people the representative rows mark as being in a
//! national setup, the per-row population, and whether named probe players
//! carry the mark — to check the signal against known internationals.
//!
//! ```text
//! cargo run --release --example natwho -- <save.fm> Haaland "van Dijk"
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: natwho <save.fm> [player name]...");
        std::process::exit(2);
    };
    let names: Vec<String> = args.collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let rows = fm_save::squad::scan_representative_squads(&main.data);
    let members: usize = rows.iter().map(|r| r.player_eids.len()).sum();
    println!("{} representative rows, {} members", rows.len(), members);
    let men = rows.iter().filter(|r| r.team_eid < 250).count();
    println!("  team eids < 250 (men's sides): {men}");
    println!(
        "  team eids >= 261 (women's):    {}",
        rows.iter().filter(|r| r.team_eid >= 261).count()
    );

    let save = fm_save::Save::parse(&bytes)?;
    let marked = save.people.iter().filter(|p| p.in_national_squad).count();
    println!("people marked in a national squad: {marked}\n");

    for p in &save.people {
        if names.iter().any(|n| p.full_name.contains(n.as_str())) {
            println!(
                "{:35} eid {:?} in_national_squad {}",
                p.full_name, p.eid, p.in_national_squad
            );
        }
    }
    Ok(())
}
