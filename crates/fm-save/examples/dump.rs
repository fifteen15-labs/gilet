//! Reads a save and prints what was parsed. Development aid for checking the
//! reader against real files:
//!
//! ```text
//! cargo run --release --example dump -- ~/Library/.../Career.fm
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: dump <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let started = std::time::Instant::now();
    let save = fm_save::Save::parse(&bytes)?;
    let elapsed = started.elapsed();

    let total: usize = save.frame_sizes.iter().sum();
    println!("file        {} bytes", bytes.len());
    println!("frames      {}", save.frame_sizes.len());
    println!("decompressed {total} bytes");
    println!("people      {}", save.people.len());
    println!("parsed in   {elapsed:.2?}");

    let today = fm_save::Date { year: 2026, month: 8, day: 1 };
    println!("\nfirst 10 people:");
    for p in save.people.iter().take(10) {
        match p.date_of_birth {
            Some(d) => println!(
                "  {:<34} {:04}-{:02}-{:02}  age {}",
                p.full_name,
                d.year,
                d.month,
                d.day,
                d.age_on(today)
            ),
            None => println!("  {:<34} compact (no birth date)", p.full_name),
        }
    }

    println!();
    for probe in [
        "Erling Braut Haaland",
        "Jude Victor William Bellingham",
        "Florian Richard Wirtz",
        "Bukayo Ayoyinka Saka",
        "Kylian Mbappé Lottin",
    ] {
        match save.people.iter().find(|p| p.full_name == probe) {
            Some(p) => {
                let born = p.date_of_birth.map_or_else(
                    || "compact".to_owned(),
                    |d| format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
                );
                let ability = p.ability.as_ref().map_or_else(
                    || "staff (no attribute block)".to_owned(),
                    |a| format!("CA {} / PA {}  {}", a.current, a.potential, a.natural_positions().join(", ")),
                );
                println!("  {:<32} {born:>10}  {ability}", p.full_name);
            }
            None => println!("  MISSING {probe}"),
        }
    }

    for who in ["Erling Braut Haaland", "Alisson Ramsés Becker", "Rúben Santos Gato Alves Dias"] {
        if let Some(p) = save.people.iter().find(|p| p.full_name == who) {
            if let Some(a) = p.ability.as_ref() {
                let named: Vec<String> = (0..fm_save::ability::ATTRIBUTE_COUNT)
                    .filter_map(|i| {
                        fm_save::ability::attribute_name(i)
                            .zip(a.attributes.get(i))
                            .map(|(n, v)| format!("{n} {v}"))
                    })
                    .collect();
                println!("\n{who}\n   {}", named.join("  ·  "));
            }
        }
    }

    let players = save.people.iter().filter(|p| p.is_player()).count();
    println!("\nplayers with ability: {players}   staff: {}", save.people.len() - players);

    let mut best: Vec<_> = save.people.iter().filter_map(|p| p.ability.as_ref().map(|a| (a, p))).collect();
    best.sort_by_key(|(a, _)| std::cmp::Reverse(a.potential));
    println!("\nhighest potential in the save:");
    for (a, p) in best.iter().take(8) {
        let age = p
            .date_of_birth
            .map_or("?".to_owned(), |d| d.age_on(today).to_string());
        println!(
            "  PA {:>3}  CA {:>3}  age {age:>2}  {:<12} {}",
            a.potential, a.current,
            a.natural_positions().join(","), p.full_name
        );
    }

    println!("\nclubs      {}", save.clubs.len());
    for c in save.clubs.iter().take(8) {
        println!("  {:<38} {:<18} id {:<6} nation {}", c.name, c.short_name, c.club_id, c.nation_id);
    }
    for probe in ["Manchester City", "Arsenal", "Borussia Dortmund"] {
        match save.clubs.iter().find(|c| c.name == probe) {
            Some(c) => println!("  found {:<20} short {:<18} id {}", c.name, c.short_name, c.club_id),
            None => println!("  MISSING {probe}"),
        }
    }
    Ok(())
}
