//! Hex-dumps a byte range of one frame. Companion to `staffscan`, for
//! inspecting a hit's neighbourhood.
//!
//! ```text
//! cargo run --release --example dumpat -- <save.fm> <frame> <offset-hex> <len>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(frame), Some(offset), Some(len)) =
        (args.get(1), args.get(2), args.get(3), args.get(4))
    else {
        eprintln!("usage: dumpat <save.fm> <frame> <offset-hex> <len>");
        std::process::exit(2);
    };
    let frame: usize = frame.parse()?;
    let offset = usize::from_str_radix(offset.trim_start_matches("0x"), 16)?;
    let len: usize = len.parse()?;

    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(f) = frames.get(frame) else {
        eprintln!("no frame {frame}");
        std::process::exit(2);
    };
    println!("frame {frame}: {} bytes", f.data.len());

    let mut at = offset;
    while at < offset + len {
        let row: Vec<String> =
            f.data.iter().skip(at).take(32).map(|b| format!("{b:02x}")).collect();
        println!("0x{at:08x}: {}", row.join(" "));
        at += 32;
    }
    Ok(())
}
