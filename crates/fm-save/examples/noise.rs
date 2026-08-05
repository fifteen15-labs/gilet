//! Separates the person records that describe a footballing person from the
//! ones the scanner accepted by coincidence, and shows what tells them apart.
//!
//! ```text
//! cargo run --release --example noise -- <save.fm>
//! ```

// Research spike, not shipped code.
#![allow(clippy::indexing_slicing)]

// A diagnostic script reads better top to bottom than sliced up for the lint.
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: noise <save.fm>")?;
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;
    let today = save.game_date.ok_or("no game date")?;
    println!("save date {today:?}   people {}", save.people.len());

    // The highest nation identifier any club in this save carries. Clubs name
    // their country in the clear, so this is the save's own ceiling rather
    // than a number picked by hand.
    let club_max = save.clubs.iter().map(|c| c.nation_id).max().unwrap_or(0);
    println!("highest nation id on a club record: {club_max}");

    let mut buckets = [(0usize, 0usize, 0usize); 3]; // (count, with eid, with ability)
    for p in save.people.iter().filter(|p| !p.compact) {
        let Some(dob) = p.date_of_birth else { continue };
        let age = dob.age_on(today);
        let bucket = if age < 14 {
            0
        } else if age > 90 {
            2
        } else {
            1
        };
        buckets[bucket].0 += 1;
        buckets[bucket].1 += usize::from(p.eid.is_some());
        buckets[bucket].2 += usize::from(p.ability.is_some());
    }
    for (label, b) in ["under 14", "14 to 90", "over 90"].iter().zip(buckets) {
        println!("  {label:<9} {:>7} people, {:>7} with an eid, {:>7} with ability", b.0, b.1, b.2);
    }

    println!("\nnation identifier against age:");
    let mut wild_and_young = 0usize;
    let mut wild_but_sane_age = 0usize;
    let mut sane_but_young = 0usize;
    for p in save.people.iter().filter(|p| !p.compact) {
        let Some(dob) = p.date_of_birth else { continue };
        let age = dob.age_on(today);
        let plausible_age = (14..=90).contains(&age);
        let wild = p.nation_id.is_some_and(|n| u32::from(n) > club_max);
        match (wild, plausible_age) {
            (true, false) => wild_and_young += 1,
            (true, true) => wild_but_sane_age += 1,
            (false, false) => sane_but_young += 1,
            (false, true) => {}
        }
    }
    println!("  nation over the ceiling and age implausible: {wild_and_young}");
    println!("  nation over the ceiling, age plausible:      {wild_but_sane_age}");
    println!("  nation in range, age implausible:            {sane_but_young}");

    // What a nation-identifier ceiling would cost and buy.
    for bound in [255u16, 300, 512] {
        let mut dropped = 0usize;
        let mut dropped_with_eid = 0usize;
        let mut dropped_with_club = 0usize;
        let mut dropped_with_ability = 0usize;
        let mut odd_ages_left = 0usize;
        for p in save.people.iter().filter(|p| !p.compact) {
            let over = p.nation_id.is_some_and(|n| n > bound);
            if over {
                dropped += 1;
                dropped_with_eid += usize::from(p.eid.is_some());
                dropped_with_club += usize::from(p.club_eid.is_some());
                dropped_with_ability += usize::from(p.ability.is_some());
            } else if p.date_of_birth.is_some_and(|d| !(14..=95).contains(&d.age_on(today))) {
                odd_ages_left += 1;
            }
        }
        println!(
            "  nation ceiling {bound}: drops {dropped} (eid {dropped_with_eid}, club {dropped_with_club}, ability {dropped_with_ability}); implausible ages left {odd_ages_left}"
        );
    }
    let mut real_ids: Vec<u16> = save
        .people
        .iter()
        .filter_map(|p| p.nation_id)
        .filter(|n| *n <= 512)
        .collect();
    real_ids.sort_unstable();
    real_ids.dedup();
    println!("  highest nation id under 512 carried by a person: {:?}", real_ids.last());

    println!("\nthe outliers that do carry an entity id — the ones an age window would cost:");
    for p in save
        .people
        .iter()
        .filter(|p| !p.compact && p.eid.is_some())
        .filter(|p| p.date_of_birth.is_some_and(|d| !(14..=90).contains(&d.age_on(today))))
        .take(20)
    {
        println!(
            "  0x{:08x} {:<34} dob {:?} age {} nation {:?} club {:?} ability {}",
            p.offset,
            p.full_name,
            p.date_of_birth.map(|d| (d.year, d.month, d.day)),
            p.date_of_birth.map_or(0, |d| d.age_on(today)),
            p.nation_id,
            p.club_eid,
            p.ability.is_some()
        );
    }

    println!("\nwhere the implausible records sit — offset run:");
    let bad: Vec<usize> = save
        .people
        .iter()
        .filter(|p| !p.compact)
        .filter(|p| p.date_of_birth.is_some_and(|d| !(14..=90).contains(&d.age_on(today))))
        .map(|p| p.offset)
        .collect();
    let good_first = save
        .people
        .iter()
        .filter(|p| !p.compact && p.eid.is_some() && p.ability.is_some())
        .map(|p| p.offset)
        .min();
    println!(
        "  {} implausible, first at 0x{:08x}, last at 0x{:08x}; first scouted person at 0x{:08x?}",
        bad.len(),
        bad.first().copied().unwrap_or(0),
        bad.last().copied().unwrap_or(0),
        good_first.unwrap_or(0)
    );
    let after = bad.iter().filter(|o| Some(**o) > good_first).count();
    println!("  of those, {after} sit after the first scouted person");

    println!("\nrecords with an implausible age, in the raw:");
    for p in save
        .people
        .iter()
        .filter(|p| !p.compact)
        .filter(|p| p.date_of_birth.is_some_and(|d| !(14..=90).contains(&d.age_on(today))))
        .take(12)
    {
        println!(
            "  0x{:08x} {:<34} dob {:?} nation {:?} eid {:?} ability {}",
            p.offset,
            p.full_name,
            p.date_of_birth.map(|d| (d.year, d.month, d.day)),
            p.nation_id,
            p.eid,
            p.ability.is_some()
        );
    }

    println!("\nfor contrast, the same fields on people who scout normally:");
    for p in save
        .people
        .iter()
        .filter(|p| !p.compact && p.ability.is_some() && p.eid.is_some())
        .take(5)
    {
        println!(
            "  0x{:08x} {:<34} dob {:?} nation {:?}",
            p.offset,
            p.full_name,
            p.date_of_birth.map(|d| (d.year, d.month, d.day)),
            p.nation_id
        );
    }
    Ok(())
}
