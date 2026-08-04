//! Hex-dumps a range of the main (largest) frame, the coordinate space every
//! other probe reports offsets in. `dumpat` needs a frame index; this does not.
//!
//! ```text
//! cargo run --release --example maindump -- <save.fm> <offset-hex> <len>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(offset), Some(len)) = (args.get(1), args.get(2), args.get(3)) else {
        eprintln!("usage: maindump <save.fm> <offset-hex> <len>");
        std::process::exit(2);
    };
    let offset = usize::from_str_radix(offset.trim_start_matches("0x"), 16)?;
    let len: usize = len.parse()?;

    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };

    let mut at = offset;
    while at < offset.saturating_add(len) {
        let row: Vec<String> =
            main.data.iter().skip(at).take(32).map(|b| format!("{b:02x}")).collect();
        if row.is_empty() {
            break;
        }
        let dec: Vec<String> =
            main.data.iter().skip(at).take(32).map(std::string::ToString::to_string).collect();
        println!("0x{at:08x}: {}", row.join(" "));
        println!("            {}", dec.join(" "));
        at += 32;
    }
    Ok(())
}
