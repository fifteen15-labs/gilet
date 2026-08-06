//! Finds an ASCII needle in every decompressed frame and prints where it sits,
//! with a hex window around each hit. For asking "is this name in the file at
//! all?" before asking why the parser did not surface it.
//!
//! ```text
//! cargo run --release --example strgrep -- <save.fm> "Heybridge" [window]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(needle)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: strgrep <save.fm> <needle> [window]");
        std::process::exit(2);
    };
    let window: usize = args.get(3).and_then(|w| w.parse().ok()).unwrap_or(48);

    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let pat = needle.as_bytes();

    let mut hits = 0;
    for (i, frame) in frames.iter().enumerate() {
        let data = &frame.data;
        let mut at = 0;
        while let Some(found) = data
            .get(at..)
            .and_then(|tail| tail.windows(pat.len()).position(|w| w == pat))
        {
            let off = at + found;
            let lo = off.saturating_sub(window);
            let hi = (off + pat.len() + window).min(data.len());
            let slice = data.get(lo..hi).unwrap_or_default();
            let text: String = slice
                .iter()
                .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
                .collect();
            println!("frame {i}  0x{off:x}  {text}");
            println!("            {}", hex(slice));
            hits += 1;
            at = off + 1;
        }
    }
    println!("\n{hits} hits for {needle:?} across {} frames", frames.len());
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x} ");
        s
    })
}
