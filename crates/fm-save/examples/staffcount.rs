//! Counts people carrying a non-player sheet, and names a few — for checking
//! that staff binding holds on a given save before blaming the UI.
//!
//! ```text
//! cargo run --release --example staffcount -- <save.fm> [name substring...]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: staffcount <save.fm> [name substring...]");
        std::process::exit(2);
    };
    let needles: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let with = save.people.iter().filter(|p| p.staff.is_some()).count();
    let pure_staff = save
        .people
        .iter()
        .filter(|p| p.staff.is_some() && p.ability.is_none())
        .count();
    let staff_no_sheet = save
        .people
        .iter()
        .filter(|p| !p.compact && p.ability.is_none() && p.staff.is_none())
        .count();
    println!("{} people, {with} with a staff sheet ({pure_staff} pure staff)", save.people.len());
    println!("{staff_no_sheet} non-players carry no sheet at all");
    let employed = save
        .people
        .iter()
        .filter(|p| p.staff.is_some() && p.ability.is_none() && p.club_eid.is_some())
        .count();
    println!("{employed} pure staff carry an employer");

    let club_names: std::collections::HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();
    for p in &save.people {
        let name = p.full_name.to_lowercase();
        if needles.iter().any(|n| name.contains(n.as_str())) {
            let club = p
                .club_eid
                .and_then(|e| club_names.get(&e).copied())
                .unwrap_or("-");
            println!(
                "0x{:x}  eid={:?}  staff={}  ability={}  club={club}  {}",
                p.offset,
                p.eid,
                p.staff.is_some(),
                p.ability.is_some(),
                p.full_name
            );
        }
    }
    Ok(())
}
