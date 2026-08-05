//! Prints the date-bearing regions of every save it is given: header frame
//! bytes 0..96 and database-frame bytes 0..96, plus what the current readers
//! make of them. Ground truth for the date hunt.
//!
//! The database frame is named through the manifest, never taken as the
//! largest frame: on a long career the match-history member outgrows
//! `game_db.dat`, and reading its head is what once made the 26.0.0 week stamp
//! look like a wrong date.
//!
//! ```text
//! cargo run --release --example datescan -- <a.fm> <b.fm> ...
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path)?;
        let frames = fm_save::container::read_frames(&bytes)?;
        println!("=== {path} ===");
        if let Some(header) = frames.first() {
            println!("header {} bytes", header.data.len());
            hex_rows(&header.data, 0, 96);
            println!("  find_wall_clock_date: {:?}", fm_save::gamedate::find_wall_clock_date(&header.data));
        }
        match database_frame(&frames) {
            Some(db) => {
                println!("game_db.dat {} bytes", db.data.len());
                hex_rows(&db.data, 0, 96);
                println!("  find_main_frame_date: {:?}", fm_save::gamedate::find_main_frame_date(&db.data));
            }
            None => println!("game_db.dat: not resolved through the manifest"),
        }
        println!();
    }
    Ok(())
}

/// `game_db.dat` by name, the way `Save::parse` finds it.
fn database_frame(frames: &[fm_save::container::Frame]) -> Option<&fm_save::container::Frame> {
    let members = fm_save::manifest::read_manifest(&frames.last()?.data)?;
    let i = fm_save::manifest::frame_index_of(&members, "game_db.dat")?;
    let plain = members.get(i).map(|m| m.plain)?;
    let frame = frames.get(i)?;
    (frame.data.len() as u64 == plain).then_some(frame)
}

fn hex_rows(data: &[u8], from: usize, len: usize) {
    for row in 0..len.div_ceil(16) {
        let start = from + row * 16;
        let bytes: Vec<String> = data
            .iter()
            .skip(start)
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("  0x{start:03x}: {}", bytes.join(" "));
    }
}
