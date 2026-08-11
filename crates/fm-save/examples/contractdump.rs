//! Dumps the contract block for named players: the raw window from before
//! the expiry run to just past the wage anchor, plus every aligned-or-not
//! u32 that reads money-shaped, labelled by its offset from the anchor.
//! For hunting the unparsed fields — bonuses, clauses, the signing dates
//! (`SAVE_FORMAT.md` §6e).
//!
//! ```text
//! cargo run --release --example contractdump -- <save.fm> Haaland Pedri
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: contractdump <save.fm> <player name>...");
        std::process::exit(2);
    };
    let names: Vec<String> = args.collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    for p in &save.people {
        if !names.iter().any(|n| p.full_name.contains(n.as_str())) {
            continue;
        }
        let Some(eid) = p.eid else { continue };
        println!(
            "\n=== {} (eid {eid}, wage {:?}, until {:?}) record at {:#x}",
            p.full_name,
            p.wage,
            p.contract_until.map(|d| (d.year, d.month, d.day)),
            p.offset
        );

        // Find the anchor the parser found: walk back like bind_contracts.
        let lo = p.offset.saturating_sub(600);
        let mut anchor = None;
        let mut at = p.offset;
        while at > lo + 4 {
            at -= 1;
            if read_u32(data, at) == Some(eid)
                && data.get(at + 8..at + 12) == Some(&[0u8; 4][..])
                && data.get(at + 16) == Some(&0x01)
                && data.get(at + 18) == Some(&0x00)
                && data.get(at + 19..at + 23) == Some(&[0xFF; 4][..])
            {
                anchor = Some(at);
                break;
            }
        }
        let Some(anchor) = anchor else {
            println!("  no contract anchor");
            continue;
        };
        println!("  anchor at {anchor:#x}");

        // The block window: 450 back through 60 past the anchor.
        let from = anchor.saturating_sub(450);
        let to = (anchor + 60).min(data.len());
        for row_start in (from..to).step_by(16) {
            let row = data.get(row_start..(row_start + 16).min(to)).unwrap_or(&[]);
            let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
            let (sign, mag) = if row_start >= anchor {
                ('+', row_start - anchor)
            } else {
                ('-', anchor - row_start)
            };
            println!("  {sign}{mag:4}  {}", hex.join(" "));
        }

        // Money-shaped u32s, labelled relative to the anchor. Two tiers:
        // bonus-sized round money, and clause-sized large round money.
        let rel = |at: usize| {
            if at >= anchor {
                format!("+{}", at - anchor)
            } else {
                format!("-{}", anchor - at)
            }
        };
        println!("  bonus-shaped (1K..1M, multiple of 100):");
        for at in from..to.saturating_sub(4) {
            let Some(v) = read_u32(data, at) else { continue };
            if (1_000..1_000_000).contains(&v) && v % 100 == 0 {
                println!("    {:>6}: {v}", rel(at));
            }
        }
        println!("  clause-shaped (1M..4B, multiple of 10K):");
        for at in from..to.saturating_sub(4) {
            let Some(v) = read_u32(data, at) else { continue };
            if v >= 1_000_000 && v % 10_000 == 0 {
                println!("    {:>6}: {v}", rel(at));
            }
        }
    }
    Ok(())
}
