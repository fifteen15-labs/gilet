//! Census of separator type bytes in the squad table, spine-validated: walks
//! the same `[type] FF [flag]` heads `scan_team_squads` does, keeps the longest
//! ordinal-ascending run (the table itself), and tallies types with member
//! counts and sample clubs — the map of what the parser handles and what it
//! still ignores.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
use std::collections::HashMap;

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn lis(vals: &[u32]) -> Vec<usize> {
    let mut tails: Vec<u32> = Vec::new();
    let mut tidx: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; vals.len()];
    for (i, &v) in vals.iter().enumerate() {
        let k = tails.partition_point(|&e| e < v);
        prev[i] = k.checked_sub(1).and_then(|j| tidx.get(j).copied());
        if k == tails.len() {
            tails.push(v);
            tidx.push(i);
        } else if v < tails[k] {
            tails[k] = v;
            tidx[k] = i;
        }
    }
    let mut out = Vec::new();
    let mut cur = tidx.last().copied();
    while let Some(i) = cur {
        out.push(i);
        cur = prev[i];
    }
    out.reverse();
    out
}

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let frames = fm_save::container::read_frames(&bytes).expect("frames");
        let members = fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
        let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
        let frame = &frames[index].data;
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().into_owned();
        println!("== {name}");

        let club_name: HashMap<u32, &str> = save
            .clubs
            .iter()
            .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
            .collect();
        let person_eids: std::collections::HashSet<u32> =
            save.people.iter().filter_map(|p| p.eid).collect();

        // Heads, as scan_team_squads sees them.
        let mut heads: Vec<usize> = Vec::new();
        let mut rows: Vec<(usize, u8, u8, u32, u32)> = Vec::new(); // off, ty, flag, eid, ordinal
        let mut at = 3usize;
        while at + 26 <= frame.len() {
            if frame.get(at + 4..at + 14) != Some(&[0u8; 10][..]) {
                at += 1;
                continue;
            }
            if frame.get(at.wrapping_sub(2)) != Some(&0xFF) {
                at += 1;
                continue;
            }
            let (Some(eid), Some(ordinal), Some(uid)) =
                (read_u32(frame, at), read_u32(frame, at + 14), read_u32(frame, at + 18))
            else {
                at += 1;
                continue;
            };
            heads.push(at);
            if eid > 0 && eid < 3_000_000 && uid != 0 && uid != u32::MAX {
                rows.push((at, frame[at - 3], frame[at - 1], eid, ordinal));
            }
            at += 26;
        }
        let ords: Vec<u32> = rows.iter().map(|r| r.4).collect();
        let spine = lis(&ords);
        println!("  heads {}  plausible rows {}  spine {}", heads.len(), rows.len(), spine.len());

        let mut tally: HashMap<u8, (usize, usize, usize, Vec<String>)> = HashMap::new();
        for &i in &spine {
            let (off, ty, _flag, eid, _) = rows[i];
            let next = heads.partition_point(|&o| o <= off);
            let end = heads.get(next).copied().unwrap_or(frame.len()).min(off + 6000);
            // members: the real list shape — FF FF FF FF, u16 count, eids
            let mut n = 0usize;
            let mut p = off + 26;
            while p + 6 < end {
                if frame.get(p..p + 4) != Some(&[0xFF; 4][..]) {
                    p += 1;
                    continue;
                }
                let count = u16::from_le_bytes(frame[p + 4..p + 6].try_into().unwrap()) as usize;
                if !(1..=200).contains(&count) || p + 6 + count * 4 > end {
                    p += 1;
                    continue;
                }
                let eids: Vec<u32> = (0..count)
                    .filter_map(|i| read_u32(frame, p + 6 + i * 4))
                    .collect();
                if eids.len() == count && eids.iter().all(|e| person_eids.contains(e)) {
                    n = count;
                }
                break;
            }
            let e = tally.entry(ty).or_default();
            e.0 += 1;
            e.1 += n;
            e.2 += usize::from(n > 0);
            if n > 0 && e.3.len() < 5 {
                let cn = club_name.get(&eid).copied().unwrap_or("?");
                e.3.push(format!("{cn}#{eid}({n})"));
            }
        }
        let mut out: Vec<_> = tally.into_iter().collect();
        out.sort_by_key(|(_, (_, m, _, _))| std::cmp::Reverse(*m));
        for (ty, (recs, members, nonempty, ex)) in out {
            println!(
                "  ty {ty:02x}: {recs:>5} rows ({nonempty:>5} non-empty)  {members:>6} members  {}",
                ex.join(" ")
            );
        }
    }
}
