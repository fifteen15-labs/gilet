//! Hex-dumps a window of the *largest* frame — the `game_db.dat` database —
//! without needing its frame index, unlike `dumpat`. Offsets are the ones
//! every other probe reports.
//!
//! ```text
//! cargo run --release --example hexmain -- <save.fm> 0x642576 256
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(off_s)) = (args.next(), args.next()) else {
        eprintln!("usage: hexmain <save.fm> <hex offset> [len]");
        std::process::exit(2);
    };
    let start = usize::from_str_radix(off_s.trim_start_matches("0x"), 16)?;
    let len: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(256);

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;

    let end = (start.saturating_add(len)).min(data.len());
    let mut at = start;
    while at < end {
        let row = data.get(at..(at + 16).min(end)).unwrap_or(&[]);
        let hex: String = row.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x} ");
            s
        });
        let ascii: String = row
            .iter()
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { char::from(b) } else { '.' })
            .collect();
        println!("0x{at:08x}  {hex:<48}  {ascii}");
        at += 16;
    }
    Ok(())
}
