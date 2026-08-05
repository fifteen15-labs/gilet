//! Hex-dumps a named archive member, with printable ASCII alongside.
//!
//! ```text
//! cargo run --release --example memberhex -- <save.fm> save_game_summary.dat [len] [offset-hex]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: memberhex <save.fm> <member> [len]");
        std::process::exit(2);
    };
    let want = args.next().unwrap_or_else(|| "save_game_summary.dat".to_owned());
    let len: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(1024);
    let from: usize = args
        .next()
        .and_then(|s| usize::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let members = frames
        .last()
        .and_then(|f| fm_save::manifest::read_manifest(&f.data))
        .unwrap_or_default();
    let Some(i) = fm_save::manifest::frame_index_of(&members, &want) else {
        eprintln!("no member named {want}");
        for m in members.iter().take(40) {
            eprintln!("  {} ({} B)", m.name, m.plain);
        }
        std::process::exit(1);
    };
    let Some(frame) = frames.get(i) else {
        eprintln!("frame {i} missing");
        std::process::exit(1);
    };
    println!("{want}: frame {i}, {} bytes", frame.data.len());
    let d = &frame.data;
    let end = from.saturating_add(len).min(d.len());
    for row in 0..end.saturating_sub(from).div_ceil(16) {
        let start = from + row * 16;
        let slice: Vec<u8> = d.iter().skip(start).take(16).copied().collect();
        let hex: Vec<String> = slice.iter().map(|b| format!("{b:02x}")).collect();
        let ascii: String = slice
            .iter()
            .map(|b| if (0x20..0x7f).contains(b) { char::from(*b) } else { '.' })
            .collect();
        println!("  0x{start:04x}: {:<47}  {ascii}", hex.join(" "));
    }
    Ok(())
}
