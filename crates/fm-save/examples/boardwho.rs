//! Prints each named club's boardroom as bound people — the director of
//! football seat and the board members, resolved to names — plus population
//! counts, to pick real-save test anchors for `scan_boardrooms`.
//!
//! ```text
//! cargo run --release --example boardwho -- <save.fm> "Man City" Liverpool Lens
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: boardwho <save.fm> [club name]...");
        std::process::exit(2);
    };
    let names: Vec<String> = args.collect();

    let save = fm_save::Save::parse(&std::fs::read(&path)?)?;

    let mut dofs = 0usize;
    let mut boarders = 0usize;
    for p in &save.people {
        match p.staff_role {
            Some(fm_save::backroom::Role::DirectorOfFootball) => dofs += 1,
            Some(fm_save::backroom::Role::Board) => boarders += 1,
            _ => {}
        }
    }
    println!("bound: {dofs} directors of football, {boarders} board members\n");

    for c in &save.clubs {
        if !names.iter().any(|n| c.name.contains(n.as_str())) {
            continue;
        }
        let Some(club_eid) = c.eid else { continue };
        println!("{} (eid {club_eid}):", c.name);
        for p in &save.people {
            if p.club_eid != Some(club_eid) {
                continue;
            }
            match p.staff_role {
                Some(r @ (fm_save::backroom::Role::DirectorOfFootball | fm_save::backroom::Role::Board)) => {
                    println!("  {:22} {} (eid {:?})", r.name(), p.full_name, p.eid);
                }
                _ => {}
            }
        }
    }
    Ok(())
}
