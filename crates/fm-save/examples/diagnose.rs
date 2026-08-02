//! Diagnoses the two open reader gaps against a real save: the in-game date
//! (header pair, main-frame week stamp) and players whose contract parsed but
//! whose club did not resolve.
//!
//! ```text
//! cargo run --release --example diagnose -- <save.fm>
//! ```

// A diagnostic dump reads better as one top-to-bottom script than sliced into
// functions for the lint's sake.
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: diagnose <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;

    println!("== date ==");
    if let Some(header) = frames.first() {
        println!("header frame: {} bytes", header.data.len());
        let window: Vec<String> = header
            .data
            .iter()
            .skip(40)
            .take(24)
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("  bytes 40..64: {}", window.join(" "));
        println!("  find_game_date: {:?}", fm_save::gamedate::find_game_date(&header.data));
    }
    let main = frames.iter().max_by_key(|f| f.data.len());
    if let Some(main) = main {
        println!("main frame: {} bytes", main.data.len());
        let window: Vec<String> = main
            .data
            .iter()
            .skip(0x20)
            .take(32)
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("  bytes 0x20..0x40: {}", window.join(" "));
        println!("  find_main_frame_date: {:?}", fm_save::gamedate::find_main_frame_date(&main.data));
    }

    let save = fm_save::Save::parse(&bytes)?;
    println!("  Save::parse game_date: {:?}", save.game_date);

    println!("\n== contracted players without a club ==");
    let squad_eids: std::collections::HashSet<u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().copied())
        .collect();
    let max_squad_club = save.squads.iter().map(|s| s.club_eid).max();
    println!("squads: {}   max club eid with a squad: {max_squad_club:?}", save.squads.len());
    let max_club_eid = save.clubs.iter().filter_map(|c| c.eid).max();
    println!("clubs: {}   max club eid: {max_club_eid:?}", save.clubs.len());

    let mut orphaned = 0usize;
    let mut orphaned_referenced = 0usize;
    let mut samples: Vec<String> = Vec::new();
    for p in &save.people {
        if p.ability.is_none() || p.wage.is_none() || p.club_eid.is_some() {
            continue;
        }
        orphaned += 1;
        let referenced = p.eid.is_some_and(|e| squad_eids.contains(&e));
        if referenced {
            orphaned_referenced += 1;
        }
        if samples.len() < 25 {
            let ca = p.ability.as_ref().map_or(0, |a| a.current);
            samples.push(format!(
                "  eid {:?}  CA {ca:>3}  wage {:?}  until {:?}  in-squad-table {referenced}  {}",
                p.eid, p.wage, p.contract_until, p.full_name
            ));
        }
    }
    let contracted_players = save
        .people
        .iter()
        .filter(|p| p.ability.is_some() && p.wage.is_some())
        .count();
    println!("players with a contract: {contracted_players}");
    println!("of those, no club: {orphaned}  (referenced by a squad list but unresolved: {orphaned_referenced})");
    for s in &samples {
        println!("{s}");
    }

    println!("\n== named club squads ==");
    for name in ["Tottenham Hotspur", "Manchester United", "Liverpool", "Afan Lido"] {
        let Some(club) = save.clubs.iter().find(|c| c.name == name) else {
            println!("  {name}: club not found");
            continue;
        };
        let squad = club.eid.and_then(|e| save.squads.iter().find(|s| s.club_eid == e));
        match squad {
            Some(s) => {
                let names: Vec<&str> = s
                    .player_eids
                    .iter()
                    .filter_map(|e| {
                        save.people
                            .iter()
                            .find(|p| p.eid == Some(*e))
                            .map(|p| p.full_name.as_str())
                    })
                    .collect();
                println!("  {name} (eid {:?}): {} members @0x{:x}", club.eid, s.player_eids.len(), s.offset);
                println!("    {}", names.join(", "));
            }
            None => println!("  {name} (eid {:?}): NO SQUAD RECORD", club.eid),
        }
    }

    println!("\n== where do the orphans' squad lists hide? ==");
    if let Some(main) = main {
        let by_eid: std::collections::HashMap<u32, u32> = save
            .clubs
            .iter()
            .filter_map(|c| Some((c.eid?, c.uid?)))
            .collect();
        let probes: Vec<(String, u32)> = ["Willian Borges da Silva", "Jonathan Calleri", "Jay Rodriguez", "Luciano Federico Acosta"]
            .iter()
            .filter_map(|n| {
                let p = save.people.iter().find(|p| p.full_name == *n)?;
                let eid = p.eid?;
                let uid = p.uid?;
                Some(vec![
                    (format!("{n} eid"), eid),
                    (format!("{n} uid"), uid),
                ])
            })
            .flatten()
            .collect();
        for (label, eid) in probes {
            println!("  {label} {eid}:");
            let needle = eid.to_le_bytes();
            let mut at = 0usize;
            while let Some(pos) = find_from(&main.data, at, needle) {
                at = pos + 1;
                // Is this hit inside an FF-marked count list? Walk back up to
                // MAX_SQUAD entries looking for FF FF FF FF [count u16].
                for back in 1..=80usize {
                    let Some(list_at) = pos.checked_sub(back * 4 + 6) else { break };
                    if main.data.get(list_at..list_at + 4) != Some(&[0xFF; 4]) {
                        continue;
                    }
                    let Some(cnt) = main
                        .data
                        .get(list_at + 4..list_at + 6)
                        .and_then(|s| <[u8; 2]>::try_from(s).ok())
                        .map(|s| usize::from(u16::from_le_bytes(s)))
                    else {
                        break;
                    };
                    if cnt < back || cnt > 80 {
                        continue;
                    }
                    // Hunt a club head in the 600 bytes before the list.
                    let mut found = None;
                    for hb in 26..600 {
                        let Some(h) = list_at.checked_sub(hb) else { break };
                        if main.data.get(h + 4..h + 14) != Some(&[0u8; 10][..]) {
                            continue;
                        }
                        let (Some(ceid), Some(uid), Some(uid2)) = (
                            read_u32(&main.data, h),
                            read_u32(&main.data, h + 18),
                            read_u32(&main.data, h + 22),
                        ) else {
                            continue;
                        };
                        if uid == uid2 && uid != 0 && uid != u32::MAX && ceid < 3_000_000 {
                            found = Some((h, ceid, uid));
                            break;
                        }
                    }
                    match found {
                        Some((h, ceid, uid)) => {
                            let in_table = by_eid.get(&ceid);
                            let name = save
                                .clubs
                                .iter()
                                .find(|c| c.eid == Some(ceid))
                                .map(|c| c.name.as_str());
                            println!(
                                "    list @0x{list_at:x} count {cnt}; head @0x{h:x} club eid {ceid} uid {uid}  table {in_table:?}  {name:?}"
                            );
                        }
                        None => println!("    list @0x{list_at:x} count {cnt}; NO head within 600 bytes"),
                    }
                    break;
                }
            }
        }
    }
    println!("\n== club table coverage ==");
    let with_ids = save.clubs.iter().filter(|c| c.eid.is_some() && c.uid.is_some()).count();
    println!("clubs with (eid, uid): {with_ids} of {}", save.clubs.len());

    // The list at 0x1fa483d holds Sarr and Bissouma but was not parsed as a
    // squad. Hunt backwards for its record head and check it against the club
    // table.
    println!("\n== head hunt before the unparsed list ==");
    if let Some(main) = main {
        let list_at = 0x01fa_483d_usize.saturating_sub(4); // start of FF FF FF FF
        let by_eid: std::collections::HashMap<u32, u32> = save
            .clubs
            .iter()
            .filter_map(|c| Some((c.eid?, c.uid?)))
            .collect();
        for back in 26..600 {
            let Some(at) = list_at.checked_sub(back) else { break };
            let Some(zeros) = main.data.get(at + 4..at + 14) else { continue };
            if zeros != [0u8; 10] {
                continue;
            }
            let eid = read_u32(&main.data, at);
            let uid = read_u32(&main.data, at + 18);
            let uid2 = read_u32(&main.data, at + 22);
            let known = eid.and_then(|e| by_eid.get(&e).copied());
            println!(
                "  candidate head @0x{at:x} (-{back}): eid {eid:?} uid {uid:?}/{uid2:?}  club-table uid {known:?}"
            );
        }
        // Which parsed squads bracket that offset?
        let before = save.squads.iter().filter(|s| s.offset < list_at).max_by_key(|s| s.offset);
        let after = save.squads.iter().filter(|s| s.offset > list_at).min_by_key(|s| s.offset);
        for (tag, s) in [("prev", before), ("next", after)] {
            if let Some(s) = s {
                let club = save.clubs.iter().find(|c| c.eid == Some(s.club_eid));
                println!(
                    "  {tag} parsed squad @0x{:x} club eid {} ({:?})",
                    s.offset,
                    s.club_eid,
                    club.map(|c| c.name.as_str())
                );
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
