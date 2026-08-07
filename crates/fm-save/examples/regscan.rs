//! Scans the squad table's *secondary* records — B-team and youth squads.
//!
//! The table stores several records per club. Each begins behind a
//! separator whose last two bytes are `FF [flag]`, then `[club eid u32]
//! [00 x10] [ordinal u32] [uid u32]` — the ordinal ascends across the whole
//! table, one per record, which is the spine that separates real records
//! from byte-shifted shadows. Some records carry a `FF FF FF FF [count]`
//! player list (the B or youth squad, armbands after); most are empty.
//!
//! Prints keying stats (does a list's known-member majority match the head's
//! club?) and how many club-less players the lists would bind
//! (`OPEN_PROBLEMS` §1).
//!
//! ```text
//! cargo run --release --example regscan -- <save.fm> [flag-member-eid]
//! ```

use std::collections::{HashMap, HashSet};

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}
fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

const MAX_RECORD: usize = 4_000;
const MAX_LIST: usize = 80;
const MAX_PERSON_EID: u32 = 3_000_000;

fn read_list(frame: &[u8], from: usize, end: usize) -> Option<(Vec<u32>, Option<u32>, Option<u32>)> {
    let mut at = from;
    while at + 6 < end {
        if frame.get(at..at + 4) != Some(&[0xFF; 4]) {
            at += 1;
            continue;
        }
        let count = usize::from(read_u16(frame, at + 4)?);
        if !(1..=MAX_LIST).contains(&count) || at + 6 + count * 4 + 8 > end {
            at += 1;
            continue;
        }
        let list_at = at + 6;
        let mut eids = Vec::with_capacity(count);
        for i in 0..count {
            match read_u32(frame, list_at + i * 4) {
                Some(v) if v > 0 && v < MAX_PERSON_EID => eids.push(v),
                _ => break,
            }
        }
        if eids.len() != count {
            at += 1;
            continue;
        }
        let cap_at = list_at + count * 4;
        let captain = read_u32(frame, cap_at).filter(|&v| v != u32::MAX && v != 0);
        let vice = read_u32(frame, cap_at + 4).filter(|&v| v != u32::MAX && v != 0);
        let ok = |s: Option<u32>| s.is_none_or(|v| eids.contains(&v));
        if !ok(captain) || !ok(vice) {
            at += 1;
            continue;
        }
        if eids
            .iter()
            .enumerate()
            .any(|(i, e)| eids.get(..i).is_some_and(|prior| prior.contains(e)))
        {
            at += 1;
            continue;
        }
        return Some((eids, captain, vice));
    }
    None
}

#[allow(clippy::too_many_lines)] // a linear diagnosis script, read top to bottom
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: regscan <save.fm> [flag-eid]");
        std::process::exit(2);
    };
    let flag_eid: Option<u32> = args.next().and_then(|a| a.parse().ok());

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let club_eids: HashSet<u32> = save.clubs.iter().filter_map(|c| c.eid).collect();
    let club_name: HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();
    let person_club: HashMap<u32, u32> = save
        .people
        .iter()
        .filter_map(|p| Some((p.eid?, p.club_eid?)))
        .collect();
    let claimed: HashSet<usize> = save.squads.iter().map(|s| s.offset).collect();

    // Heads: separator tail `FF [flag]` then `[eid][00 x10][ordinal][uid]`.
    // Every head bounds its neighbour's record; only unclaimed B/youth rows
    // (type 0x13/0x15) above the nation-id range are read. The 0x64 rows are
    // senior teams — nation teams among them, whose eids are nation entities
    // colliding with small club eids.
    let mut all_heads: Vec<usize> = Vec::new();
    let mut kept: Vec<(usize, u32, u32)> = Vec::new();
    let mut at = 2usize;
    while at + 26 < data.len() {
        if data.get(at + 4..at + 14) != Some(&[0u8; 10]) {
            at += 1;
            continue;
        }
        if data.get(at.wrapping_sub(2)) != Some(&0xFF) {
            at += 1;
            continue;
        }
        let (Some(eid), Some(ordinal)) = (read_u32(data, at), read_u32(data, at + 14)) else {
            at += 1;
            continue;
        };
        all_heads.push(at);
        let ty = data.get(at.wrapping_sub(3)).copied().unwrap_or(0);
        if club_eids.contains(&eid)
            && !claimed.contains(&at)
            && (ty == 0x13 || ty == 0x15)
            && eid > 260
        {
            kept.push((at, eid, ordinal));
        }
        at += 26;
    }
    println!("heads: {} total, {} kept B/youth", all_heads.len(), kept.len());

    let kept = longest_ascending(&kept);
    println!("on the ascending-ordinal spine: {}", kept.len());

    let mut records: Vec<(usize, u32, Vec<u32>)> = Vec::new();
    let mut empty = 0usize;
    for &(offset, eid, _) in &kept {
        let next = all_heads.partition_point(|&o| o <= offset);
        let end = all_heads
            .get(next)
            .copied()
            .unwrap_or(data.len())
            .min(offset + MAX_RECORD);
        match read_list(data, offset + 26, end) {
            Some((members, _, _)) => records.push((offset, eid, members)),
            None => empty += 1,
        }
    }
    println!("records with a list: {}, empty: {empty}", records.len());

    let mut judged = 0usize;
    let mut agree = 0usize;
    let mut disagreements = 0usize;
    for (offset, eid, members) in &records {
        let mut counts: HashMap<u32, usize> = HashMap::new();
        let mut known = 0usize;
        for m in members {
            if let Some(&c) = person_club.get(m) {
                *counts.entry(c).or_default() += 1;
                known += 1;
            }
        }
        if flag_eid.is_some_and(|f| members.contains(&f)) {
            println!(
                ">>> flagged member in record @0x{offset:x} head {eid} ({}) n {}",
                club_name.get(eid).copied().unwrap_or("?"),
                members.len()
            );
        }
        if known < 3 {
            continue;
        }
        judged += 1;
        let top = counts.iter().max_by_key(|(_, &n)| n).map(|(&c, &n)| (c, n));
        if top.map(|(c, _)| c) == Some(*eid) {
            agree += 1;
        } else {
            disagreements += 1;
            if disagreements <= 10 {
                if let Some((c, n)) = top {
                    println!(
                        "  DISAGREE @0x{offset:x} head {eid} ({}) majority {c} ({}) {n}/{} of {known} known",
                        club_name.get(eid).copied().unwrap_or("?"),
                        club_name.get(&c).copied().unwrap_or("?"),
                        members.len(),
                    );
                }
            }
        }
    }
    println!("records with >=3 known-club members: {judged}, majority == head club: {agree}");

    let mut appearances: HashMap<u32, Vec<u32>> = HashMap::new();
    for (_, eid, members) in &records {
        for m in members {
            appearances.entry(*m).or_default().push(*eid);
        }
    }
    let mut one = 0usize;
    let mut multi_same = 0usize;
    let mut multi_diff = 0usize;
    for p in &save.people {
        let Some(eid) = p.eid else { continue };
        if p.club_eid.is_some() || p.compact || p.ability.is_none() {
            continue;
        }
        match appearances.get(&eid).map(Vec::as_slice) {
            Some([_]) => one += 1,
            Some(list @ [first, ..]) => {
                if list.iter().all(|c| c == first) {
                    multi_same += 1;
                } else {
                    multi_diff += 1;
                }
            }
            _ => {}
        }
    }
    println!("club-less players in exactly one list: {one}");
    println!("in several lists of the same club: {multi_same}");
    println!("in several lists of differing clubs: {multi_diff}");
    Ok(())
}

/// Longest strictly-ascending run over the third tuple field, patience LIS.
fn longest_ascending(heads: &[(usize, u32, u32)]) -> Vec<(usize, u32, u32)> {
    let mut tails_val: Vec<u32> = Vec::new();
    let mut tails_idx: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; heads.len()];

    for (i, &(_, _, v)) in heads.iter().enumerate() {
        let k = tails_val.partition_point(|&e| e < v);
        if let Some(slot) = prev.get_mut(i) {
            *slot = k.checked_sub(1).and_then(|j| tails_idx.get(j).copied());
        }
        if k == tails_val.len() {
            tails_val.push(v);
            tails_idx.push(i);
        } else if tails_val.get(k).is_some_and(|&e| v < e) {
            if let (Some(tv), Some(ti)) = (tails_val.get_mut(k), tails_idx.get_mut(k)) {
                *tv = v;
                *ti = i;
            }
        }
    }

    let mut chain = Vec::new();
    let mut cur = tails_idx.last().copied();
    while let Some(i) = cur {
        if let Some(h) = heads.get(i) {
            chain.push(*h);
        }
        cur = prev.get(i).copied().flatten();
    }
    chain.reverse();
    chain
}
