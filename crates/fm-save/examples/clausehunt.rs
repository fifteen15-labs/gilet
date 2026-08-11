//! Hunts a known release-clause value near a player's record: scans ±window
//! bytes around the record offset for u32s inside a target range, at every
//! byte offset — for matching a published clause figure whose save-currency
//! conversion is only known to ~1%.
//!
//! ```text
//! cargo run --release --example clausehunt -- <save.fm> "Pedro González" 700000000 800000000 8192
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(name)) = (args.next(), args.next()) else {
        eprintln!("usage: clausehunt <save.fm> <name> <min> <max> [window]");
        std::process::exit(2);
    };
    let min: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(500_000_000);
    let max: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1_100_000_000);
    let window: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(8192);

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(frame) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &frame.data;
    let save = fm_save::Save::parse(&bytes)?;

    for p in &save.people {
        if !p.full_name.contains(name.as_str()) {
            continue;
        }
        println!("=== {} record at {:#x}", p.full_name, p.offset);
        let from = p.offset.saturating_sub(window);
        let to = (p.offset + window).min(data.len());
        for at in from..to.saturating_sub(4) {
            let Some(v) = read_u32(data, at) else { continue };
            for (scale, label) in
                [(1u64, "x1"), (10, "x10"), (100, "x100"), (1000, "x1000"), (10_000, "x10000")]
            {
                let scaled = u64::from(v) * scale;
                if scaled >= u64::from(min) && scaled <= u64::from(max) {
                    let (sign, mag) = if at >= p.offset {
                        ('+', at - p.offset)
                    } else {
                        ('-', p.offset - at)
                    };
                    println!("  {sign}{mag:5}: {v} ({label} -> {scaled})");
                }
            }
        }
    }
    Ok(())
}
