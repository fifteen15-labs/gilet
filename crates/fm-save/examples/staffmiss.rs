//! Explains why a person has no staff sheet: finds every entity object whose
//! eid is one below theirs and reports which acceptance check the block
//! failed. For chasing "this manager shows nothing" reports.
//!
//! ```text
//! cargo run --release --example staffmiss -- <save.fm> "Slot" "Arteta"
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

#[allow(clippy::too_many_lines, clippy::naive_bytecount)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: staffmiss <save.fm> <name substring>...");
        std::process::exit(2);
    };
    let needles: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let frame = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut offsets: Vec<usize> = save.people.iter().map(|p| p.offset).collect();
    offsets.sort_unstable();

    let targets: Vec<(u32, String, usize)> = save
        .people
        .iter()
        .filter(|p| {
            let n = p.full_name.to_lowercase();
            needles.iter().any(|q| n.contains(q.as_str()))
        })
        .filter_map(|p| Some((p.eid?, p.full_name.clone(), p.offset)))
        .collect();

    for (person_eid, name, offset) in &targets {
        // Every doubled-uid triple inside the record: the person's own
        // identity plus any referenced entities, whose objects may hold the
        // non-player sheet.
        let end = offsets
            .iter()
            .find(|&&o| o > *offset)
            .copied()
            .unwrap_or(offset + 2048);
        println!("\n-- {name}: triples inside the record 0x{offset:x}..0x{end:x}");
        let mut wants: Vec<u32> = vec![person_eid.checked_sub(1).unwrap_or_default()];
        for at in *offset..end.saturating_sub(12) {
            let (Some(eid), Some(u1), Some(u2)) = (
                read_u32(frame, at),
                read_u32(frame, at + 4),
                read_u32(frame, at + 8),
            ) else {
                continue;
            };
            if u1 == u2 && u1 != 0 && u1 != u32::MAX && eid != 0 && eid < 3_000_000 {
                println!("  +{}: eid {eid} uid {u1}", at - offset);
                if eid != *person_eid {
                    wants.push(eid);
                }
            }
        }

        for want in wants {
        println!("== {name} (person eid {person_eid}, looking for object eid {want})");
        let mut hits = 0usize;
        let mut at = 0usize;
        while at + 24 <= frame.len() {
            let header_ok = frame.get(at).is_some_and(|&b| b <= 0x02)
                && frame.get(at + 1) == Some(&0x40);
            if !header_ok {
                at += 1;
                continue;
            }
            let (Some(eid), Some(u1), Some(u2)) = (
                read_u32(frame, at + 7),
                read_u32(frame, at + 11),
                read_u32(frame, at + 15),
            ) else {
                at += 1;
                continue;
            };
            if eid != want || u1 != u2 || u1 == 0 || u1 == u32::MAX {
                at += 1;
                continue;
            }
            hits += 1;
            println!("object at 0x{at:x}  uid {u1}  tag byte {:02x?}", frame.get(at + 19));
            if frame.get(at + 19) != Some(&0x01) {
                println!("  -> rejected: tag is not 01");
                at += 1;
                continue;
            }
            // Walk the field search window the way scan_staff does and report
            // the nearest miss.
            let mut reason = String::from("no candidate fields in the window");
            for fields in at + 20..at + 84 {
                let vals: Vec<u16> =
                    (0..5).filter_map(|i| read_u16(frame, fields + i * 2)).collect();
                let [home, current, world, ca, pa] = vals[..] else { continue };
                if home > 10_000 || current > 10_000 || world > 10_000 {
                    continue;
                }
                if ca == 0 || ca > 200 || pa < ca || pa > 200 {
                    continue;
                }
                let start = fields + 10 + 8;
                let Some(block) = frame.get(start..start + 54) else { continue };
                let zeros = block.iter().filter(|&&b| b == 0).count();
                let over = block.iter().filter(|&&b| b > 100).count();
                if zeros == 0 && over == 0 {
                    reason = format!(
                        "fields at +{} accepted?! home/cur/world {home}/{current}/{world} ca/pa {ca}/{pa}",
                        fields - at
                    );
                    break;
                }
                reason = format!(
                    "fields at +{} look sane (reps {home}/{current}/{world}, ca/pa {ca}/{pa}) but block has {zeros} zeros, {over} over-100",
                    fields - at
                );
            }
            println!("  -> {reason}");
            let tail: Vec<String> = frame
                .iter()
                .skip(at + 20)
                .take(96)
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("  bytes after tag: {}", tail.join(" "));
            at += 1;
        }
        if hits == 0 {
            println!("no object with eid {want} anywhere in the frame");
        }
        }
    }
    Ok(())
}
