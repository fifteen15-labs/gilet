//! Walks the §6h roster table reading the newly decoded club reputation:
//! entry = `[eid2][club uid][club uid] 0a 00 [u8 last-season position]
//! [u16 REPUTATION] [division eid][division eid] FF FF FF FF ...`.
//! One pass over the frame keyed on the doubled club uid, then a report:
//! coverage over the club table, the ladder clubs against fminside, and a
//! top-20 to eyeball (the most reputable clubs in the world are not in
//! dispute).
//!
//! ```text
//! cargo run --release --example clubrepwalk -- <save.fm> [name ...]
//! ```

// Research spike, not shipped code.
#![allow(
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

use std::collections::HashMap;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

struct Row {
    rep: u16,
    position: u8,
    division: u32,
    at: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: clubrepwalk <save.fm> [club name ...]");
        std::process::exit(2);
    };
    let names: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let main = frames.iter().max_by_key(|f| f.data.len()).ok_or("no frames")?;
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    // Club uid -> club index. Uids are unique per club record in every save
    // tested; keep the first and count collisions honestly.
    let mut by_uid: HashMap<u32, usize> = HashMap::new();
    let mut uid_collisions = 0usize;
    for (i, c) in save.clubs.iter().enumerate() {
        if let Some(uid) = c.uid {
            if by_uid.insert(uid, i).is_some() {
                uid_collisions += 1;
            }
        }
    }

    // One pass: doubled u32 that is a known club uid, `0a 00` behind it,
    // doubled division u32 and the FF run at the documented offsets.
    let mut rows: HashMap<usize, Row> = HashMap::new();
    let mut shape_misses = 0usize;
    for p in 0..data.len().saturating_sub(25) {
        let (Some(a), Some(b)) = (u32_at(data, p), u32_at(data, p + 4)) else { continue };
        if a != b || a == 0 || a == u32::MAX {
            continue;
        }
        let Some(&club_idx) = by_uid.get(&a) else { continue };
        if data.get(p + 8) != Some(&0x0a) || data.get(p + 9) != Some(&0x00) {
            continue;
        }
        let (Some(d1), Some(d2), Some(ff)) =
            (u32_at(data, p + 13), u32_at(data, p + 17), u32_at(data, p + 21))
        else {
            continue;
        };
        if d1 != d2 || ff != u32::MAX {
            shape_misses += 1;
            continue;
        }
        let rep = u16_at(data, p + 11).unwrap_or(0);
        let position = data[p + 10];
        if rows.contains_key(&club_idx) {
            // Second row for the same club would make the read ambiguous.
            println!(
                "DUPLICATE row for {} at 0x{p:x} (first at 0x{:x})",
                save.clubs[club_idx].name, rows[&club_idx].at
            );
            continue;
        }
        rows.insert(club_idx, Row { rep, position, division: d1, at: p });
    }

    let with_uid = save.clubs.iter().filter(|c| c.uid.is_some()).count();
    println!(
        "clubs {} (with uid {with_uid}, uid collisions {uid_collisions}); roster rows matched {}; near-miss shapes {}",
        save.clubs.len(),
        rows.len(),
        shape_misses
    );
    let over = rows.values().filter(|r| r.rep > 10_000).count();
    let zero = rows.values().filter(|r| r.rep == 0).count();
    println!("rep > 10000: {over}; rep == 0: {zero}");

    // Top 20 by reputation.
    let mut all: Vec<(&usize, &Row)> = rows.iter().collect();
    all.sort_by_key(|(_, r)| std::cmp::Reverse(r.rep));
    println!("\ntop 20 by reputation:");
    for (idx, row) in all.iter().take(20) {
        let c = &save.clubs[**idx];
        println!(
            "  {:5}  {}  (pos {}, division eid {}, row 0x{:x})",
            row.rep, c.name, row.position, row.division, row.at
        );
    }

    // Named clubs.
    if !names.is_empty() {
        println!("\nnamed clubs:");
        for want in &names {
            for (i, c) in save.clubs.iter().enumerate() {
                if c.name.to_lowercase() != *want && c.short_name.to_lowercase() != *want {
                    continue;
                }
                match rows.get(&i) {
                    Some(r) => println!(
                        "  {} / {} eid {:?}: rep {} (pos {}, div {})",
                        c.name, c.short_name, c.eid, r.rep, r.position, r.division
                    ),
                    None => println!("  {} / {} eid {:?}: NO ROSTER ROW", c.name, c.short_name, c.eid),
                }
            }
        }
    }
    Ok(())
}
