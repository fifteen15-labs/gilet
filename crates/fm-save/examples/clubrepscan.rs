//! Hunts the club reputation field. For each ladder club (published FM26
//! reputation r on fminside's /100 display, so the DB value should sit near
//! r*100), lists every offset in the club's body whose u16/u32 reads as a
//! value consistent with r under a handful of scales. Common structural
//! positions across clubs are then read off by eye.
//!
//! ```text
//! cargo run --release --example clubrepscan -- <save.fm>
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: clubrepscan <save.fm>");
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
            println!("{label}: eid {eid} not in club table");
            continue;
        };
        let tail = c.offset + 4 + c.name.len() + 4 + c.short_name.len();
        let end = offsets.iter().find(|&&o| o > c.offset).copied().unwrap_or(data.len());
        let want = u64::from(rep) * 100;
        println!(
            "\n=== {label} (display {rep}, db ~{want}) offset 0x{:x} tail +{} body {} ===",
            c.offset,
            tail - c.offset,
            end - c.offset
        );
        // The first bytes after the short name, for shape comparison.
        let head: Vec<String> =
            data[tail..(tail + 40).min(end)].iter().map(|b| format!("{b:02x}")).collect();
        println!("tail bytes: {}", head.join(" "));

        let lo = want.saturating_sub(99);
        let hi = want + 99;
        for at in c.offset..end.min(data.len()) {
            let d = at as i64 - tail as i64; // delta from record tail
            if let Some(v) = u16_at(data, at) {
                let v = u64::from(v);
                if (lo..=hi).contains(&v) {
                    println!("  tail{d:+5}  u16        {v}");
                }
            }
            if let Some(v) = u32_at(data, at) {
                let v = u64::from(v);
                if (lo..=hi).contains(&v) {
                    println!("  tail{d:+5}  u32        {v}");
                }
                if v % 1000 == 0 && (lo * 1000..=hi * 1000).contains(&v) {
                    println!("  tail{d:+5}  u32x1000   {v} -> {}", v / 1000);
                }
                if v % 100 == 0 && (lo * 100..=hi * 100).contains(&v) {
                    println!("  tail{d:+5}  u32x100    {v} -> {}", v / 100);
                }
                if v == u64::from(rep) {
                    println!("  tail{d:+5}  u32==disp  {v}");
                }
            }
        }
    }
    Ok(())
}
