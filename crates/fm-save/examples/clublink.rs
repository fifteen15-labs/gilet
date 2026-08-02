//! Hunts the person→club reference inside person records: for players whose
//! club is known from the squad table, scans the whole record span (and the
//! contract window before it) for the club's eid, uid and `club_id`, printing
//! offsets relative to the record prefix. A stable relative offset across
//! players is the link; noise is not (`OPEN_PROBLEMS` §1 residual 1).
//!
//! ```text
//! cargo run --release --example clublink -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: clublink <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let save = fm_save::Save::parse(&bytes)?;

    let mut offsets: Vec<usize> = save.people.iter().map(|p| p.offset).collect();
    offsets.sort_unstable();

    for probe in [
        "Erling Braut Haaland",
        "Virgil van Dijk",
        "Bukayo Ayoyinka Saka",
        "Pape Matar Sarr",
        "Willian Borges da Silva",
        "Jonathan Calleri",
    ] {
        let Some(p) = save.people.iter().find(|q| q.full_name == probe) else {
            println!("{probe}: not in save");
            continue;
        };
        let end = offsets
            .iter()
            .find(|&&o| o > p.offset)
            .copied()
            .unwrap_or(main.data.len())
            .min(p.offset + 4000);
        let club = p
            .club_eid
            .and_then(|e| save.clubs.iter().find(|c| c.eid == Some(e)));
        match club {
            Some(c) => {
                println!(
                    "{probe}: record 0x{:x}..+{}  club {} (eid {:?} uid {:?} club_id {})",
                    p.offset,
                    end - p.offset,
                    c.name,
                    c.eid,
                    c.uid,
                    c.club_id
                );
                let from = p.offset.saturating_sub(300);
                for (label, v) in [
                    ("eid", c.eid.unwrap_or(0)),
                    ("uid", c.uid.unwrap_or(0)),
                    ("club_id", c.club_id),
                ] {
                    let needle = v.to_le_bytes();
                    let mut at = from;
                    let mut hits = Vec::new();
                    while let Some(h) = find_from(&main.data, at, needle) {
                        if h >= end {
                            break;
                        }
                        hits.push(i64::try_from(h).unwrap_or(0) - i64::try_from(p.offset).unwrap_or(0));
                        at = h + 1;
                    }
                    if !hits.is_empty() {
                        println!("    club {label} {v} at rel {hits:?}");
                    }
                }
            }
            None => println!("{probe}: no known club (record 0x{:x}..+{})", p.offset, end - p.offset),
        }

        // The contract anchor: [eid][u32 ?][00 00 00 00][wage] — what is the
        // u32 between the eid and the zero run?
        if let Some(eid) = p.eid {
            let lo = p.offset.saturating_sub(220);
            let needle = eid.to_le_bytes();
            let mut at = lo;
            while let Some(h) = find_from(&main.data, at, needle) {
                if h >= p.offset {
                    break;
                }
                at = h + 1;
                if main.data.get(h + 8..h + 12) != Some(&[0u8; 4][..]) {
                    continue;
                }
                let after = read_u32(&main.data, h + 4);
                let wage = read_u32(&main.data, h + 12);
                let named = after.and_then(|v| {
                    save.clubs
                        .iter()
                        .find(|c| c.uid == Some(v) || c.eid == Some(v))
                        .map(|c| c.name.as_str())
                });
                println!("    contract anchor @rel {}: u32-after-eid {after:?} ({named:?})  wage {wage:?}",
                    i64::try_from(h).unwrap_or(0) - i64::try_from(p.offset).unwrap_or(0));
            }
        }
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
