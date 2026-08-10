//! Validates the identity-7 gender bit against the squad-is-single-gender
//! fact, save-wide: counts squads whose resolved members mix bit values,
//! and dumps the pre-identity bytes of one woman and one man for layout.
//!
//! ```text
//! cargo run --release --example gendersquad -- <save.fm>...
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss, clippy::indexing_slicing)]

use std::collections::HashMap;

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let frames = fm_save::container::read_frames(&bytes).expect("frames");
        let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
        let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
        let frame = &frames[index].data;

        let name = std::path::Path::new(&path)
            .file_stem()
            .map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());

        let mut bit_of: HashMap<u32, bool> = HashMap::new();
        for p in &save.people {
            if p.compact { continue; }
            let (Some(eid), Some(uid)) = (p.eid, p.uid) else { continue };
            let mut needle = Vec::with_capacity(12);
            needle.extend_from_slice(&eid.to_le_bytes());
            needle.extend_from_slice(&uid.to_le_bytes());
            needle.extend_from_slice(&uid.to_le_bytes());
            let Some(window) = frame.get(p.offset..p.offset + 2048) else { continue };
            let Some(pos) = window.windows(12).position(|w| w == needle) else { continue };
            let at = p.offset + pos;
            bit_of.insert(eid, frame[at - 7] & 0x10 != 0);
        }

        let mut clean = 0usize;
        let mut mixed = 0usize;
        let mut mixed_examples: Vec<String> = Vec::new();
        for squad in &save.squads {
            let verdicts: Vec<bool> = squad.player_eids.iter().filter_map(|e| bit_of.get(e).copied()).collect();
            if verdicts.len() < 5 { continue; }
            let women = verdicts.iter().filter(|v| **v).count();
            if women == 0 || women == verdicts.len() {
                clean += 1;
            } else {
                mixed += 1;
                if mixed_examples.len() < 5 {
                    let club = save.clubs.iter().find(|c| c.eid == Some(squad.club_eid))
                        .map_or_else(|| format!("club {}", squad.club_eid), |c| c.short_name.clone());
                    mixed_examples.push(format!("{club} ({women}/{} women)", verdicts.len()));
                }
            }
        }
        println!("{name}: squads clean {clean}  mixed {mixed}  {}", mixed_examples.join(", "));
    }
}
