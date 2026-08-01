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
        let d = p.date_of_birth;
        println!(
            "  {:<34} {:04}-{:02}-{:02}  age {}",
            p.full_name,
            d.year,
            d.month,
            d.day,
            d.age_on(today)
        );
    }

    for probe in [
        "Erling Braut Haaland",
        "Jude Victor William Bellingham",
        "Florian Richard Wirtz",
        "Bukayo Ayoyinka Saka",
        "Kylian Mbappé Lottin",
    ] {
        match save.people.iter().find(|p| p.full_name == probe) {
            Some(p) => {
                let d = p.date_of_birth;
                println!("  found {:<32} {:04}-{:02}-{:02}", p.full_name, d.year, d.month, d.day);
            }
            None => println!("  MISSING {probe}"),
        }
    }

    let nicknamed = save.people.iter().filter(|p| p.common_name_id.is_some()).count();
    println!("\nwith a nickname: {nicknamed}");

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
