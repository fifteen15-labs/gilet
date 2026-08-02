//! Prints the date-bearing regions of every save it is given: header frame
//! bytes 0..96 and main-frame bytes 0..96, plus what the current readers make
//! of them. Ground truth for the 26.2.0 date hunt.
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
            println!("  find_game_date: {:?}", fm_save::gamedate::find_game_date(&header.data));
        }
        if let Some(main) = frames.iter().max_by_key(|f| f.data.len()) {
            println!("main {} bytes", main.data.len());
            hex_rows(&main.data, 0, 96);
            println!("  find_main_frame_date: {:?}", fm_save::gamedate::find_main_frame_date(&main.data));
        }
        println!();
    }
    Ok(())
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
