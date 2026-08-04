//! Tries to decode names for grey-player stub entities — the 33-byte
//! `02 40 10`-headed entries that hold non-contract squad fillers
//! (pay-to-play players). For each stub around a given eid, every u32 window
//! in the entry is tested against the forename and surname pools; whichever
//! offsets resolve for *all* probed stubs are the name fields.
//!
//! ```text
//! cargo run --release --example greynames -- <save.fm> <eid> [eid ...]
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: greynames <save.fm> <eid> [eid ...]");
        std::process::exit(2);
    };
    let eids: Vec<u32> = std::env::args().skip(2).filter_map(|a| a.parse().ok()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let Some(strings) = fm_save::strings::scan_strings(data) else {
        println!("no string table");
        return Ok(());
    };

    for eid in eids {
        // Find the stub: `02 40 10 00 00 00 00 [eid][uid][uid]` with a
        // doubled uid.
        let needle = eid.to_le_bytes();
        let Some(at) = (0..data.len().saturating_sub(19)).find(|&i| {
            data.get(i..i + 7) == Some(&[0x02, 0x40, 0x10, 0, 0, 0, 0][..])
                && data.get(i + 7..i + 11) == Some(&needle[..])
                && read_u32(data, i + 11).is_some_and(|u| Some(u) == read_u32(data, i + 15))
        }) else {
            println!("eid {eid}: no stub found");
            continue;
        };
        let row: Vec<String> =
            data.iter().skip(at).take(44).map(|b| format!("{b:02x}")).collect();
        println!("eid {eid} stub @0x{at:x}:\n  {}", row.join(" "));
        for off in 19..40 {
            let Some(v) = read_u32(data, at + off) else { continue };
            let fore = strings.forenames.get(&v);
            let sur = strings.surnames.get(&v);
            if fore.is_some() || sur.is_some() {
                println!(
                    "    +{off}: {v}  forename={:?} surname={:?}",
                    fore.map(String::as_str),
                    sur.map(String::as_str)
                );
            }
        }
    }
    Ok(())
}
