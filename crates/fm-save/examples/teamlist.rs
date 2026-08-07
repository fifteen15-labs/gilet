//! Parses the boardroom from club record bodies — the bytes right after the
//! short name: `01 [u16] [u32 director-of-football person eid] [flag u8]
//! [count u8] [count x u32 board person eids] 01` — and prints them for
//! named clubs, plus shape statistics over every club.
//!
//! Found while hunting the team→club binding: the u32s resolve to *people*,
//! and the real ones check out — Jean-Louis Leca as Lens' sporting director
//! with Joseph Oughourlian on the board, Hugo Viana at Manchester City,
//! Andrew Cavenagh chairing Rangers (`OPEN_PROBLEMS` §3c: the
//! director-of-football and the chairperson were the undecoded staff). Only
//! ~700 clubs carry this exact shape; the rest vary and are not yet mapped.
//!
//! ```text
//! cargo run --release --example teamlist -- <save.fm> Rangers "Man City" Lens
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

struct TeamList {
    tag: u16,
    first: u32,
    flag: u8,
    teams: Vec<u32>,
}

fn parse_team_list(data: &[u8], body_at: usize) -> Option<TeamList> {
    if data.get(body_at) != Some(&0x01) {
        return None;
    }
    let tag = read_u16(data, body_at + 1)?;
    let first = read_u32(data, body_at + 3)?;
    let flag = *data.get(body_at + 7)?;
    let count = usize::from(*data.get(body_at + 8)?);
    if count > 8 {
        return None;
    }
    let mut teams = Vec::with_capacity(count);
    for i in 0..count {
        teams.push(read_u32(data, body_at + 9 + i * 4)?);
    }
    if data.get(body_at + 9 + count * 4) != Some(&0x01) {
        return None;
    }
    Some(TeamList { tag, first, flag, teams })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: teamlist <save.fm> [club name]...");
        std::process::exit(2);
    };
    let names: Vec<String> = args.collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut parsed = 0usize;
    let mut refused = 0usize;
    for c in &save.clubs {
        // Body starts right after the two length-prefixed strings.
        let body = c.offset + 4 + c.name.len() + 4 + c.short_name.len();
        let list = parse_team_list(data, body);
        match &list {
            Some(_) => parsed += 1,
            None => refused += 1,
        }
        if names
            .iter()
            .any(|n| c.name.contains(n.as_str()) || c.short_name.contains(n.as_str()))
        {
            match list {
                Some(l) => println!(
                    "{} (eid {:?}): tag {} first {} flag {} teams {:?}",
                    c.name, c.eid, l.tag, l.first, l.flag, l.teams
                ),
                None => println!("{} (eid {:?}): no team-list shape", c.name, c.eid),
            }
        }
    }
    println!("\nclubs with team-list shape: {parsed}, without: {refused}");
    Ok(())
}
