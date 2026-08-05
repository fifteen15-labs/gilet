//! Prints named people's non-player sheet values at chosen slots — for
//! pinning the editor's unnamed rows against banked editor sheets.
//!
//! ```text
//! cargo run --release --example slotcheck -- <save.fm> 45 46 -- "Nikolić" "Fradley"
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: slotcheck <save.fm> <slot>... -- <name substring>...");
        std::process::exit(2);
    };
    let rest: Vec<String> = args.collect();
    let split = rest.iter().position(|a| a == "--").unwrap_or(rest.len());
    let slots: Vec<usize> = rest.iter().take(split).filter_map(|a| a.parse().ok()).collect();
    let needles: Vec<String> = rest.iter().skip(split + 1).map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;
    for p in &save.people {
        let name = p.full_name.to_lowercase();
        if !needles.iter().any(|n| name.contains(n.as_str())) {
            continue;
        }
        let Some(s) = &p.staff else {
            println!("{}: no sheet", p.full_name);
            continue;
        };
        let vals: Vec<String> = slots
            .iter()
            .map(|&i| format!("s{i}={}", s.attributes.get(i).copied().unwrap_or(0)))
            .collect();
        println!("{}: {}", p.full_name, vals.join(" "));
    }
    Ok(())
}
