//! Hunts the identity block for given person eids — `[eid][uid][uid]` with
//! the doubled uid — and shows the surrounding person record, to confirm that
//! unresolved squad members are people whose identity lost the LIS race in
//! `bind_identities` (`OPEN_PROBLEMS` §3b residuals).
//!
//! ```text
//! cargo run --release --example eidprobe -- <save.fm> <eid> [eid ...]
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: eidprobe <save.fm> <eid> [eid ...]");
        std::process::exit(2);
    };
    let eids: Vec<u32> = std::env::args().skip(2).filter_map(|a| a.parse().ok()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut people: Vec<(usize, &fm_save::Person)> =
        save.people.iter().map(|p| (p.offset, p)).collect();
    people.sort_unstable_by_key(|(o, _)| *o);
    let offsets: Vec<usize> = people.iter().map(|(o, _)| *o).collect();

    for eid in eids {
        println!("eid {eid}:");
        let needle = eid.to_le_bytes();
        let mut found = 0usize;
        for at in 0..data.len().saturating_sub(12) {
            if data.get(at..at + 4) != Some(&needle[..]) {
                continue;
            }
            let (Some(u1), Some(u2)) = (read_u32(data, at + 4), read_u32(data, at + 8)) else {
                continue;
            };
            if u1 != u2 || u1 == 0 || u1 == u32::MAX {
                continue;
            }
            // Which parsed person record does this offset sit inside?
            let idx = offsets.partition_point(|&o| o <= at);
            let host = idx.checked_sub(1).and_then(|i| people.get(i));
            let (owner, rel, owner_eid) = host.map_or(("<none>", 0, None), |(o, p)| {
                (p.full_name.as_str(), at - o, p.eid)
            });
            println!(
                "  identity @0x{at:x} uid {u1}  inside \"{owner}\" (+{rel}, record eid {owner_eid:?})"
            );
            found += 1;
            if found >= 4 {
                break;
            }
        }
        if found == 0 {
            println!("  no doubled-uid identity block anywhere");
        }
    }
    Ok(())
}
