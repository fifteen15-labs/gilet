#![allow(clippy::cast_precision_loss)]
//! Reports player-reputation coverage after the tag-02 line binding, plus
//! named spot checks. The line's CA/PA must repeat the person's parsed
//! ability exactly, so every bound reading is self-verified.
//!
//! ```text
//! cargo run --release --example playerrep -- <save.fm> [name...]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: playerrep <save.fm> <name>...");
        std::process::exit(2);
    };
    let names: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let players = save.people.iter().filter(|p| p.is_player()).count();
    let with_rep = save
        .people
        .iter()
        .filter(|p| p.is_player() && p.reputation.is_some())
        .count();
    println!(
        "players: {players}, with reputation: {with_rep} ({:.1}%)",
        with_rep as f64 / players as f64 * 100.0
    );

    for needle in &names {
        for p in save
            .people
            .iter()
            .filter(|p| p.full_name.to_lowercase().contains(needle) && p.is_player())
            .take(3)
        {
            match &p.reputation {
                Some(r) => println!(
                    "{}: home {} current {} world {} (x50: {}/{}/{})",
                    p.full_name,
                    r.home,
                    r.current,
                    r.world,
                    u32::from(r.home) * 50,
                    u32::from(r.current) * 50,
                    u32::from(r.world) * 50,
                ),
                None => println!("{}: no reputation bound", p.full_name),
            }
        }
    }

    // Raw diagnosis: for each named person, walk the frame for their
    // one-below identity triple and print whatever five u16s follow the tag,
    // with no acceptance test — shows which gate is the one that fails.
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    for needle in &names {
        for p in save
            .people
            .iter()
            .filter(|p| p.full_name.to_lowercase().contains(needle) && p.is_player())
            .take(1)
        {
            let (Some(eid), Some(ability)) = (p.eid, p.ability.as_ref()) else {
                continue;
            };
            let Some(below) = eid.checked_sub(1) else { continue };
            let pat = below.to_le_bytes();
            for i in 0..data.len().saturating_sub(24) {
                if data.get(i..i + 4) != Some(&pat[..]) {
                    continue;
                }
                let rd32 = |at: usize| {
                    data.get(at..at + 4)
                        .and_then(|s| <[u8; 4]>::try_from(s).ok())
                        .map(u32::from_le_bytes)
                };
                let (Some(u1), Some(u2)) = (rd32(i + 4), rd32(i + 8)) else {
                    continue;
                };
                if u1 != u2 || u1 == 0 || u1 == u32::MAX {
                    continue;
                }
                let tag = data.get(i + 12).copied();
                let rd16 = |at: usize| {
                    data.get(at..at + 2)
                        .and_then(|s| <[u8; 2]>::try_from(s).ok())
                        .map(u16::from_le_bytes)
                };
                let vals: Vec<u16> = (0..5).filter_map(|k| rd16(i + 13 + k * 2)).collect();
                println!(
                    "{}: triple for eid-1 {below} at 0x{i:x} tag {tag:02x?} fields {vals:?} parsed CA/PA {}/{}",
                    p.full_name, ability.current, ability.potential
                );
            }
        }
    }
    Ok(())
}
