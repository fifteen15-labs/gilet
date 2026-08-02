//! Dumps the id→value row region of a staff member's record, hunting the
//! coaching rows described in `SAVE_FORMAT.md` §3 (`OPEN_PROBLEMS` §3b). The
//! published FM 26 values for the same database version are the key that
//! names the row ids once the rows can be read.
//!
//! ```text
//! cargo run --release --example staffrows -- <save.fm> "Name"
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (Some(path), Some(who)) = (std::env::args().nth(1), std::env::args().nth(2)) else {
        eprintln!("usage: staffrows <save.fm> <name>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let save = fm_save::Save::parse(&bytes)?;

    let Some(p) = save.people.iter().find(|p| p.full_name.contains(&who)) else {
        println!("{who}: not in save");
        return Ok(());
    };
    let mut offsets: Vec<usize> = save.people.iter().map(|q| q.offset).collect();
    offsets.sort_unstable();
    let end = offsets
        .iter()
        .find(|&&o| o > p.offset)
        .copied()
        .unwrap_or(main.data.len())
        .min(p.offset + 6000);
    println!(
        "{} record 0x{:x}..+{}  eid {:?} ability {}",
        p.full_name,
        p.offset,
        end - p.offset,
        p.eid,
        p.ability.is_some()
    );

    // Hex dump the whole span, 32 bytes per row, with offsets relative to
    // the record prefix.
    let mut at = p.offset;
    while at < end {
        let row: Vec<String> = main
            .data
            .iter()
            .skip(at)
            .take(32)
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("  +{:04}: {}", at - p.offset, row.join(" "));
        at += 32;
    }
    Ok(())
}
