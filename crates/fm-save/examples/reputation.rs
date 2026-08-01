//! Probes the reputation candidate: the `01/02 [u16 ×3] [u16 pair]` run that
//! follows a person's **second** identity block (`SAVE_FORMAT.md` §3).
//!
//! `OPEN_PROBLEMS.md` §3b rejected it on three people whose in-game badges
//! disagreed with their values. That is a five-person test. This runs the
//! population test instead: if a field is reputation, its top of the table has
//! to be household names, because in a freshly started save the most
//! reputable people in the world are not in dispute.
//!
//! ```text
//! cargo run --release --example reputation -- save.fm
//! ```

// Research spike, not shipped code.
#![allow(clippy::indexing_slicing, clippy::cast_precision_loss, clippy::too_many_lines)]

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

/// Finds the second identity block: `[eid+1][uid][uid]` at or after the
/// person's record offset, and returns the offset just past it.
fn second_identity(frame: &[u8], from: usize, eid: u32, window: usize) -> Option<usize> {
    let end = (from + window).min(frame.len().saturating_sub(12));
    let wanted = (eid + 1).to_le_bytes();
    let mut at = from;
    while at < end {
        let found = frame[at..end].windows(4).position(|w| w == wanted)?;
        let p = at + found;
        let uid = u32_at(frame, p + 4)?;
        if uid != 0 && Some(uid) == u32_at(frame, p + 8) {
            return Some(p + 12);
        }
        at = p + 1;
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: reputation <save.fm>")?;
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let frame = frames
        .iter()
        .max_by_key(|f| f.data.len())
        .ok_or("no frames")?
        .data
        .as_slice();

    // Hex dump around the second identity block for people whose in-game
    // reputation is not in dispute, so the layout can be read rather than
    // guessed at.
    let probes = ["Erling Braut Haaland", "Bukayo Ayoyinka Saka", "Virgil van Dijk"];
    for want in probes {
        let Some(person) = save.people.iter().find(|p| p.full_name == want) else {
            continue;
        };
        let Some(eid) = person.eid else { continue };
        let Some(at) = second_identity(frame, person.offset, eid, 4_000) else {
            println!("{want}: no second identity block\n");
            continue;
        };
        println!("=== {want} (eid {eid}, record at {}, second block ends {at}) ===", person.offset);
        for row in 0..6 {
            let from = at + row * 16;
            let Some(slice) = frame.get(from..from + 16) else { break };
            let hex: Vec<String> = slice.iter().map(|b| format!("{b:02x}")).collect();
            let u16s: Vec<String> = (0..8)
                .filter_map(|i| u16_at(frame, from + i * 2))
                .map(|v| format!("{v:>5}"))
                .collect();
            println!("  +{:>3}  {}   {}", row * 16, hex.join(" "), u16s.join(" "));
        }
        println!();
    }

    // Five candidate slots: the marker byte, three u16s, then the pair.
    let mut fields: Vec<Vec<(String, u16, Option<u8>)>> = vec![Vec::new(); 5];
    let mut found = 0usize;

    // Records are variable-length, so bound each search by where the next
    // person's record starts. Without this the `[eid+1]` search runs past the
    // end of the record and finds the *next person's* identity block, because
    // consecutive people carry consecutive entity ids — which silently
    // attributes each person's neighbour's numbers to them.
    let mut ordered: Vec<&fm_save::person::Person> = save.people.iter().collect();
    ordered.sort_by_key(|p| p.offset);
    let next_offset: std::collections::HashMap<usize, usize> = ordered
        .windows(2)
        .map(|w| (w[0].offset, w[1].offset))
        .collect();

    for person in &save.people {
        let Some(eid) = person.eid else { continue };
        let limit = next_offset
            .get(&person.offset)
            .map_or(4_000, |next| next.saturating_sub(person.offset));
        let Some(at) = second_identity(frame, person.offset, eid, limit) else {
            continue;
        };
        // Skip the 01/02 marker byte, then read five u16s.
        let base = at + 1;
        let mut values = Vec::new();
        for slot in 0..5 {
            match u16_at(frame, base + slot * 2) {
                Some(v) => values.push(v),
                None => break,
            }
        }
        if values.len() < 5 {
            continue;
        }
        // Reject anything that does not look like the run: the observed shape
        // is three values on a 1-10000 scale where the first two are within
        // one of each other, and the fourth is the first divided by fifty.
        // Without this the search lands in the FF run in front of the second
        // attribute block and every slot reads 65535.
        let (a, b, d) = (values[0], values[1], values[3]);
        if a == 0 || a > 10_000 || b > 10_000 || values[2] > 10_000 {
            continue;
        }
        if a.abs_diff(b) > 1 || d != a / 50 {
            continue;
        }
        found += 1;
        let ca = person.ability.as_ref().map(|a| a.current);
        for (slot, value) in values.iter().enumerate() {
            fields[slot].push((person.full_name.clone(), *value, ca));
        }
    }

    println!("second identity block located for {found} of {} people\n", save.people.len());

    // A reputation field should put the famous at the top. Thirteen names that
    // are unarguably world-famous in a save starting in 2025.
    let famous: Vec<&str> = vec![
        "Kylian", "Haaland", "Bellingham", "Vinícius", "Salah", "Mbappé", "Rodri", "Lewandowski",
        "Modrić", "Kane", "Saka", "Yamal", "Musiala",
    ];

    for (slot, mut rows) in fields.into_iter().enumerate() {
        if rows.is_empty() {
            continue;
        }
        rows.sort_by_key(|(_, v, _)| std::cmp::Reverse(*v));
        let top500: Vec<&String> = rows.iter().take(500).map(|(n, _, _)| n).collect();
        let hits = famous
            .iter()
            .filter(|f| top500.iter().any(|n| n.contains(**f)))
            .count();

        let with_ca: Vec<f64> = rows
            .iter()
            .take(200)
            .filter_map(|(_, _, ca)| ca.map(f64::from))
            .collect();
        let mean_ca = if with_ca.is_empty() {
            0.0
        } else {
            with_ca.iter().sum::<f64>() / with_ca.len() as f64
        };

        println!(
            "--- slot {slot}: range {}..{}, {hits}/13 famous in top 500, top-200 mean CA {mean_ca:.0} ({} of 200 are players)",
            rows.last().map_or(0, |r| r.1),
            rows[0].1,
            with_ca.len()
        );
        for (name, value, ca) in rows.iter().take(12) {
            println!("    {value:>6}  CA {:>4}  {name}", ca.map_or(-1, i32::from));
        }
    }

    // What the badge ground truth reads, for the three the doc names.
    Ok(())
}
