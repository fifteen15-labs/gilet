//! Finds squad-table rows by ordinal and prints their key eid + type — the
//! check that an empty unmaterialised row still names its club.
//! usage: ordfind <save.fm> <ordinal>...
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
use std::collections::HashMap;
fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let want: Vec<u32> = args[2..].iter().map(|a| a.parse().unwrap()).collect();
    let bytes = std::fs::read(path).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
    let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
    let frame = &frames[index].data;
    let name_of: HashMap<u32, &str> = save.clubs.iter().filter_map(|c| Some((c.eid?, c.short_name.as_str()))).collect();

    let mut at = 3usize;
    while at + 26 <= frame.len() {
        if frame.get(at + 4..at + 14) != Some(&[0u8; 10][..]) { at += 1; continue; }
        if frame.get(at.wrapping_sub(2)) != Some(&0xFF) { at += 1; continue; }
        let (Some(eid), Some(ord), Some(uid)) = (read_u32(frame, at), read_u32(frame, at + 14), read_u32(frame, at + 18)) else { at += 1; continue; };
        if want.contains(&ord) && eid > 0 && eid < 3_000_000 && uid != 0 {
            let ty = frame[at - 3];
            let flag = frame[at - 1];
            println!(
                "ordinal {ord}: eid {eid} ({}) ty {ty:02x} flag {flag:02x} uid {uid} at 0x{at:x}",
                name_of.get(&eid).copied().unwrap_or("?")
            );
        }
        at += 26;
    }
}
