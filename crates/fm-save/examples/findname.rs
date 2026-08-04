//! Finds parsed people whose full name contains a substring, and prints the
//! record offset plus the ids needed to look the person up elsewhere.
//!
//! ```text
//! cargo run --release --example findname -- <save.fm> Fradley Lutz
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: findname <save.fm> <substring>...");
        std::process::exit(2);
    };
    let needles: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    for person in &save.people {
        let name = person.full_name.to_lowercase();
        if needles.iter().any(|n| name.contains(n.as_str())) {
            println!(
                "0x{:x}  eid={:?} uid={:?}  {}",
                person.offset, person.eid, person.uid, person.full_name
            );
        }
    }
    Ok(())
}
