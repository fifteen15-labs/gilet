//! Resolves person entity ids against a save. Development aid for checking
//! decoded structures that reference people by eid:
//!
//! ```text
//! cargo run --release --example whois -- <save.fm> 13264 20989 21563
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: whois <save.fm> <eid>...");
        std::process::exit(2);
    };
    let eids: Vec<u32> = args.filter_map(|a| a.parse().ok()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    for eid in eids {
        match save.people.iter().find(|p| p.eid == Some(eid)) {
            Some(p) => println!("{eid}  {}", p.full_name),
            None => println!("{eid}  NOT FOUND"),
        }
    }
    Ok(())
}
