//! Writes one named member's decompressed frame to a file, for probing a
//! subsystem outside the main database (the way `scout_man.dat` fell).
//!
//! ```text
//! cargo run --release --example memberdump -- <save.fm> manager_manager.dat /tmp/mm.bin
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(name), Some(out)) = (args.get(1), args.get(2), args.get(3)) else {
        eprintln!("usage: memberdump <save.fm> <member name> <out file>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(last) = frames.last() else { return Ok(()) };
    let Some(members) = fm_save::manifest::read_manifest(&last.data) else {
        eprintln!("no manifest");
        std::process::exit(1);
    };
    let Some(index) = fm_save::manifest::frame_index_of(&members, name) else {
        eprintln!("no member named {name}");
        std::process::exit(1);
    };
    let Some(frame) = frames.get(index) else {
        eprintln!("member index {index} has no frame");
        std::process::exit(1);
    };
    std::fs::write(out, &frame.data)?;
    println!("{name}: frame {index}, {} bytes -> {out}", frame.data.len());
    Ok(())
}
