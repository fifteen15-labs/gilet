//! Measures whether the squad table's unclaimed 0x64-typed rows — senior
//! squads whose uid the club table does not carry, i.e. clubs outside the
//! loaded leagues — can bind safely. The risk is national sides: senior
//! nation teams share the 0x64 type at nation-entity eids (1..250), and any
//! *other* representative entity landing in club-eid space would misbind a
//! whole international squad to a club.
//!
//! Prints spine/keying stats, conflict counts over club-less players, and
//! resolves named probe players (Willian, Calleri) to the row that claims
//! them (`OPEN_PROBLEMS` §1).
//!
//! ```text
//! cargo run --release --example seniorscan -- <save.fm> Willian Calleri
//! ```

use std::collections::HashMap;

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

fn read_list(frame: &[u8], from: usize, end: usize) -> Option<Vec<u32>> {
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
        return Some(eids);
    }
    None
}

#[allow(clippy::too_many_lines)] // a linear diagnosis script, read top to bottom
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: seniorscan <save.fm> [probe name]...");
        std::process::exit(2);
    };
    let probes: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let club_uid: HashMap<u32, u32> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.uid?)))
        .collect();
    let club_name: HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();
    let by_eid: HashMap<u32, &fm_save::Person> =
        save.people.iter().filter_map(|p| Some((p.eid?, p))).collect();

    // All separator-anchored heads bound records; keep unclaimed 0x64 rows
    // above the nation range whose uid the club table does not carry.
    let mut all_heads: Vec<usize> = Vec::new();
    let mut kept: Vec<(usize, u32, u32)> = Vec::new();
    let mut eid_not_in_club_table = 0usize;
    let mut at = 3usize;
    while at + 26 < data.len() {
        if data.get(at + 4..at + 14) != Some(&[0u8; 10]) {
            at += 1;
            continue;
        }
        if data.get(at.wrapping_sub(2)) != Some(&0xFF) {
            at += 1;
            continue;
        }
        let (Some(eid), Some(ordinal), Some(uid)) = (
            read_u32(data, at),
            read_u32(data, at + 14),
            read_u32(data, at + 18),
        ) else {
            at += 1;
            continue;
        };
        all_heads.push(at);
        let ty = data.get(at.wrapping_sub(3)).copied().unwrap_or(0);
        if ty == 0x64 && eid > 260 {
            match club_uid.get(&eid) {
                Some(&cu) if cu != uid => kept.push((at, eid, ordinal)),
                Some(_) => {}
                None => eid_not_in_club_table += 1,
            }
        }
        let _flag = data.get(at.wrapping_sub(1)).copied().unwrap_or(0);
        at += 26;
    }
    println!("kept unclaimed senior rows keyed by a club eid: {}", kept.len());
    println!("0x64 rows above 260 keyed by NO club eid: {eid_not_in_club_table}");

    // Spine.
    let before = kept.len();
    let mut tails_val: Vec<u32> = Vec::new();
    let mut tails_idx: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; kept.len()];
    for (i, &(_, _, v)) in kept.iter().enumerate() {
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
        if let Some(&h) = kept.get(i) {
            chain.push(h);
        }
        cur = prev.get(i).copied().flatten();
    }
    chain.reverse();
    println!("on the ordinal spine: {} (dropped {})", chain.len(), before - chain.len());

    // Read lists; keying stats, split by the separator's flag byte.
    let mut with_list = 0usize;
    let mut judged = 0usize;
    let mut agree = 0usize;
    let mut disagreements = 0usize;
    let mut appearances: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut flag_agree: HashMap<u8, usize> = HashMap::new();
    let mut flag_disagree: HashMap<u8, usize> = HashMap::new();
    let mut flag_thin: HashMap<u8, usize> = HashMap::new();
    for &(offset, eid, _) in &chain {
        let next = all_heads.partition_point(|&o| o <= offset);
        let end = all_heads
            .get(next)
            .copied()
            .unwrap_or(data.len())
            .min(offset + MAX_RECORD);
        let Some(members) = read_list(data, offset + 26, end) else {
            continue;
        };
        with_list += 1;
        let mut counts: HashMap<u32, usize> = HashMap::new();
        let mut known = 0usize;
        for m in &members {
            if let Some(c) = by_eid.get(m).and_then(|p| p.club_eid) {
                *counts.entry(c).or_default() += 1;
                known += 1;
            }
        }
        for m in &members {
            appearances.entry(*m).or_default().push(eid);
        }
        for m in &members {
            if let Some(p) = by_eid.get(m) {
                let lower = p.full_name.to_lowercase();
                if probes.iter().any(|q| lower.contains(q.as_str())) {
                    println!(
                        ">>> {} in row of {} ({})  members {}  known {known}",
                        p.full_name,
                        eid,
                        club_name.get(&eid).copied().unwrap_or("?"),
                        members.len(),
                    );
                }
            }
        }
        let flag = data.get(offset.wrapping_sub(1)).copied().unwrap_or(0);
        if known >= 3 {
            judged += 1;
            let top = counts.iter().max_by_key(|(_, &n)| n).map(|(&c, &n)| (c, n));
            if top.map(|(c, _)| c) == Some(eid) {
                agree += 1;
                *flag_agree.entry(flag).or_default() += 1;
            } else {
                disagreements += 1;
                *flag_disagree.entry(flag).or_default() += 1;
                if disagreements <= 10 || flag & 0x20 == 0 {
                    if let Some((c, n)) = top {
                        println!(
                            "  DISAGREE @0x{offset:x} flag 0x{flag:02x} head {eid} ({}) majority {c} ({}) {n}/{} of {known}",
                            club_name.get(&eid).copied().unwrap_or("?"),
                            club_name.get(&c).copied().unwrap_or("?"),
                            members.len(),
                        );
                    }
                }
            }
        } else {
            *flag_thin.entry(flag).or_default() += 1;
        }
    }
    println!("rows with a list: {with_list}");
    println!("rows with >=3 known-club members: {judged}, majority == head club: {agree}");
    println!("flag bytes — agree: {flag_agree:?}  disagree: {flag_disagree:?}  thin: {flag_thin:?}");

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
    println!("club-less players in exactly one senior list: {one}");
    println!("in several lists of the same club: {multi_same}");
    println!("in several lists of differing clubs: {multi_diff}");
    Ok(())
}
