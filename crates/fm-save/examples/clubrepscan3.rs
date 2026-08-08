//! Third-stage club reputation hunt: dumps the neighbourhood of the scalar
//! candidates (values near display*100 outside the stride-25 deal table) and
//! writes each ladder club's full body as a hex file for eyeballing.
//!
//! ```text
//! cargo run --release --example clubrepscan3 -- <save.fm> <outdir>
//! ```

// Research spike, not shipped code.
#![allow(
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

use std::fmt::Write as _;

const LADDER: &[(&str, u32, u32)] = &[
    ("mancity", 369, 92),
    ("rmadrid", 1079, 91),
    ("liverpool", 366, 89),
    ("millwall", 376, 57),
    ("grimsby", 348, 44),
    ("braintree", 2527, 31),
];

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(outdir)) = (args.next(), args.next()) else {
        eprintln!("usage: clubrepscan3 <save.fm> <outdir>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let main = frames.iter().max_by_key(|f| f.data.len()).ok_or("no frames")?;
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut offsets: Vec<usize> = save.clubs.iter().map(|c| c.offset).collect();
    offsets.sort_unstable();

    for &(label, eid, rep) in LADDER {
        let Some(c) = save.clubs.iter().find(|c| c.eid == Some(eid)) else {
            continue;
        };
        let tail = c.offset + 4 + c.name.len() + 4 + c.short_name.len();
        let end = offsets.iter().find(|&&o| o > c.offset).copied().unwrap_or(data.len());

        // Full-body hex file, 16 bytes a row, deltas from tail.
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{label} display {rep} offset 0x{:x} tail +{} end +{}",
            c.offset,
            tail - c.offset,
            end - c.offset
        );
        let from = c.offset.saturating_sub(64);
        let mut at = from;
        while at < end {
            let to = (at + 16).min(end);
            let hex: Vec<String> = data[at..to].iter().map(|x| format!("{x:02x}")).collect();
            let ascii: String = data[at..to]
                .iter()
                .map(|&x| if (0x20..0x7f).contains(&x) { x as char } else { '.' })
                .collect();
            let rel = at as i64 - tail as i64;
            let _ = writeln!(out, "{rel:+6}  {:<48}  {ascii}", hex.join(" "));
            at = to;
        }
        std::fs::write(format!("{outdir}/{label}.hex"), &out)?;

        // Scalar candidates: near display*100, not repeating at stride 25.
        let want = i64::from(rep) * 100;
        println!("\n=== {label} (display {rep}) ===");
        for at in tail..end.saturating_sub(2) {
            let Some(v) = u16_at(data, at) else { continue };
            let v = i64::from(v);
            if (v - want).abs() > 99 {
                continue;
            }
            let in_run = (at >= tail + 25 && u16_at(data, at - 25) == u16_at(data, at))
                || (at + 27 <= end && u16_at(data, at + 25) == u16_at(data, at));
            if in_run {
                continue;
            }
            println!("  scalar {v} at tail+{} (end-{})", at - tail, end - at);
            let lo = at.saturating_sub(48);
            let hi = (at + 48).min(end);
            let hex: Vec<String> = data[lo..hi].iter().map(|x| format!("{x:02x}")).collect();
            println!("    ctx: {}", hex.join(" "));
        }
    }
    Ok(())
}
