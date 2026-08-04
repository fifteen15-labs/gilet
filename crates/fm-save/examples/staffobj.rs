//! Finds every occurrence of a repeated-uid identity pair in the main frame
//! and dumps the bytes that follow — hunting the external non-player object
//! that `staffscan` showed carries a 54-value attribute block
//! (`OPEN_PROBLEMS` §3b).
//!
//! ```text
//! cargo run --release --example staffobj -- <save.fm> <uid>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (Some(path), Some(uid)) = (std::env::args().nth(1), std::env::args().nth(2)) else {
        eprintln!("usage: staffobj <save.fm> <uid>");
        std::process::exit(2);
    };
    let uid: u32 = uid.parse()?;

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;

    let mut pattern = Vec::with_capacity(8);
    pattern.extend_from_slice(&uid.to_le_bytes());
    pattern.extend_from_slice(&uid.to_le_bytes());

    let hits: Vec<usize> = data
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, w)| *w == pattern.as_slice())
        .map(|(i, _)| i)
        .collect();
    println!("uid {uid}: {} occurrence(s) of the doubled pair", hits.len());

    for &h in hits.iter().take(12) {
        // Show the 16 bytes before (object header + id) and 160 after.
        let start = h.saturating_sub(16);
        println!("\n== pair at 0x{h:x}");
        let mut at = start;
        while at < h + 160 {
            let row: Vec<String> =
                data.iter().skip(at).take(32).map(|b| format!("{b:02x}")).collect();
            println!("0x{at:08x}: {}", row.join(" "));
            at += 32;
        }
    }
    Ok(())
}
