//! Field survey of stub-people bodies (`SAVE_FORMAT.md` §6d-bis). For every
//! squad-referenced stub, tries each byte offset in the body as a u32 and
//! measures how often it resolves in each name pool — the real name id, if
//! any, should resolve for nearly every stub. Also histograms the age-shaped
//! byte and the year field.
//!
//! ```text
//! cargo run --release --example stubfields -- <save.fm>
//! ```

use std::collections::HashMap;

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: stubfields <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;
    let Some(strings) = fm_save::strings::scan_strings(data) else {
        return Ok(());
    };

    println!("{} squad-referenced stubs", save.stubs.len());

    // Pool hit rate per candidate u32 offset within the stub body.
    let offsets: Vec<usize> = (20..34).collect();
    let mut fore = HashMap::new();
    let mut sur = HashMap::new();
    let mut distinct: HashMap<usize, std::collections::HashSet<u32>> = HashMap::new();
    let mut ages: HashMap<u8, usize> = HashMap::new();
    let mut years: HashMap<u16, usize> = HashMap::new();

    for s in &save.stubs {
        for &o in &offsets {
            let Some(v) = read_u32(data, s.offset + o) else { continue };
            if strings.forenames.contains_key(&v) {
                *fore.entry(o).or_insert(0usize) += 1;
            }
            if strings.surnames.contains_key(&v) {
                *sur.entry(o).or_insert(0usize) += 1;
            }
            distinct.entry(o).or_default().insert(v);
        }
        if let Some(&a) = data.get(s.offset + 28) {
            *ages.entry(a).or_insert(0) += 1;
        }
        // Year candidate: a u16 in 2020..2040 anywhere in +20..+28.
        for o in 20..27 {
            let Some(y) = data
                .get(s.offset + o..s.offset + o + 2)
                .and_then(|p| Some(u16::from_le_bytes(<[u8; 2]>::try_from(p).ok()?)))
            else {
                continue;
            };
            if (2020..2040).contains(&y) {
                *years.entry(y).or_insert(0) += 1;
                break;
            }
        }
    }

    let n = save.stubs.len().max(1);
    println!("\noffset  forename%  surname%  distinct");
    for &o in &offsets {
        println!(
            "  +{o:<4} {:8} {:9} {:9}",
            fore.get(&o).unwrap_or(&0) * 100 / n,
            sur.get(&o).unwrap_or(&0) * 100 / n,
            distinct.get(&o).map_or(0, std::collections::HashSet::len)
        );
    }

    let mut a: Vec<_> = ages.into_iter().collect();
    a.sort_unstable();
    println!("\n+28 byte histogram (top): {:?}", a.iter().take(30).collect::<Vec<_>>());
    let mut y: Vec<_> = years.into_iter().collect();
    y.sort_unstable();
    println!("year candidates in +20..+28: {y:?}");

    // Render +29 in each pool for a sample, with the referencing club — the
    // pool whose rendering matches the club's country flavour is the name.
    let club_names: HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();
    let club_by_member: HashMap<u32, u32> = save
        .squads
        .iter()
        .flat_map(|s| s.player_eids.iter().map(|&e| (e, s.club_eid)))
        .collect();
    println!("\nsample renderings of +29:");
    for s in save.stubs.iter().take(25) {
        let Some(v) = read_u32(data, s.offset + 29) else { continue };
        let club = club_by_member
            .get(&s.eid)
            .and_then(|c| club_names.get(c))
            .unwrap_or(&"?");
        println!(
            "  {club:24} id {v:>7}  fore={:?} sur={:?} common={:?}",
            strings.forenames.get(&v).map(String::as_str),
            strings.surnames.get(&v).map(String::as_str),
            strings.common_names.get(&v).map(String::as_str)
        );
    }
    Ok(())
}
