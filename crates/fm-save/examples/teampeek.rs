//! Prints the member names of squad-table rows of a given separator type —
//! the tool for naming what an unhandled type actually is.
//! usage: teampeek <save.fm> <type-hex> [max-rows]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
use std::collections::HashMap;

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let want_ty = u8::from_str_radix(args[2].trim_start_matches("0x"), 16).unwrap();
    let max_rows: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(6);

    let bytes = std::fs::read(path).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
    let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
    let frame = &frames[index].data;

    let club_name: HashMap<u32, &str> = save.clubs.iter().filter_map(|c| Some((c.eid?, c.short_name.as_str()))).collect();
    let by_eid: HashMap<u32, &fm_save::person::Person> = save.people.iter().filter_map(|p| Some((p.eid?, p))).collect();

    let mut heads: Vec<usize> = Vec::new();
    let mut rows: Vec<(usize, u8, u8, u32)> = Vec::new();
    let mut at = 3usize;
    while at + 26 <= frame.len() {
        if frame.get(at + 4..at + 14) != Some(&[0u8; 10][..]) { at += 1; continue; }
        if frame.get(at.wrapping_sub(2)) != Some(&0xFF) { at += 1; continue; }
        let (Some(eid), Some(_ord), Some(uid)) = (read_u32(frame, at), read_u32(frame, at + 14), read_u32(frame, at + 18)) else { at += 1; continue; };
        heads.push(at);
        if eid > 0 && eid < 3_000_000 && uid != 0 && uid != u32::MAX {
            rows.push((at, frame[at - 3], frame[at - 1], eid));
        }
        at += 26;
    }

    let mut shown = 0usize;
    for (i, &(off, ty, flag, eid)) in rows.iter().enumerate() {
        let _ = i;
        if ty != want_ty { continue; }
        let next = heads.partition_point(|&o| o <= off);
        let end = heads.get(next).copied().unwrap_or(frame.len()).min(off + 6000);
        let mut p = off + 26;
        let mut list: Vec<u32> = Vec::new();
        while p + 6 < end {
            if frame.get(p..p + 4) != Some(&[0xFF; 4][..]) { p += 1; continue; }
            let count = u16::from_le_bytes(frame[p + 4..p + 6].try_into().unwrap()) as usize;
            if !(1..=200).contains(&count) || p + 6 + count * 4 > end { p += 1; continue; }
            let eids: Vec<u32> = (0..count).filter_map(|j| read_u32(frame, p + 6 + j * 4)).collect();
            if eids.len() == count && eids.iter().all(|e| by_eid.contains_key(e)) { list = eids; }
            break;
        }
        if list.len() < 4 { continue; }
        let cn = club_name.get(&eid).copied().unwrap_or("?");
        let pre: Vec<String> = frame[off - 6..off].iter().map(|b| format!("{b:02x}")).collect();
        println!("club {cn}#{eid} flag {flag:02x} sep[{}] {} members:", pre.join(" "), list.len());
        for e in list.iter().take(10) {
            let p = by_eid[e];
            let age = p.date_of_birth.map_or("?".into(), |d| format!("{}", d.year));
            let their_club = p.club_eid.and_then(|c| club_name.get(&c)).copied().unwrap_or("-");
            println!("   {e:>8} {:<34} b.{age:<6} club={their_club} female={:?}", p.full_name, p.female);
        }
        shown += 1;
        if shown >= max_rows { break; }
    }
}
