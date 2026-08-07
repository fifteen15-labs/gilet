//! Resolves club entity ids to names — the club-table counterpart of `whois`.
//!
//! ```text
//! cargo run --release --example clubwho -- <save.fm> 19265 4496 4783
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: clubwho <save.fm> <club eid>...");
        std::process::exit(2);
    };
    let eids: Vec<u32> = args.filter_map(|a| a.parse().ok()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    for eid in eids {
        let found = save.clubs.iter().find(|c| c.eid == Some(eid));
        match found {
            Some(c) => println!(
                "{eid}  {} / {}  club_id {} nation {} uid {:?}",
                c.name, c.short_name, c.club_id, c.nation_id, c.uid
            ),
            None => println!("{eid}  (no club with this eid)"),
        }
    }
    Ok(())
}
