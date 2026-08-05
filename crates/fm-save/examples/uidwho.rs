//! Resolves person uids against a save — the uid-keyed sibling of `whois`.
//!
//! ```text
//! cargo run --release --example uidwho -- <save.fm> <uid>...
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: uidwho <save.fm> <uid>...");
        std::process::exit(2);
    };
    let uids: Vec<u32> = args.filter_map(|a| a.parse().ok()).collect();
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;
    for uid in uids {
        match save.people.iter().find(|p| p.uid == Some(uid)) {
            Some(p) => println!("{uid}  {}", p.full_name),
            None => println!("{uid}  NOT FOUND"),
        }
    }
    Ok(())
}
