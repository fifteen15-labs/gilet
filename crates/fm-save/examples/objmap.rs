//! Maps entity objects per person record: every `00 00 00 [eid][uid][uid]`
//! block inside each record's span, to establish how many objects a person
//! carries and how their eids interleave (`OPEN_PROBLEMS` §3b).
//!
//! ```text
//! cargo run --release --example objmap -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: objmap <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let save = fm_save::Save::parse(&bytes)?;

    let mut order: Vec<usize> = (0..save.people.len()).collect();
    order.sort_by_key(|&i| save.people.get(i).map_or(0, |p| p.offset));

    let mut histogram: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut consecutive_pairs = 0usize;
    let mut pair_total = 0usize;

    for (pos, &i) in order.iter().enumerate() {
        let Some(p) = save.people.get(i) else { continue };
        let end = order
            .get(pos + 1)
            .and_then(|&j| save.people.get(j))
            .map_or(main.data.len(), |q| q.offset);
        let blocks = identity_blocks(&main.data, p.offset, end);
        *histogram.entry(blocks.len()).or_insert(0) += 1;
        if let [a, b] = blocks.as_slice() {
            pair_total += 1;
            if b.1 == a.1 + 1 {
                consecutive_pairs += 1;
            }
        }
    }

    println!("identity-shaped blocks per record:");
    for (n, count) in &histogram {
        println!("  {n} blocks: {count}");
    }
    println!("\ntwo-block records with consecutive eids: {consecutive_pairs} of {pair_total}");

    // A few records in full, from the middle of the table.
    println!("\nsample records:");
    for &i in order.iter().skip(20_000).take(8) {
        let Some(p) = save.people.get(i) else { continue };
        let pos = order.iter().position(|&j| j == i).unwrap_or(0);
        let end = order
            .get(pos + 1)
            .and_then(|&j| save.people.get(j))
            .map_or(main.data.len(), |q| q.offset);
        let blocks = identity_blocks(&main.data, p.offset, end);
        let ca = p.ability.as_ref().map_or(0, |a| a.current);
        let desc: Vec<String> = blocks
            .iter()
            .map(|&(o, e, u)| format!("eid {e} uid {u} @+{}", o - p.offset))
            .collect();
        println!("  {} (CA {ca}, span {}): {}", p.full_name, end - p.offset, desc.join("  |  "));
    }
    Ok(())
}

/// All `00 00 00 [eid][uid][uid]` blocks in `[from, to)` with plausible ids.
fn identity_blocks(frame: &[u8], from: usize, to: usize) -> Vec<(usize, u32, u32)> {
    let mut out = Vec::new();
    let mut at = from;
    while at + 15 <= to {
        if frame.get(at..at + 3) != Some(&[0, 0, 0][..]) {
            at += 1;
            continue;
        }
        let (Some(eid), Some(u1), Some(u2)) = (
            read_u32(frame, at + 3),
            read_u32(frame, at + 7),
            read_u32(frame, at + 11),
        ) else {
            break;
        };
        if eid > 0 && eid < 3_000_000 && u1 == u2 && u1 > eid && u1 != u32::MAX {
            out.push((at + 3, eid, u1));
            at += 15;
        } else {
            at += 1;
        }
    }
    out
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}
