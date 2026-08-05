//! Probes `hall_of_fame.dat` as the staff-employment source: entries carry
//! an inline name, the person's uid after an `a6 07 4f 00` marker, and a
//! spell list whose `03 03 00`-tagged u32 reads as **club uid + 1** (Slot
//! 676 → Liverpool 675, Arteta 602 → Arsenal 601). Joins the entries
//! against the parsed save to measure coverage and print known people.
//!
//! ```text
//! cargo run --release --example hof -- <save.fm> [name substring...]
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: hof <save.fm> [name substring...]");
        std::process::exit(2);
    };
    let needles: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let members = frames
        .last()
        .and_then(|f| fm_save::manifest::read_manifest(&f.data));
    let Some(index) = members
        .as_deref()
        .and_then(|m| fm_save::manifest::frame_index_of(m, "hall_of_fame.dat"))
    else {
        eprintln!("no hall_of_fame.dat");
        return Ok(());
    };
    let Some(frame) = frames.get(index) else { return Ok(()) };
    let data = &frame.data;
    let save = fm_save::Save::parse(&bytes)?;

    let people: std::collections::HashMap<u32, &fm_save::Person> =
        save.people.iter().filter_map(|p| Some((p.uid?, p))).collect();
    let clubs: std::collections::HashMap<u32, &fm_save::Club> =
        save.clubs.iter().filter_map(|c| Some((c.uid?, c))).collect();

    let marker = [0xA6, 0x07, 0x4F, 0x00];
    let mut total = 0usize;
    let mut with_person = 0usize;
    let mut with_club = 0usize;
    let mut at = 0usize;
    while at + 8 <= data.len() {
        if data.get(at..at + 4) != Some(&marker[..]) {
            at += 1;
            continue;
        }
        let Some(uid) = read_u32(data, at + 4) else { break };
        total += 1;
        // First spell: the `03 03 00` tag within the next 32 bytes, club uid+1 after.
        let club_uid = (at + 8..at + 40).find_map(|j| {
            (data.get(j..j + 3) == Some(&[0x03, 0x03, 0x00][..]))
                .then(|| read_u32(data, j + 3))
                .flatten()
                .and_then(|v| v.checked_sub(1))
        });
        let person = people.get(&uid);
        let club = club_uid.and_then(|u| clubs.get(&u));
        if person.is_some() {
            with_person += 1;
        }
        if club.is_some() {
            with_club += 1;
        }
        if let Some(p) = person {
            let name = p.full_name.to_lowercase();
            if needles.iter().any(|n| name.contains(n.as_str())) {
                println!(
                    "0x{at:x}  {}  -> club {:?}",
                    p.full_name,
                    club.map(|c| c.name.as_str())
                );
            }
        }
        at += 8;
    }
    println!("\n{total} entries; {with_person} match a parsed person's uid; {with_club} first-spell clubs resolve");
    Ok(())
}
