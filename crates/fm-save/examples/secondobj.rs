//! Locates the second entity object of known people by structure: every
//! `00 00 00 [eid][uid][uid]` block whose eid is the person's or the
//! person's + 1, reported relative to the record boundaries. Ground work for
//! attributing reputation and staff rows safely (`OPEN_PROBLEMS` §3b).
//!
//! ```text
//! cargo run --release --example secondobj -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: secondobj <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let save = fm_save::Save::parse(&bytes)?;

    // People in file order, so the next record's prefix bounds this one.
    let mut order: Vec<usize> = (0..save.people.len()).collect();
    order.sort_by_key(|&i| save.people.get(i).map_or(0, |p| p.offset));

    for probe in [
        "Erling Braut Haaland",
        "Bukayo Ayoyinka Saka",
        "Virgil van Dijk",
        "Jamal Musiala",
        "Unai Emery",
    ] {
        let Some(pos) = order
            .iter()
            .position(|&i| save.people.get(i).is_some_and(|p| p.full_name == probe))
        else {
            println!("{probe}: not found");
            continue;
        };
        let Some(p) = order.get(pos).and_then(|&i| save.people.get(i)) else {
            continue;
        };
        let next_offset = order
            .get(pos + 1)
            .and_then(|&i| save.people.get(i))
            .map(|q| q.offset);
        let (Some(eid), Some(uid)) = (p.eid, p.uid) else {
            println!("{probe}: no identity");
            continue;
        };
        println!(
            "{probe}: record @0x{:x}  next record @{:?}  eid {eid} uid {uid}",
            p.offset,
            next_offset.map(|o| format!("0x{o:x}")),
        );

        for want in [eid, eid + 1, eid + 2] {
            let needle = want.to_le_bytes();
            let mut at = 0usize;
            let mut shown = 0usize;
            while let Some(h) = find_from(&main.data, at, needle) {
                at = h + 1;
                // Identity shape: uid repeated right after the eid, three
                // zero bytes before it.
                let (Some(u1), Some(u2)) = (read_u32(&main.data, h + 4), read_u32(&main.data, h + 8))
                else {
                    continue;
                };
                if u1 != u2 || u1 == 0 || u1 == u32::MAX {
                    continue;
                }
                if main.data.get(h.wrapping_sub(3)..h) != Some(&[0, 0, 0][..]) {
                    continue;
                }
                let rel = i64::try_from(h).unwrap_or(0) - i64::try_from(p.offset).unwrap_or(0);
                let after: Vec<String> = main
                    .data
                    .iter()
                    .skip(h + 12)
                    .take(28)
                    .map(|b| format!("{b:02x}"))
                    .collect();
                println!(
                    "  eid {want} uid {u1} @0x{h:x} (rel {rel:+})  after: {}",
                    after.join(" ")
                );
                shown += 1;
                if shown >= 6 {
                    break;
                }
            }
        }
        println!();
    }
    Ok(())
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn find_from(haystack: &[u8], from: usize, needle: [u8; 4]) -> Option<usize> {
    let slice = haystack.get(from..)?;
    slice
        .windows(4)
        .position(|w| w == needle)
        .map(|p| p + from)
}
