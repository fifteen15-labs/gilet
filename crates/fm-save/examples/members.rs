//! Lists the save's member manifest — every named member with its frame
//! index and sizes — for spotting which subsystem might hold a structure
//! (the way `scout_man.dat` gave up the in-game shortlists).
//!
//! ```text
//! cargo run --release --example members -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: members <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(last) = frames.last() else { return Ok(()) };
    let Some(members) = fm_save::manifest::read_manifest(&last.data) else {
        eprintln!("no manifest");
        return Ok(());
    };
    println!("{} members", members.len());
    for (i, m) in members.iter().enumerate() {
        let size = frames.get(i).map_or(0, |f| f.data.len());
        println!("{i:4}  {:>12}  {}", size, m.name);
    }
    Ok(())
}
