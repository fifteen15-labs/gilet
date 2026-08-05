//! Hunts the save's current date by frequency: every `(day_of_year, year)`
//! pair in every frame, masked and unmasked, tallied so the date the save
//! repeats most often stands out from the ones it mentions once.
//!
//! ```text
//! cargo run --release --example datehunt -- <save.fm> [year_lo] [year_hi]
//! ```

use std::collections::HashMap;

/// A `(day of year, year)` pair as the scan reads it.
type Stamp = (u16, u16);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: datehunt <save.fm> [year_lo] [year_hi]");
        std::process::exit(2);
    };
    let lo: u16 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2025);
    let hi: u16 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2045);

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let members = frames
        .last()
        .and_then(|f| fm_save::manifest::read_manifest(&f.data))
        .unwrap_or_default();
    let name_of = |i: usize| -> String {
        members.get(i).map_or_else(|| format!("frame {i}"), |m| m.name.clone())
    };

    // Whole-save tally: which date does this file repeat?
    let mut tally: HashMap<Stamp, usize> = HashMap::new();
    // Per-frame tally, for the small frames where a single stamp is findable.
    let mut per_frame: HashMap<(usize, Stamp), usize> = HashMap::new();

    for (i, frame) in frames.iter().enumerate() {
        let d = &frame.data;
        let mut at = 0usize;
        while at + 4 <= d.len() {
            let raw = u16::from_le_bytes([
                d.get(at).copied().unwrap_or(0),
                d.get(at + 1).copied().unwrap_or(0),
            ]);
            let year = u16::from_le_bytes([
                d.get(at + 2).copied().unwrap_or(0),
                d.get(at + 3).copied().unwrap_or(0),
            ]);
            if (lo..=hi).contains(&year) && raw & 0x01FF <= 366 && raw & 0x01FF > 0 {
                *tally.entry((raw & 0x01FF, year)).or_default() += 1;
                *per_frame.entry((i, (raw & 0x01FF, year))).or_default() += 1;
            }
            at += 2;
        }
    }

    let mut top: Vec<(Stamp, usize)> = tally.into_iter().collect();
    top.sort_unstable_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("== most repeated (doy, year) across the whole save ==");
    for ((doy, year), n) in top.iter().take(20) {
        let date = fm_save::Date::from_day_of_year(*doy, *year);
        println!("  doy {doy:3} year {year}  x{n:<8}  {date:?}");
    }

    println!("\n== per-frame leader, small frames only ==");
    let mut by_frame: HashMap<usize, Vec<(Stamp, usize)>> = HashMap::new();
    for ((i, key), n) in per_frame {
        by_frame.entry(i).or_default().push((key, n));
    }
    let mut ids: Vec<usize> = by_frame.keys().copied().collect();
    ids.sort_unstable();
    for i in ids {
        let Some(frame) = frames.get(i) else { continue };
        if frame.data.len() > 4_000_000 {
            continue;
        }
        let Some(list) = by_frame.get_mut(&i) else { continue };
        list.sort_unstable_by_key(|(_, n)| std::cmp::Reverse(*n));
        let shown: Vec<String> = list
            .iter()
            .take(3)
            .map(|((doy, year), n)| {
                fm_save::Date::from_day_of_year(*doy, *year).map_or_else(
                    || format!("doy{doy}/{year} x{n}"),
                    |d| format!("{:04}-{:02}-{:02} x{n}", d.year, d.month, d.day),
                )
            })
            .collect();
        println!("  {:<34} {:>10}B  {}", name_of(i), frame.data.len(), shown.join("  "));
    }
    Ok(())
}
