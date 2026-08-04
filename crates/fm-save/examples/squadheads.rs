//! Counts every squad-record head for one club in the main frame — testing
//! whether a club has one squad record with several team lists, or several
//! records (first team, U19, B, …) of which the LIS walk keeps only one.
//!
//! ```text
//! cargo run --release --example squadheads -- <save.fm> "Club Name"
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (Some(path), Some(who)) = (std::env::args().nth(1), std::env::args().nth(2)) else {
        eprintln!("usage: squadheads <save.fm> <club name>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let Some(club) = save
        .clubs
        .iter()
        .find(|c| c.name.contains(&who) || c.short_name.contains(&who))
    else {
        println!("{who}: no such club");
        return Ok(());
    };
    let (Some(eid), Some(uid)) = (club.eid, club.uid) else {
        return Ok(());
    };
    println!("{}: eid {eid} uid {uid}", club.name);
    let chosen = save.squads.iter().find(|s| s.club_eid == eid).map(|s| s.offset);
    println!("LIS-chosen squad record: {chosen:?}");

    // Every `[eid][00 x10][u32][uid][uid]` head for this club, anywhere.
    let needle = eid.to_le_bytes();
    for at in 0..data.len().saturating_sub(26) {
        if data.get(at..at + 4) != Some(&needle[..]) {
            continue;
        }
        if data.get(at + 4..at + 14) != Some(&[0u8; 10][..]) {
            continue;
        }
        let (u1, u2) = (read_u32(data, at + 18), read_u32(data, at + 22));
        if u1 == Some(uid) && u2 == Some(uid) {
            let mark = if Some(at) == chosen { "  <- chosen" } else { "" };
            println!("head @0x{at:x}{mark}");
        }
    }
    Ok(())
}
