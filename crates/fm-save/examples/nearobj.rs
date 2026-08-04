//! Lists every entity object whose eid is near a given one, with its five
//! u16s and the 54 bytes where a staff attribute block would sit.
//!
//! A person's non-player data lives in a *second* object with its own eid and
//! uid — Haaland's player uid is 29179241 and his non-player uid 29179299,
//! and Sterling's second object is eid 8401 against his person eid 8402. So a
//! player-coach whose record holds no staff block should have one under a
//! neighbouring eid (`OPEN_PROBLEMS.md` §3b).
//!
//! ```text
//! cargo run --release --example nearobj -- <save.fm> <eid> [radius]
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(target)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: nearobj <save.fm> <eid> [radius]");
        std::process::exit(2);
    };
    let target: u32 = target.parse()?;
    let radius: u32 = args.get(3).and_then(|r| r.parse().ok()).unwrap_or(8);

    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;
    let by_eid: std::collections::HashMap<u32, &str> = save
        .people
        .iter()
        .filter_map(|p| Some((p.eid?, p.full_name.as_str())))
        .collect();

    for at in 0..data.len().saturating_sub(24) {
        if data.get(at..at + 2).is_none_or(|h| {
            !matches!(h.first(), Some(0x00..=0x02)) || h.get(1) != Some(&0x40)
        }) {
            continue;
        }
        let (Some(eid), Some(u1), Some(u2)) =
            (read_u32(data, at + 7), read_u32(data, at + 11), read_u32(data, at + 15))
        else {
            continue;
        };
        if u1 != u2 || u1 == 0 || u1 == u32::MAX || eid.abs_diff(target) > radius {
            continue;
        }
        let Some(&tag) = data.get(at + 19) else { continue };
        let fields = at + 20 + usize::from(tag == 0x01) * 2;
        let vals: Vec<u16> = (0..5).filter_map(|i| read_u16(data, fields + i * 2)).collect();

        println!(
            "\n0x{at:x}  eid {eid}  uid {u1}  tag {tag:02x}  fields {vals:?}  person: {}",
            by_eid.get(&eid).copied().unwrap_or("(none)")
        );
        let start = fields + 10 + 8;
        match data.get(start..start + 54) {
            Some(block) if block.iter().all(|&b| (1..=100).contains(&b)) => {
                println!("  block {block:?}");
            }
            Some(block) => {
                let head: Vec<u8> = block.iter().take(8).copied().collect();
                println!("  no block (first bytes {head:?})");
            }
            None => println!("  no block (truncated)"),
        }
    }
    Ok(())
}
