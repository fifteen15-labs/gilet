//! Prints the parsed people nearest a main-frame offset, for asking "which
//! record does this byte belong to, and who parsed around it".
//!
//! ```text
//! cargo run --release --example nearoff -- <save.fm> <offset-hex> [count]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(offset)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: nearoff <save.fm> <offset-hex> [count]");
        std::process::exit(2);
    };
    let offset = usize::from_str_radix(offset.trim_start_matches("0x"), 16)?;
    let count: usize = args.get(3).and_then(|c| c.parse().ok()).unwrap_or(6);

    let bytes = std::fs::read(path)?;
    let save = fm_save::Save::parse(&bytes)?;
    println!("{} people parsed", save.people.len());
    if let (Some(first), Some(last)) = (save.people.first(), save.people.last()) {
        println!("person offsets 0x{:x}..0x{:x}", first.offset, last.offset);
    }

    let split = save.people.partition_point(|p| p.offset <= offset);
    let lo = split.saturating_sub(count);
    for (i, p) in save.people.iter().enumerate().skip(lo).take(count * 2) {
        let mark = if i == split { " <-- first after" } else { "" };
        println!(
            "[{i}] 0x{:x}  eid={:?} uid={:?} club={:?}  {}{mark}",
            p.offset, p.eid, p.uid, p.club_eid, p.full_name
        );
    }
    Ok(())
}
