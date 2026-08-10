//! Distribution of the two bytes before the identity triple (at-7, at-6)
//! across every bound person, cross-tabbed with the gender bit — pins down
//! the type-byte enum and what the newgen variants look like.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
use std::collections::HashMap;
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let frames = fm_save::container::read_frames(&bytes).expect("frames");
        let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
        let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
        let frame = &frames[index].data;
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy();
        let mut counts: HashMap<(u8, u8), usize> = HashMap::new();
        for p in &save.people {
            if p.compact { continue; }
            let (Some(eid), Some(uid)) = (p.eid, p.uid) else { continue };
            let mut needle = Vec::new();
            needle.extend_from_slice(&eid.to_le_bytes());
            needle.extend_from_slice(&uid.to_le_bytes());
            needle.extend_from_slice(&uid.to_le_bytes());
            let Some(window) = frame.get(p.offset..p.offset + 2048) else { continue };
            let Some(pos) = window.windows(12).position(|w| w == needle) else { continue };
            let at = p.offset + pos;
            if at < 7 { continue; }
            *counts.entry((frame[at - 7], frame[at - 6])).or_default() += 1;
        }
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let show: Vec<String> = rows.iter().take(14).map(|((a, b), n)| format!("{a:02x} {b:02x}:{n}")).collect();
        println!("{name}: {}", show.join("  "));
    }
}
