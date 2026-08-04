#![allow(clippy::cast_possible_wrap)]
//! Hunts the exact reputation triple the editor shows for Haaland —
//! 9350/9300/9300 scaled, 187/186/186 raw — anywhere in the main frame,
//! in u16-LE triples and raw byte triples, reporting hits relative to his
//! person record (`OPEN_PROBLEMS` §3b five-u16 semantics).
//!
//! ```text
//! cargo run --release --example repfind -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: repfind <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;
    let haaland = save
        .people
        .iter()
        .find(|p| p.full_name.contains("Erling Braut Haaland"))
        .map(|p| p.offset);
    println!("Haaland record: {haaland:?}");

    // u16-LE triple 9350, 9300, 9300.
    let mut pat = Vec::new();
    pat.extend_from_slice(&9350u16.to_le_bytes());
    pat.extend_from_slice(&9300u16.to_le_bytes());
    pat.extend_from_slice(&9300u16.to_le_bytes());
    let hits: Vec<usize> = data
        .windows(pat.len())
        .enumerate()
        .filter(|(_, w)| *w == pat.as_slice())
        .map(|(i, _)| i)
        .collect();
    println!("u16 triple 9350/9300/9300: {} hit(s)", hits.len());
    for h in hits.iter().take(10) {
        let rel = haaland.map(|o| *h as i64 - o as i64);
        println!("  0x{h:x} (rel to record {rel:?})");
        let row: Vec<String> = data
            .iter()
            .skip(h.saturating_sub(16))
            .take(48)
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("    {}", row.join(" "));
    }

    // Raw byte triple 187 186 186.
    let raw = [187u8, 186, 186];
    let rhits: Vec<usize> = data
        .windows(3)
        .enumerate()
        .filter(|(_, w)| *w == raw)
        .map(|(i, _)| i)
        .collect();
    println!("raw byte triple 187/186/186: {} hit(s)", rhits.len());
    if let Some(o) = haaland {
        for h in &rhits {
            let d = (*h as i64 - o as i64).abs();
            if d < 40_000 {
                println!("  0x{h:x} rel {:+}", *h as i64 - o as i64);
            }
        }
    }
    Ok(())
}
