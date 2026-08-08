//! Frame-wide club reputation hunt. The deal table stamps City 9150,
//! Liverpool 8900, Millwall 5650, Grimsby 4400 — matching fminside exactly
//! under round(v/100) — so those are taken as each club's true value and the
//! whole frame is searched for a u16 with that value sitting within a window
//! of the club's eid or uid (as u32 LE). People's reputations live on a
//! separate object found the same way; clubs may too.
//!
//! ```text
//! cargo run --release --example clubrepscan4 -- <save.fm>
//! ```

// Research spike, not shipped code.
#![allow(
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

/// (label, eid, exact stamp or (lo, hi) window when unknown).
const LADDER: &[(&str, u32, u16, u16)] = &[
    ("mancity", 369, 9150, 9150),
    ("rmadrid", 1079, 9050, 9149),
    ("liverpool", 366, 8900, 8900),
    ("millwall", 376, 5650, 5650),
    ("grimsby", 348, 4400, 4400),
    ("braintree", 2527, 3050, 3149),
];

const WINDOW: usize = 96;

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn find_all(hay: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0;
    while from + needle.len() <= hay.len() {
        let Some(p) = hay[from..].windows(needle.len()).position(|w| w == needle) else {
            break;
        };
        out.push(from + p);
        from += p + 1;
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: clubrepscan4 <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let main = frames.iter().max_by_key(|f| f.data.len()).ok_or("no frames")?;
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    for &(label, eid, lo, hi) in LADDER {
        let Some(c) = save.clubs.iter().find(|c| c.eid == Some(eid)) else {
            continue;
        };
        let uid = c.uid.unwrap_or(0);
        println!(
            "\n=== {label} eid {eid} uid {uid} record 0x{:x} want [{lo}..{hi}] ===",
            c.offset
        );
        for (idname, idval) in [("eid", eid), ("uid", uid)] {
            let hits = find_all(data, &idval.to_le_bytes());
            println!("  {idname} {idval}: {} occurrences", hits.len());
            if hits.len() > 20_000 {
                println!("  (too many, skipping window scan)");
                continue;
            }
            for h in hits {
                let from = h.saturating_sub(WINDOW);
                let to = (h + WINDOW).min(data.len().saturating_sub(2));
                for at in from..to {
                    let Some(v) = u16_at(data, at) else { continue };
                    if v < lo || v > hi {
                        continue;
                    }
                    // Skip matches inside the club record body itself
                    // (already surveyed) — anything within 4KB of the record.
                    if h.abs_diff(c.offset) < 4096 {
                        continue;
                    }
                    let d = at as i64 - h as i64;
                    let lo_c = at.saturating_sub(24);
                    let hi_c = (at + 24).min(data.len());
                    let hex: Vec<String> =
                        data[lo_c..hi_c].iter().map(|x| format!("{x:02x}")).collect();
                    println!(
                        "    {idname}@0x{h:x} {v} at {idname}{d:+}  ctx {}",
                        hex.join(" ")
                    );
                }
            }
        }
    }
    Ok(())
}
