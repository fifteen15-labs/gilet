//! Hunts for the staff→employer link: takes person/club name pairs with a
//! known employment (Slot at Liverpool on a day-one save), and reports every
//! occurrence of the club's eid or uid around the person's record, as a
//! relative offset. A link exists if the offsets agree across people.
//!
//! ```text
//! cargo run --release --example staffclub -- <save.fm> "Arend Martijn Slot=Liverpool" "Mikel Arteta Amatriain=Arsenal"
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

/// How far before the record prefix and after the record to look.
const BEFORE: usize = 768;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: staffclub <save.fm> \"Person Name=Club Name\"...");
        std::process::exit(2);
    };
    let pairs: Vec<(String, String)> = args
        .filter_map(|a| {
            let (person, club) = a.split_once('=')?;
            Some((person.to_owned(), club.to_owned()))
        })
        .collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let frame = &main.data;
    let save = fm_save::Save::parse(&bytes)?;
    let mut offsets: Vec<usize> = save.people.iter().map(|p| p.offset).collect();
    offsets.sort_unstable();

    for (person_name, club_name) in &pairs {
        let Some(person) = save.people.iter().find(|p| &p.full_name == person_name) else {
            println!("{person_name}: not parsed");
            continue;
        };
        let club: Vec<&fm_save::Club> =
            save.clubs.iter().filter(|c| &c.name == club_name || &c.short_name == club_name).collect();
        let Some(club) = club.first() else {
            println!("{club_name}: no such club");
            continue;
        };
        let end = offsets
            .iter()
            .find(|&&o| o > person.offset)
            .copied()
            .unwrap_or(person.offset + 2048);
        println!(
            "\n{person_name} at 0x{:x} (span to 0x{end:x}) — {club_name} eid {:?} uid {:?}",
            person.offset, club.eid, club.uid
        );
        let lo = person.offset.saturating_sub(BEFORE);
        for at in lo..end {
            let Some(v) = read_u32(frame, at) else { continue };
            let hit = club.eid == Some(v) || club.uid == Some(v);
            if !hit {
                continue;
            }
            let rel = if at >= person.offset {
                format!("+{}", at - person.offset)
            } else {
                format!("-{}", person.offset - at)
            };
            let what = if club.eid == Some(v) { "eid" } else { "uid" };
            let ctx: Vec<String> = frame
                .iter()
                .skip(at.saturating_sub(8))
                .take(20)
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("  {what} at {rel}: {}", ctx.join(" "));
        }
    }
    Ok(())
}
