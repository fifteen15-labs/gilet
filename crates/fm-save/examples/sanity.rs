//! One pass over a save reporting the three things a scout notices first: what
//! date the readers make of it, whether ages are plausible, and which nation
//! identifiers have no name.
//!
//! ```text
//! cargo run --release --example sanity -- <save.fm>
//! ```

use std::collections::HashMap;

#[allow(clippy::too_many_lines)] // a diagnostic script, top to bottom
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: sanity <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;

    println!("== date ==");
    if let Some(header) = frames.first() {
        let d = &header.data;
        let u16_at = |i: usize| -> u16 {
            u16::from_le_bytes([d.get(i).copied().unwrap_or(0), d.get(i + 1).copied().unwrap_or(0)])
        };
        println!("version {:?}", fm_save::gamedate::format_version(d));
        let raw = u16_at(50);
        let year = u16_at(52);
        println!("header[50] raw {raw} (0x{raw:04x})  masked {}  year {year}", raw & 0x01FF);
        println!("  plain  {:?}", fm_save::Date::from_day_of_year(raw, year));
        println!("  masked {:?}", fm_save::Date::from_day_of_year(raw & 0x01FF, year));
        println!("  find_wall_clock_date {:?}", fm_save::gamedate::find_wall_clock_date(d));
    }

    // The database frame by name, the way Save::parse finds it.
    let db = frames
        .last()
        .and_then(|f| fm_save::manifest::read_manifest(&f.data))
        .and_then(|members| {
            let i = fm_save::manifest::frame_index_of(&members, "game_db.dat")?;
            let plain = members.get(i).map(|m| m.plain)?;
            let frame = frames.get(i)?;
            (frame.data.len() as u64 == plain).then_some(frame)
        });
    match db {
        Some(frame) => {
            let d = &frame.data;
            let u16_at = |i: usize| -> u16 {
                u16::from_le_bytes([
                    d.get(i).copied().unwrap_or(0),
                    d.get(i + 1).copied().unwrap_or(0),
                ])
            };
            let raw = u16_at(0x2A);
            let year = u16_at(0x2C);
            println!(
                "game_db.dat {} bytes  [0x2A] raw {raw} masked {}  year {year}",
                d.len(),
                raw & 0x01FF
            );
            println!("  find_main_frame_date {:?}", fm_save::gamedate::find_main_frame_date(d));
        }
        None => println!("game_db.dat: not resolved through the manifest"),
    }

    let save = fm_save::Save::parse(&bytes)?;
    println!("  Save::parse game_date {:?}", save.game_date);
    println!("people {}  clubs {}  squads {}", save.people.len(), save.clubs.len(), save.squads.len());

    println!("\n== dates of birth ==");
    let mut by_year: HashMap<i32, usize> = HashMap::new();
    for p in &save.people {
        if let Some(d) = p.date_of_birth {
            *by_year.entry(i32::from(d.year)).or_default() += 1;
        }
    }
    let mut years: Vec<(i32, usize)> = by_year.into_iter().collect();
    years.sort_unstable();
    if let (Some(first), Some(last)) = (years.first(), years.last()) {
        println!("birth years {} .. {}", first.0, last.0);
    }
    for (y, n) in years.iter().rev().take(12) {
        println!("  {y}  {n}");
    }
    println!("  ...");
    for (y, n) in years.iter().take(8) {
        println!("  {y}  {n}");
    }

    // Age as the app computes it: the save's date if known, else the clock.
    let today = save.game_date.unwrap_or(fm_save::Date { year: 2026, month: 8, day: 5 });
    println!("\nages against {today:?}");
    let mut young: Vec<&fm_save::person::Person> = Vec::new();
    let mut old: Vec<&fm_save::person::Person> = Vec::new();
    for p in &save.people {
        let Some(d) = p.date_of_birth else { continue };
        let age = d.age_on(today);
        if age < 14 {
            young.push(p);
        } else if age > 75 {
            old.push(p);
        }
    }
    println!("under 14: {}   over 75: {}", young.len(), old.len());
    for p in young.iter().take(10) {
        show(p, today);
    }
    println!("  --");
    for p in old.iter().take(10) {
        show(p, today);
    }

    println!("\n== nations ==");
    let mut counts: HashMap<u16, usize> = HashMap::new();
    for p in &save.people {
        if let Some(n) = p.nation_id {
            *counts.entry(n).or_default() += 1;
        }
    }
    let mut unnamed: Vec<(u16, usize)> = counts
        .iter()
        .filter(|(id, _)| fm_save::person::nation_name(**id).is_none())
        .map(|(id, n)| (*id, *n))
        .collect();
    unnamed.sort_unstable_by_key(|(_, n)| std::cmp::Reverse(*n));
    let named: usize = counts
        .iter()
        .filter(|(id, _)| fm_save::person::nation_name(**id).is_some())
        .map(|(_, n)| *n)
        .sum();
    let total: usize = counts.values().sum();
    println!(
        "distinct ids {}  named {} ({named} people)  unnamed {} ({} people of {total})",
        counts.len(),
        counts.len() - unnamed.len(),
        unnamed.len(),
        total - named
    );
    println!("largest unnamed groups: best-known people, then the clubs that\ncarry the same identifier — club names are in the clear, so they name the\ncountry where a squad of players only suggests it.");
    for (id, n) in unnamed.iter().take(40) {
        let mut best: Vec<&fm_save::person::Person> = save
            .people
            .iter()
            .filter(|p| p.nation_id == Some(*id) && p.ability.is_some())
            .collect();
        best.sort_unstable_by_key(|p| std::cmp::Reverse(p.ability.as_ref().map_or(0, |a| a.current)));
        let best_known: Vec<String> = best
            .iter()
            .take(5)
            .map(|p| {
                format!("{} ({})", p.full_name, p.ability.as_ref().map_or(0, |a| a.current))
            })
            .collect();
        let clubs: Vec<String> = save
            .clubs
            .iter()
            .filter(|c| c.nation_id == u32::from(*id))
            .take(6)
            .map(|c| c.name.clone())
            .collect();
        println!("  id {id:<5} {n:>6} people");
        println!("    people: {}", best_known.join(", "));
        println!("    clubs:  {}", clubs.join(" | "));
    }
    Ok(())
}

fn show(p: &fm_save::person::Person, today: fm_save::Date) {
    let dob = p.date_of_birth.map_or_else(
        || "-".to_owned(),
        |d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
    );
    let age = p.date_of_birth.map_or(0, |d| d.age_on(today));
    let ca = p.ability.as_ref().map_or(0, |a| a.current);
    println!("  {:<34} {dob}  age {age:>4}  CA {ca:>3}  eid {:?}", p.full_name, p.eid);
}
