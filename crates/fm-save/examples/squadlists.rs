//! Dumps every `FF FF FF FF [count u16] [eids]` list inside one club's squad
//! record, with names resolved — hunting the list that holds non-contract
//! ("pay to play") players, who are missing from the parsed squad
//! (`OPEN_PROBLEMS` squad residuals).
//!
//! ```text
//! cargo run --release --example squadlists -- <save.fm> "Club Name"
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
    let (Some(path), Some(who)) = (std::env::args().nth(1), std::env::args().nth(2)) else {
        eprintln!("usage: squadlists <save.fm> <club name>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let by_eid: std::collections::HashMap<u32, &fm_save::Person> =
        save.people.iter().filter_map(|p| Some((p.eid?, p))).collect();

    let Some(club) = save
        .clubs
        .iter()
        .find(|c| c.name.contains(&who) || c.short_name.contains(&who))
    else {
        println!("{who}: no such club");
        return Ok(());
    };
    let (Some(eid), Some(uid)) = (club.eid, club.uid) else {
        println!("{}: club has no identity", club.name);
        return Ok(());
    };
    println!("{} eid {eid} uid {uid}", club.name);

    let Some(squad) = save.squads.iter().find(|s| s.club_eid == eid) else {
        println!("no squad record parsed for this club");
        return Ok(());
    };
    println!(
        "squad record @0x{:x}: {} members parsed",
        squad.offset,
        squad.player_eids.len()
    );

    // Walk the whole record span for every count-marked list.
    let end = (squad.offset + 6_000).min(data.len());
    let mut at = squad.offset + 26;
    let mut nth = 0;
    while at + 6 < end {
        if data.get(at..at + 4) != Some(&[0xFF; 4]) {
            at += 1;
            continue;
        }
        let Some(count) = read_u16(data, at + 4).map(usize::from) else {
            break;
        };
        if !(1..=200).contains(&count) {
            at += 1;
            continue;
        }
        let list_at = at + 6;
        let mut eids = Vec::new();
        for i in 0..count {
            match read_u32(data, list_at + i * 4) {
                Some(v) if v > 0 && v < 3_000_000 => eids.push(v),
                _ => break,
            }
        }
        if eids.len() != count {
            at += 1;
            continue;
        }
        nth += 1;
        let known = eids.iter().filter(|e| by_eid.contains_key(e)).count();
        println!(
            "\nlist {nth} @0x{at:x} (rel +{}): {count} entries, {known} resolve to people",
            at - squad.offset
        );
        let pre: Vec<String> = data
            .iter()
            .skip(at.saturating_sub(40))
            .take(40)
            .map(|b| format!("{b:02x}"))
            .collect();
        let post_at = list_at + count * 4;
        let post: Vec<String> =
            data.iter().skip(post_at).take(16).map(|b| format!("{b:02x}")).collect();
        println!("    pre : {}", pre.join(" "));
        println!("    post: {}", post.join(" "));
        for e in &eids {
            match by_eid.get(e) {
                Some(p) => {
                    let wage = p.wage.map_or("wage -".to_string(), |w| format!("wage {w}"));
                    let until = p
                        .contract_until
                        .map_or("until -".to_string(), |d| format!("until {}-{}", d.year, d.month));
                    let abil = if p.ability.is_some() { "player" } else { "staff?" };
                    let age = save
                        .game_date
                        .map_or("age ?".to_string(), |t| format!("age {}", p.date_of_birth.age_on(t)));
                    println!("    {e:>8}  {:32}  {abil:7} {age:7} {wage:12} {until}", p.full_name);
                }
                None => println!("    {e:>8}  ?"),
            }
        }
        at = list_at + count * 4;
    }
    Ok(())
}
