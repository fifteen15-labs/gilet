//! Prints `eid offset` for pure staff (sheet, no ability) still lacking a
//! club after the manager bind — the population whose employer is unfound.
//!
//! ```text
//! cargo run --release --example staffeids -- <save.fm> [limit]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: staffeids <save.fm> [limit]");
        std::process::exit(2);
    };
    let limit: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(usize::MAX);
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;
    for p in save
        .people
        .iter()
        .filter(|p| p.staff.is_some() && p.ability.is_none() && p.club_eid.is_none())
        .take(limit)
    {
        if let Some(eid) = p.eid {
            println!("{eid} {:#x}", p.offset);
        }
    }
    Ok(())
}
