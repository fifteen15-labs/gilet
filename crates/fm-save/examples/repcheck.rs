//! Reads the candidate reputation run that follows each person's own
//! identity block — `02 [u16 A][u16 B][u16 C][u16 D]` with the expected
//! invariant `D == B / 50` — and reports coverage plus the top of the table,
//! to test whether the field tracks quality when attributed to the record it
//! sits in (`OPEN_PROBLEMS` §3b: the earlier read was contaminated by an
//! unbounded search).
//!
//! ```text
//! cargo run --release --example repcheck -- <save.fm>
//! ```

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: repcheck <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let save = fm_save::Save::parse(&bytes)?;

    let mut offsets: Vec<usize> = save.people.iter().map(|p| p.offset).collect();
    offsets.sort_unstable();

    let mut with_block = 0usize;
    let mut with_tag = 0usize;
    let mut with_invariant = 0usize;
    let mut table: Vec<(u16, u16, u16, u16, usize)> = Vec::new();

    for (i, p) in save.people.iter().enumerate() {
        let (Some(eid), Some(uid)) = (p.eid, p.uid) else { continue };
        let next = offsets
            .iter()
            .find(|&&o| o > p.offset)
            .copied()
            .unwrap_or(main.data.len())
            .min(p.offset + 4000);
        let Some(h) = find_identity(&main.data, p.offset, next, eid, uid) else {
            continue;
        };
        with_block += 1;
        if main.data.get(h + 12) != Some(&0x02) {
            continue;
        }
        with_tag += 1;
        let (Some(a), Some(b), Some(c), Some(d)) = (
            read_u16(&main.data, h + 13),
            read_u16(&main.data, h + 15),
            read_u16(&main.data, h + 17),
            read_u16(&main.data, h + 19),
        ) else {
            continue;
        };
        if d == b / 50 && a <= 10_000 && b <= 10_000 && c <= 10_000 {
            with_invariant += 1;
            table.push((a, b, c, d, i));
        }
    }

    println!(
        "people {}   own identity found {with_block}   02-tag {with_tag}   invariant D==B/50 {with_invariant}",
        save.people.len()
    );

    table.sort_by_key(|&(a, _, _, _, _)| std::cmp::Reverse(a));
    println!("\ntop 25 by A:");
    for &(a, b, c, d, i) in table.iter().take(25) {
        let Some(p) = save.people.get(i) else { continue };
        let ca = p.ability.as_ref().map_or(0, |x| x.current);
        println!(
            "  A {a:>5}  B {b:>5}  C {c:>5}  D {d:>3}  CA {ca:>3}  {}",
            p.full_name
        );
    }

    // Sanity: A against CA for players.
    let players: Vec<&(u16, u16, u16, u16, usize)> = table
        .iter()
        .filter(|&&(_, _, _, _, i)| save.people.get(i).is_some_and(|p| p.ability.is_some()))
        .collect();
    let mut top: Vec<_> = players.clone();
    top.sort_by_key(|&&(a, _, _, _, _)| std::cmp::Reverse(a));
    top.truncate(200);
    let ca_sum: u32 = top
        .iter()
        .map(|&&(_, _, _, _, i)| {
            u32::from(save.people.get(i).and_then(|p| p.ability.as_ref()).map_or(0, |x| x.current))
        })
        .sum();
    let mean_ca = ca_sum / u32::try_from(top.len().max(1)).unwrap_or(1);
    println!("\nplayers with reading: {}   mean CA of top-200 by A: {mean_ca}", players.len());

    // Raw bytes after the identity block for known people, unfiltered — the
    // cross-save diff is what separates live values from static identity data.
    println!("\nraw post-identity bytes (identity block +12):");
    for probe in [
        "Erling Braut Haaland",
        "Bukayo Ayoyinka Saka",
        "Virgil van Dijk",
        "Jamal Musiala",
        "Caroline Graham Hansen",
        "Lucas Chevalier",
        "Matías Soulé Malvano",
        "Unai Emery",
        "Isaac Leckie",
        "Kylian Mbappé",
        "Jude Bellingham",
        "Mohamed Salah",
        "Yoane Wissa",
        "Bruno Guimarães",
        "Lamine Yamal",
        "Steven Darren Kirk",
    ] {
        let Some(p) = save.people.iter().find(|p| p.full_name.contains(probe)) else {
            println!("  {probe}: not in save");
            continue;
        };
        let (Some(eid), Some(uid)) = (p.eid, p.uid) else {
            println!("  {probe}: no identity");
            continue;
        };
        let next = offsets
            .iter()
            .find(|&&o| o > p.offset)
            .copied()
            .unwrap_or(main.data.len())
            .min(p.offset + 4000);
        let Some(h) = find_identity(&main.data, p.offset, next, eid, uid) else {
            println!("  {probe}: identity not found in record");
            continue;
        };
        let raw: Vec<String> = main
            .data
            .iter()
            .skip(h + 12)
            .take(48)
            .map(|b| format!("{b:02x}"))
            .collect();
        let ca = p.ability.as_ref().map_or(0, |x| x.current);
        println!("  {probe} (CA {ca}): {}", raw.join(" "));
    }
    Ok(())
}

/// First `[eid][uid][uid]` with a three-zero prefix inside `[from, to)`.
fn find_identity(frame: &[u8], from: usize, to: usize, eid: u32, uid: u32) -> Option<usize> {
    let needle = eid.to_le_bytes();
    let mut at = from;
    while at + 12 <= to {
        let window = frame.get(at..to)?;
        let pos = window.windows(4).position(|w| w == needle)? + at;
        at = pos + 1;
        let u1 = read_u32(frame, pos + 4)?;
        let u2 = read_u32(frame, pos + 8)?;
        if u1 == uid && u2 == uid && frame.get(pos.wrapping_sub(3)..pos) == Some(&[0, 0, 0][..]) {
            return Some(pos);
        }
    }
    None
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}
