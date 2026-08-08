//! Prints the human manager, their club and their active tactic — the
//! humans.dat and tactics_man.dat reads together.
//!
//! ```text
//! cargo run --release --example mytactic -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: mytactic <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    match save.human_eid {
        Some(eid) => {
            let human = save.people.iter().find(|p| p.eid == Some(eid));
            let name = human.map_or("<unresolved>", |p| p.full_name.as_str());
            let club = human
                .and_then(|p| p.club_eid)
                .and_then(|ce| save.clubs.iter().find(|c| c.eid == Some(ce)))
                .map_or("<no club>", |c| c.name.as_str());
            println!("human: {name} (eid {eid}) at {club}");
        }
        None => println!("human: none read"),
    }

    match &save.tactic {
        Some(t) => {
            println!("tactic: {:?} style {:?}", t.name, t.style);
            println!("shape: {}", t.positions.join(" "));
            for (i, eid) in t.starters.iter().enumerate() {
                let who = save
                    .people
                    .iter()
                    .find(|p| p.eid == Some(*eid))
                    .map_or("<unresolved>", |p| p.full_name.as_str());
                let pos = t.positions.get(i).map_or("?", |p| p.as_str());
                println!("  {pos:>4} {who}");
            }
            println!("bench: {} named", t.bench.len());
        }
        None => println!("tactic: none read"),
    }
    Ok(())
}
