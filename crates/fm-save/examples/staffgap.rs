//! Samples clubless staff by non-player CA, and tests whether their records
//! carry a contract-shaped anchor whose team id resolves in the ordinal map
//! — the question of whether employer binding can extend to staff.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
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

        let mut staff: Vec<_> = save
            .people
            .iter()
            .filter(|p| !p.is_player() && !p.compact && p.club_eid.is_none() && p.staff.is_some())
            .collect();
        staff.sort_by_key(|p| std::cmp::Reverse(p.staff.as_ref().map_or(0, |s| s.current_ability)));
        let mut with_anchor = 0usize;
        let mut resolving = 0usize;
        for p in &staff {
            if let Some(team) = fm_save::person::contract_team_id(frame, p) {
                with_anchor += 1;
                if team.checked_sub(1).is_some_and(|o| ords.contains_key(&o)) {
                    resolving += 1;
                }
            }
        }
        println!("== {name}: clubless staff with sheet {}  anchor {}  team-resolving {}", staff.len(), with_anchor, resolving);
        for p in staff.iter().take(8) {
            let team = fm_save::person::contract_team_id(frame, p);
            let club = team
                .and_then(|t| t.checked_sub(1))
                .and_then(|o| ords.get(&o))
                .and_then(|&c| save.clubs.iter().find(|x| x.eid == Some(c)))
                .map(|c| c.short_name.as_str());
            println!(
                "   CA {:?} {:<32} team {:?} -> {:?}",
                p.staff.as_ref().map(|s| s.current_ability),
                p.full_name,
                team,
                club
            );
        }
    }
}
