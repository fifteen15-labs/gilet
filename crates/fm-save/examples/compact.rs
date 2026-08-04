//! Surveys compact person entries — the `10 00 [forename id][surname id] 01`
//! plus entity-object layout aged saves use for people folded out of the
//! loaded game world (Kylian Mbappé in a 2035 save). Reports how many exist
//! and how they interact with squads and fully-parsed people.
//!
//! ```text
//! cargo run --release --example compact -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: compact <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let Some(table) = fm_save::strings::scan_strings(&main.data) else {
        eprintln!("no string table");
        return Ok(());
    };
    let entries = fm_save::person::scan_compact(&main.data, &table, table.end_offset);
    let save = fm_save::Save::parse(&bytes)?;
    println!("compact entries: {}", entries.len());

    let full_eids: std::collections::HashSet<u32> = save
        .people
        .iter()
        .filter(|p| !p.compact)
        .filter_map(|p| p.eid)
        .collect();
    let compact_eids: std::collections::HashSet<u32> =
        entries.iter().filter_map(|p| p.eid).collect();
    let clash = compact_eids.intersection(&full_eids).count();
    println!("  eids also claimed by a full record: {clash}");

    let referenced: std::collections::HashSet<u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().copied())
        .collect();
    let unresolved = referenced.iter().filter(|e| !full_eids.contains(e)).count();
    println!("squad-referenced eids: {}", referenced.len());
    println!("  unresolved by full records: {unresolved}");
    let ref_compact: Vec<u32> = referenced
        .iter()
        .copied()
        .filter(|e| compact_eids.contains(e))
        .collect();
    println!("  squad-referenced compact eids: {}", ref_compact.len());

    for eid in ref_compact.iter().take(10) {
        if let Some(p) = entries.iter().find(|p| p.eid == Some(*eid)) {
            println!("    eid {eid} uid {:?}: {}", p.uid, p.full_name);
        }
    }
    for p in &entries {
        if p.uid == Some(85_139_014) {
            println!("  Mbappé check: 0x{:x} eid {:?}: {}", p.offset, p.eid, p.full_name);
        }
    }
    Ok(())
}
