//! Finds every occurrence of a little-endian u32 in the main frame, printing
//! each with a hex window and the nearest parsed structure (person, club,
//! squad) before it — for tracing what references an entity id.
//!
//! ```text
//! cargo run --release --example u32where -- <save.fm> 250676 [max]
//! ```

use std::fmt::Write as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(val_s)) = (args.next(), args.next()) else {
        eprintln!("usage: u32where <save.fm> <value> [max hits shown]");
        std::process::exit(2);
    };
    let value: u32 = if let Some(hex) = val_s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16)?
    } else {
        val_s.parse()?
    };
    let max_show: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(40);

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut marks: Vec<(usize, String)> = Vec::new();
    for p in &save.people {
        marks.push((p.offset, format!("person {}", p.full_name)));
    }
    for c in &save.clubs {
        marks.push((c.offset, format!("club {}", c.short_name)));
    }
    for s in &save.squads {
        marks.push((s.offset, format!("squad of club_eid {}", s.club_eid)));
    }
    marks.sort_by_key(|(o, _)| *o);

    let needle = value.to_le_bytes();
    let mut count = 0usize;
    for at in 0..data.len().saturating_sub(4) {
        if data.get(at..at + 4) != Some(&needle[..]) {
            continue;
        }
        count += 1;
        if count > max_show {
            continue;
        }
        let i = marks.partition_point(|(o, _)| *o <= at);
        let near = i
            .checked_sub(1)
            .and_then(|j| marks.get(j))
            .map_or(String::new(), |(o, w)| format!("{w} @0x{o:x} ({} back)", at - o));
        let lo = at.saturating_sub(20);
        let hi = (at + 24).min(data.len());
        let hex = data
            .get(lo..hi)
            .unwrap_or(&[])
            .iter()
            .fold(String::new(), |mut s, b| {
                let _ = write!(s, "{b:02x} ");
                s
            });
        println!("0x{at:x}  {near}");
        println!("    {hex}");
    }
    println!("\n{count} occurrences of {value} (0x{value:x})");
    Ok(())
}
