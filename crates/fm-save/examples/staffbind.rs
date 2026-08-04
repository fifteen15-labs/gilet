//! Reports how many scanned staff blocks actually bind to people under the
//! `person eid == block eid + 1` rule, and what the unbound remainder looks
//! like — the numbers that decide whether the binding rule is the bottleneck.
//!
//! ```text
//! cargo run --release --example staffbind -- <save.fm> [eid ...]
//! ```

#[allow(clippy::too_many_lines)] // a diagnostic dump reads best top to bottom
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: staffbind <save.fm> [person eid ...]");
        std::process::exit(2);
    };
    let probes: Vec<u32> = args.filter_map(|a| a.parse().ok()).collect();

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let sheets = fm_save::staff::scan_staff(&main.data);
    let save = fm_save::Save::parse(&bytes)?;

    let person_eids: std::collections::HashSet<u32> =
        save.people.iter().filter_map(|p| p.eid).collect();
    let bound = sheets
        .iter()
        .filter(|s| person_eids.contains(&s.eid.saturating_add(1)))
        .count();
    let self_keyed = sheets.iter().filter(|s| person_eids.contains(&s.eid)).count();
    println!(
        "{} blocks scanned; {bound} have a person at eid+1, {self_keyed} at their own eid",
        sheets.len()
    );

    // The uid settles which pattern a block follows: an exact (eid, uid)
    // person match means the object IS that person's identity (their own
    // sheet); otherwise the person one eid up owns it as their second object.
    let pair_to_person: std::collections::HashMap<(u32, u32), usize> = save
        .people
        .iter()
        .enumerate()
        .filter_map(|(i, p)| Some(((p.eid?, p.uid?), i)))
        .collect();
    let exact = sheets
        .iter()
        .filter(|s| pair_to_person.contains_key(&(s.eid, s.uid)))
        .count();
    let exact_and_next = sheets
        .iter()
        .filter(|s| {
            pair_to_person.contains_key(&(s.eid, s.uid))
                && person_eids.contains(&s.eid.saturating_add(1))
        })
        .count();
    let neither = sheets
        .iter()
        .filter(|s| {
            !pair_to_person.contains_key(&(s.eid, s.uid))
                && !person_eids.contains(&s.eid.saturating_add(1))
        })
        .count();
    println!(
        "{exact} match a person's exact (eid, uid) — their own sheet; \
         {exact_and_next} of those ALSO have a person at eid+1 (misbound today); \
         {neither} match neither rule"
    );

    // Where do unbound blocks sit relative to the people table? If they live
    // inside person records, record-adjacency can attribute them.
    let mut offsets: Vec<(usize, usize)> = save
        .people
        .iter()
        .enumerate()
        .map(|(i, p)| (p.offset, i))
        .collect();
    offsets.sort_unstable();
    let starts: Vec<usize> = offsets.iter().map(|&(o, _)| o).collect();

    let mut adjacent_person = 0usize;
    for s in &sheets {
        if person_eids.contains(&s.eid.saturating_add(1)) {
            continue;
        }
        let idx = starts.partition_point(|&o| o <= s.offset);
        if let Some(&(_, i)) = idx.checked_sub(1).and_then(|i| offsets.get(i)) {
            if let Some(p) = save.people.get(i) {
                // Does the containing record's person lack a sheet?
                if p.staff.is_none() && p.eid.is_some() {
                    adjacent_person += 1;
                }
            }
        }
    }
    println!("unbound blocks sitting inside a sheetless person's record span: {adjacent_person}");

    for probe in probes {
        let person = save.people.iter().find(|p| p.eid == Some(probe));
        println!(
            "\nperson eid {probe}: {:?}",
            person.map(|p| (&p.full_name, p.offset, p.uid))
        );
        for s in &sheets {
            if s.eid == probe
                || s.eid.saturating_add(1) == probe
                || person.is_some_and(|p| p.uid == Some(s.uid))
            {
                println!(
                    "  block 0x{:x} eid {} uid {} ca/pa {}/{}",
                    s.offset, s.eid, s.uid, s.current_ability, s.potential_ability
                );
            }
        }
        // Nearest blocks by offset, either side of the person's record.
        if let Some(p) = person {
            let mut near: Vec<&fm_save::staff::Staff> = sheets.iter().collect();
            near.sort_by_key(|s| s.offset.abs_diff(p.offset));
            for s in near.iter().take(3) {
                println!(
                    "  nearest block 0x{:x} (d {}) eid {} uid {} ca/pa {}/{}",
                    s.offset,
                    s.offset.abs_diff(p.offset),
                    s.eid,
                    s.uid,
                    s.current_ability,
                    s.potential_ability
                );
            }
        }
    }
    Ok(())
}
