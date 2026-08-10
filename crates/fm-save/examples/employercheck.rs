//! Cross-validates the employer lookup against squad-list truth: for every
//! first-team-bound player with a contract anchor, does map[second-1] give
//! the same club? Disagreements should be loans (contract names the owner).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let frames = fm_save::container::read_frames(&bytes).expect("frames");
        let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
        let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
        let frame = &frames[index].data;
        let club_ids: Vec<(u32, u32)> = save.clubs.iter().filter_map(|c| Some((c.eid?, c.uid?))).collect();
        let ords = fm_save::squad::employer_ordinals(frame, &club_ids, &save.squads);
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().into_owned();

        let (mut agree, mut differ, mut unmapped) = (0usize, 0usize, 0usize);
        let mut samples = Vec::new();
        for p in &save.people {
            if !p.is_player() || p.compact { continue; }
            if p.squad_level != Some(fm_save::squad::SquadKind::FirstTeam) { continue; }
            let (Some(eid), Some(club)) = (p.eid, p.club_eid) else { continue };
            let n = eid.to_le_bytes();
            let lo = p.offset.saturating_sub(600);
            let Some(a) = (lo..p.offset.saturating_sub(4)).rev().find(|&i| {
                frame[i..i + 4] == n
                    && frame[i + 8..i + 12] == [0, 0, 0, 0]
                    && frame[i + 16] == 0x01
                    && frame[i + 18] == 0x00
                    && frame[i + 19..i + 23] == [0xFF; 4]
            }) else { continue };
            let second = u32::from_le_bytes(frame[a + 4..a + 8].try_into().unwrap());
            match second.checked_sub(1).and_then(|o| ords.get(&o)) {
                None => unmapped += 1,
                Some(&c) if c == club => agree += 1,
                Some(&c) => {
                    differ += 1;
                    if samples.len() < 8 {
                        let cn = |e: u32| save.clubs.iter().find(|x| x.eid == Some(e)).map_or("?", |x| x.short_name.as_str()).to_owned();
                        samples.push(format!("{} squad={} contract={}", p.full_name, cn(club), cn(c)));
                    }
                }
            }
        }
        println!("== {name}: agree {agree}  differ {differ}  unmapped {unmapped}");
        for s in samples { println!("   {s}"); }
    }
}
