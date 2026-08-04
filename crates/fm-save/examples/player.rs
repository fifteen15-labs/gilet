//! Prints one person's decoded record, including every attribute index.
//!
//! Used to check the parser against a screenshot of the same player in-game,
//! which is how attribute indices get named.
//!
//! ```text
//! cargo run --release --example player -- save.fm "Jamal Musiala"
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(query)) = (args.next(), args.next()) else {
        eprintln!("usage: player <save.fm> <name>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let needle = query.to_lowercase();
    let found: Vec<&fm_save::Person> = save
        .people
        .iter()
        .filter(|p| p.full_name.to_lowercase().contains(&needle))
        .collect();

    if found.is_empty() {
        println!("no person matching {query:?}");
        return Ok(());
    }

    let clubs: std::collections::HashMap<u32, &str> = save
        .clubs
        .iter()
        .filter_map(|c| Some((c.eid?, c.short_name.as_str())))
        .collect();

    for p in found {
        println!("\n=== {} ===", p.full_name);
        match p.date_of_birth {
            Some(d) => println!("born      {:04}-{:02}-{:02}", d.year, d.month, d.day),
            None => println!("born      ? (compact entry)"),
        }
        println!("nation    {} ({:?})", p.nation().unwrap_or("?"), p.nation_id);
        println!("eid/uid   {:?} / {:?}", p.eid, p.uid);
        println!(
            "club      {}",
            p.club_eid
                .and_then(|e| clubs.get(&e).copied())
                .unwrap_or("(none)")
        );

        if let Some(s) = &p.staff {
            println!(
                "staff obj 0x{:x}  eid {}  uid {}",
                s.offset, s.eid, s.uid
            );
            println!(
                "reputation home {} / current {} / world {}   CA/PA {} / {}",
                s.home_reputation,
                s.current_reputation,
                s.world_reputation,
                s.current_ability,
                s.potential_ability
            );
            for (i, v) in s.attributes.iter().enumerate() {
                if let Some(name) = fm_save::staff::attribute_name(i) {
                    println!("  {i:>2}: {v:>2}  {name}");
                }
            }
        }

        let Some(a) = &p.ability else {
            println!("no attribute block — staff");
            continue;
        };
        // The block sits in front of the name it belongs to; printing both
        // offsets is how a record that breaks that order gets spotted.
        println!(
            "record    0x{:x}   block 0x{:x}  (block is {} bytes earlier)",
            p.offset,
            a.block_offset,
            p.offset.saturating_sub(a.block_offset)
        );
        println!("CA/PA     {} / {}", a.current, a.potential);
        println!(
            "positions {}",
            a.natural_positions().join(", ")
        );
        println!("position ratings:");
        for (name, v) in fm_save::ability::POSITION_NAMES.iter().zip(a.positions) {
            print!("  {name} {v:>2}");
        }
        println!("\nattributes:");
        for (i, v) in a.attributes.iter().enumerate() {
            let label = fm_save::ability::attribute_name(i).unwrap_or("");
            let gk = if fm_save::ability::is_goalkeeping(i) { " [GK]" } else { "" };
            println!("  {i:>2}: {v:>2}  {label}{gk}");
        }
    }
    Ok(())
}
