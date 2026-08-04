//! Extracts every `02 40 1?`-headed object in the main frame that is
//! followed by a 54-value attribute block, and attributes each to the person
//! record it sits inside (`OPEN_PROBLEMS` §3b). CSV to stdout: one row per
//! block, with the owning person's name where the object is in-record.
//!
//! The block is anchored the same way Emery's was proven: an `8×FF` run
//! within reach of the header, 54 bytes ending exactly at it.
//!
//! ```text
//! cargo run --release --example staffmap -- <save.fm> > blocks.csv
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
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: staffmap <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut people: Vec<(usize, &str)> =
        save.people.iter().map(|p| (p.offset, p.full_name.as_str())).collect();
    people.sort_unstable_by_key(|(o, _)| *o);
    let offsets: Vec<usize> = people.iter().map(|(o, _)| *o).collect();

    let by_eid: std::collections::HashMap<u32, &str> = save
        .people
        .iter()
        .filter_map(|p| Some((p.eid?, p.full_name.as_str())))
        .collect();

    println!("offset,owner,owner_by,rel,eid,uid,tag,t1,t2,t3,p1,p2,{}",
        (0..54).map(|i| format!("s{i}")).collect::<Vec<_>>().join(","));

    let mut found = 0usize;
    for at in 0..data.len().saturating_sub(24) {
        if data.get(at..at + 3).is_none_or(|h| {
            !matches!(h.first(), Some(0x00..=0x02)) || h.get(1) != Some(&0x40)
        }) {
            continue;
        }
        let (Some(eid), Some(u1), Some(u2)) =
            (read_u32(data, at + 7), read_u32(data, at + 11), read_u32(data, at + 15))
        else {
            continue;
        };
        if u1 != u2 || u1 == 0 || u1 == u32::MAX {
            continue;
        }
        let Some(&tag) = data.get(at + 19) else { continue };
        if !matches!(tag, 0x01 | 0x02) {
            continue;
        }

        // The five u16s after the tag: Emery read 5000/5000/1500/125/125.
        // Tag 01 carries them 2 bytes later than tag 02 (Emery's `01 00 00`).
        let fields = at + 20 + usize::from(tag == 0x01) * 2;
        let vals: Vec<u16> = (0..5).filter_map(|i| read_u16(data, fields + i * 2)).collect();

        // Eight bytes of filler follow the five u16s, then the block.
        let start = fields + 10 + 8;
        let Some(block) = data.get(start..start + 54) else { continue };
        if block.iter().any(|&b| b == 0 || b > 100) {
            continue;
        }

        // Owner by proven id where the person has one, else by the record
        // whose span contains the header.
        let idx = offsets.partition_point(|&o| o <= at);
        let spanned = idx
            .checked_sub(1)
            .and_then(|i| people.get(i))
            .filter(|_| offsets.get(idx).is_none_or(|&next| at < next))
            .map_or("", |(_, n)| *n);
        let (owner, owner_by) = by_eid
            .get(&eid)
            .map_or((spanned, "span"), |name| (*name, "eid"));
        let rel = idx
            .checked_sub(1)
            .and_then(|i| offsets.get(i))
            .map_or(0, |&o| at.saturating_sub(o));

        let vs: Vec<String> = vals.iter().map(ToString::to_string).collect();
        let bs: Vec<String> = block.iter().map(ToString::to_string).collect();
        println!(
            "0x{at:x},{owner},{owner_by},{rel},{eid},{u1},{tag:02x},{},{}",
            vs.join(","),
            bs.join(",")
        );
        found += 1;
    }
    eprintln!("{found} blocks");
    Ok(())
}
