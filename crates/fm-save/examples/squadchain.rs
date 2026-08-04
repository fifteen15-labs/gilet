//! How many squad heads the ascending-run filter throws away.
//!
//! `scan_squads` accepts a head only when `(eid, uid)` matches the club table,
//! then keeps just the longest strictly-ascending run of entity ids — the
//! table itself. Anything off that chain is discarded. This counts what is
//! discarded and names the clubs, which is the question behind "this club
//! shows no players" once the club record itself parses.
//!
//! Usage: `cargo run --release --example squadchain -- <save.fm> [club eid]`

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

use std::collections::{HashMap, HashSet};

fn read_u32(frame: &[u8], at: usize) -> Option<u32> {
    frame.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// The same head test `squad::head_at` applies.
fn head_at(frame: &[u8], at: usize, by_eid: &HashMap<u32, u32>) -> Option<u32> {
    if frame.get(at + 4..at + 14)? != [0u8; 10] {
        return None;
    }
    let eid = read_u32(frame, at)?;
    let want_uid = *by_eid.get(&eid)?;
    let uid = read_u32(frame, at + 18)?;
    let uid2 = read_u32(frame, at + 22)?;
    (uid == want_uid && uid2 == want_uid).then_some(eid)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: squadchain <save.fm> [club eid]");
    let focus = args.next().and_then(|s| s.parse::<u32>().ok());

    let bytes = std::fs::read(&path).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let main = frames.iter().max_by_key(|f| f.data.len()).expect("main frame");
    let frame = &main.data;

    let by_eid: HashMap<u32, u32> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.uid?)))
        .collect();
    let name_of: HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.name.as_str())))
        .collect();

    // Every head that passes the (eid, uid) test, in file order.
    let mut heads: Vec<(usize, u32)> = Vec::new();
    let mut at = 0usize;
    while at + 26 <= frame.len() {
        if let Some(eid) = head_at(frame, at, &by_eid) {
            heads.push((at, eid));
            at += 26;
        } else {
            at += 1;
        }
    }

    let kept: HashSet<usize> = save.squads.iter().map(|s| s.offset).collect();
    let kept_eids: HashSet<u32> = save.squads.iter().map(|s| s.club_eid).collect();

    println!("{path}");
    println!("  {} heads pass the (eid, uid) test", heads.len());
    println!("  {} squads survive the ascending run and have members", save.squads.len());

    let dropped: Vec<&(usize, u32)> = heads.iter().filter(|(off, _)| !kept.contains(off)).collect();
    println!("  {} heads were dropped", dropped.len());

    // A dropped head whose club has no squad at all is a club losing its only
    // chance at one; a dropped duplicate is harmless.
    let orphaned: Vec<&&(usize, u32)> =
        dropped.iter().filter(|(_, eid)| !kept_eids.contains(eid)).collect();
    println!("  {} of those are the club's ONLY head (club ends up with no squad)", orphaned.len());

    for (off, eid) in orphaned.iter().take(30) {
        // How long a member list would have followed, read the naive way.
        let mut members = 0usize;
        let mut p = off + 26;
        while let Some(v) = read_u32(frame, p) {
            if v == 0 || v > 3_000_000 {
                break;
            }
            members += 1;
            p += 4;
        }
        println!(
            "    eid {eid:>7} at {off:>10}  {:?}  ~{members} u32s follow",
            name_of.get(eid).copied().unwrap_or("?")
        );
    }

    if let Some(want) = focus {
        println!("\n=== club eid {want} ===");
        let mine: Vec<&(usize, u32)> = heads.iter().filter(|(_, e)| *e == want).collect();
        println!("  {} head(s) found for this club", mine.len());
        for (off, _) in &mine {
            println!("    at {off}, kept: {}", kept.contains(off));
            // Where the next accepted head sits, which is the window
            // `read_list` is given.
            let next = heads
                .iter()
                .map(|(o, _)| *o)
                .filter(|o| o > off)
                .min()
                .unwrap_or(frame.len());
            println!("    next head at {next} — window of {} bytes", next - off);

            // Every FFFFFFFF + plausible count in that window, which is what
            // read_list anchors on.
            let limit = next.min(off + 6_000);
            let mut p = off + 26;
            while p + 6 < limit {
                if frame.get(p..p + 4) == Some(&[0xFF; 4]) {
                    let count = frame
                        .get(p + 4..p + 6)
                        .map_or(0, |b| usize::from(u16::from_le_bytes([b[0], b[1]])));
                    if (1..=80).contains(&count) {
                        let list: Vec<u32> = (0..count)
                            .filter_map(|i| read_u32(frame, p + 6 + i * 4))
                            .collect();
                        let plausible = list.iter().filter(|v| **v > 0 && **v < 3_000_000).count();
                        let list_end = p + 6 + count * 4;
                        let captain = read_u32(frame, list_end);
                        let vice = read_u32(frame, list_end + 4);
                        let pairs = list.len().saturating_sub(1).min(6);
                        let rising = list
                            .windows(2)
                            .take(pairs)
                            .filter(|w| w[0] < w[1])
                            .count();
                        let linked = captain.is_some_and(|v| list.contains(&v))
                            || vice.is_some_and(|v| list.contains(&v));
                        println!(
                            "      anchor at +{:<5} count {count:>3}  {plausible}/{count} plausible eids  {:?}",
                            p - off,
                            &list.iter().take(8).collect::<Vec<_>>()
                        );
                        println!(
                            "        rising {rising}/{pairs} (needs {})  captain {captain:?} vice {vice:?}  captain_linked {linked}",
                            pairs.saturating_sub(1)
                        );
                    }
                }
                p += 1;
            }
        }
        if mine.is_empty() {
            println!("  no head at all — either no squad is stored, or the head shape differs");
            println!("  club uid from the club table: {:?}", by_eid.get(&want));
        }
    }
}
