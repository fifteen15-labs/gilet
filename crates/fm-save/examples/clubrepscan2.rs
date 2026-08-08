//! Second-stage club reputation hunt.
//!
//! 1. Detects, per ladder club, every "same u16 repeated at stride 25" run in
//!    the record body (no value prior) — the structure the first scan found
//!    carrying ~display*100 for four of six clubs.
//! 2. Scans deltas anchored on the *end* of the body (the next club record)
//!    for u16/u32 values that match round(v/100) == published display for
//!    every club at a common delta.
//!
//! ```text
//! cargo run --release --example clubrepscan2 -- <save.fm>
//! ```

// Research spike, not shipped code.
#![allow(
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

/// (label, club eid, fminside FM26 reputation on the /100 display).
const LADDER: &[(&str, u32, u32)] = &[
    ("Man City", 369, 92),
    ("R. Madrid", 1079, 91),
    ("Liverpool", 366, 89),
    ("Millwall", 376, 57),
    ("Grimsby", 348, 44),
    ("Braintree", 2527, 31),
];

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

struct Body {
    offset: usize,
    tail: usize,
    end: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: clubrepscan2 <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let main = frames.iter().max_by_key(|f| f.data.len()).ok_or("no frames")?;
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut offsets: Vec<usize> = save.clubs.iter().map(|c| c.offset).collect();
    offsets.sort_unstable();

    let mut bodies: Vec<(String, Body, u32)> = Vec::new();
    for &(label, eid, rep) in LADDER {
        let Some(c) = save.clubs.iter().find(|c| c.eid == Some(eid)) else {
            continue;
        };
        let tail = c.offset + 4 + c.name.len() + 4 + c.short_name.len();
        let end = offsets.iter().find(|&&o| o > c.offset).copied().unwrap_or(data.len());
        bodies.push((label.to_owned(), Body { offset: c.offset, tail, end }, rep));
    }

    // --- 1. stride-25 runs, no value prior, all alignments ---
    for (label, b, rep) in &bodies {
        println!("\n=== {label} (display {rep}) body {} bytes ===", b.end - b.offset);
        for at in b.tail..b.end.saturating_sub(2) {
            let v = u16_at(data, at).unwrap_or(0);
            if v < 150 {
                continue;
            }
            // Only run starts: the same value 25 back means we are inside one.
            if at >= b.tail + 25 && u16_at(data, at - 25) == Some(v) {
                continue;
            }
            let mut n = 1;
            while at + n * 25 + 2 <= b.end && u16_at(data, at + n * 25) == Some(v) {
                n += 1;
            }
            if n >= 4 {
                println!(
                    "  run: u16 {v} x{n} at tail+{} (round/100 = {})",
                    at - b.tail,
                    (u32::from(v) + 50) / 100
                );
                // Row shape: dump three 25-byte rows from 4 before the hit.
                for row in 0..3 {
                    let from = (at + row * 25).saturating_sub(4);
                    let hex: Vec<String> = data[from..(from + 25).min(data.len())]
                        .iter()
                        .map(|x| format!("{x:02x}"))
                        .collect();
                    println!("    row{row}: {}", hex.join(" "));
                }
            }
        }
    }

    // --- 2. end-anchored scan ---
    println!("\n--- end-anchored scan (delta back from next club record) ---");
    let min_span = bodies.iter().map(|(_, b, _)| b.end - b.tail).min().unwrap_or(0);
    for back in 2..min_span {
        let v16: Vec<u64> = bodies
            .iter()
            .filter_map(|(_, b, _)| u16_at(data, b.end - back).map(u64::from))
            .collect();
        let v32: Vec<u64> = if back >= 4 {
            bodies
                .iter()
                .filter_map(|(_, b, _)| u32_at(data, b.end - back).map(u64::from))
                .collect()
        } else {
            Vec::new()
        };
        for (kind, vals) in [("u16", &v16), ("u32", &v32)] {
            if vals.len() != bodies.len() {
                continue;
            }
            let exact = vals
                .iter()
                .zip(bodies.iter())
                .all(|(&v, (_, _, r))| v > 0 && (v + 50) / 100 == u64::from(*r));
            if exact {
                println!("EXACT end-{back} {kind}: {vals:?}");
            }
            let ordered = vals.windows(2).all(|w| w[0] > w[1]);
            if ordered && vals.iter().all(|&v| v > 100 && v <= 10_000_000) {
                println!("ORDER end-{back} {kind}: {vals:?}");
            }
        }
    }
    Ok(())
}
