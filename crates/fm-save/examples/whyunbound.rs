//! For named clubless people: contract team id, what the map says, and what
//! row carries that ordinal — why `link_employers` did not fire.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("save");
    let bytes = std::fs::read(&path).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
    let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
    let frame = &frames[index].data;
    let club_ids: Vec<(u32, u32)> = save.clubs.iter().filter_map(|c| Some((c.eid?, c.uid?))).collect();
    let ords = fm_save::squad::employer_ordinals(frame, &club_ids, &save.squads);

    for needle in args {
        let Some(p) = save.people.iter().find(|p| p.full_name.contains(&needle)) else {
            println!("{needle}: not found");
            continue;
        };
        let Some(team) = fm_save::person::contract_team_id(frame, p) else {
            println!("{}: no contract anchor", p.full_name);
            continue;
        };
        let hit = team.checked_sub(1).and_then(|o| ords.get(&o));
        let name = hit.and_then(|&c| save.clubs.iter().find(|x| x.eid == Some(c))).map(|c| c.short_name.as_str());
        println!("{}: team {team} -> map {hit:?} {name:?}", p.full_name);
        if hit.is_none() {
            // find rows carrying this ordinal
            let ord = team - 1;
            let mut at = 3usize;
            while at + 26 <= frame.len() {
                if frame.get(at + 4..at + 14) == Some(&[0u8; 10][..])
                    && frame.get(at.wrapping_sub(2)) == Some(&0xFF)
                    && read_u32(frame, at + 14) == Some(ord)
                {
                    let eid = read_u32(frame, at).unwrap();
                    let uid = read_u32(frame, at + 18).unwrap();
                    let cn = save.clubs.iter().find(|c| c.eid == Some(eid)).map(|c| c.short_name.as_str());
                    println!("   row 0x{at:08x} eid {eid} ({cn:?}) uid {uid} ty {:02x} flag {:02x}", frame[at - 3], frame[at - 1]);
                }
                at += 1;
            }
        }
    }
}
