//! Prints every parsed club's `eid uid` pair, for cross-referencing club
//! references in Python spikes.
//!
//! ```text
//! cargo run --release --example clubuids -- <save.fm> > uids.txt
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: clubuids <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;
    for c in &save.clubs {
        if let (Some(eid), Some(uid)) = (c.eid, c.uid) {
            println!("{:#x} {eid} {uid} {} / {}", c.offset, c.name, c.short_name);
        }
    }
    Ok(())
}
