//! Hunts the club reputation field. Dump mode: prints the raw bytes around
//! each named club's record (anchor: the name-length field, entity head at
//! −39 per `SAVE_FORMAT.md` §4), plus the distance to the next club record,
//! so candidate u16/u32 reputation fields can be read off against published
//! FM26 values.
//!
//! ```text
//! cargo run --release --example clubrep -- <save.fm> "Real Madrid" "Manchester City" ...
//! ```

// Research spike, not shipped code.
#![allow(clippy::indexing_slicing, clippy::too_many_lines, clippy::cast_possible_truncation)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: clubrep <save.fm> <club name>...");
        std::process::exit(2);
    };
    let names: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let save = fm_save::Save::parse(&bytes)?;
    // Clubs are scanned from game_db.dat — find it the same way lib.rs does:
    // the frame whose offset range contains the first club's offset is not
    // recoverable here, so take the largest frame and check a known club's
    // name bytes actually sit at its offset.
    let main = frames
        .iter()
        .max_by_key(|f| f.data.len())
        .ok_or("no frames")?;
    let data = &main.data;

    // Sanity: club offsets index into this frame and the name matches.
    if let Some(c) = save.clubs.iter().find(|c| c.eid.is_some()) {
        let at = c.offset + 4;
        let ok = data.get(at..at + c.name.len()).map(|s| s == c.name.as_bytes());
        println!("frame check on {:?}: name bytes at offset match = {ok:?}", c.name);
    }

    let mut offsets: Vec<usize> = save.clubs.iter().map(|c| c.offset).collect();
    offsets.sort_unstable();

    for want in &names {
        let mut matches: Vec<_> = save
            .clubs
            .iter()
            .filter(|c| c.name.to_lowercase() == *want || c.short_name.to_lowercase() == *want)
            .collect();
        if matches.is_empty() {
            matches = save
                .clubs
                .iter()
                .filter(|c| c.name.to_lowercase().contains(want.as_str()))
                .collect();
        }
        if matches.is_empty() {
            println!("\n=== {want}: NOT FOUND ===");
            continue;
        }
        for c in matches {
            let next = offsets
                .iter()
                .find(|&&o| o > c.offset)
                .copied()
                .unwrap_or(data.len());
            println!(
                "\n=== {} / {} — eid {:?} uid {:?} nation {} club_id {} offset 0x{:x}, next club at +{} ===",
                c.name,
                c.short_name,
                c.eid,
                c.uid,
                c.nation_id,
                c.club_id,
                c.offset,
                next - c.offset
            );
            // Anchor everything on the name-length field (c.offset).
            let tail_at = c.offset + 4 + c.name.len() + 4 + c.short_name.len();
            println!("record tail (after short name) at +{}", tail_at - c.offset);
            let from = c.offset.saturating_sub(64);
            let to = (tail_at + 560).min(data.len());
            let mut at = from;
            while at < to {
                let row: Vec<String> = data[at..(at + 16).min(to)]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                let ascii: String = data[at..(at + 16).min(to)]
                    .iter()
                    .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                    .collect();
                let rel = at as i64 - c.offset as i64;
                println!("{rel:+6}  {}  {ascii}", row.join(" "));
                at += 16;
            }
        }
    }
    Ok(())
}
