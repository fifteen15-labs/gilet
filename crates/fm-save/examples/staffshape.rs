//! Cross-tabs bound staff sheets by shape: the editor's tendency half is
//! 1-20, so a bound sheet whose low slots exceed 20 is not an editor-shaped
//! sheet — generated filler, or a different record class (match officials)
//! read through the staff lens. Splits by uid band and ability to see who
//! is who.
//!
//! ```text
//! cargo run --release --example staffshape -- <save.fm> [name substring...]
//! ```

const GENERATED_BAND: u32 = 2_000_000_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: staffshape <save.fm> [name substring...]");
        std::process::exit(2);
    };
    let needles: Vec<String> = args.map(|a| a.to_lowercase()).collect();

    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let mut counts = std::collections::BTreeMap::new();
    for p in &save.people {
        let Some(s) = &p.staff else { continue };
        let tail_max = s.attributes.iter().take(26).copied().max().unwrap_or(0);
        let shaped = tail_max <= 20;
        let generated = p.uid.is_some_and(|u| u >= GENERATED_BAND);
        let elite = s.current_ability >= 170;
        *counts.entry((generated, shaped, elite)).or_insert(0usize) += 1;
    }
    println!("(generated band, editor-shaped, CA>=170) -> count");
    for ((g, s, e), n) in &counts {
        println!("  gen={g} shaped={s} elite={e}: {n}");
    }

    for p in &save.people {
        let name = p.full_name.to_lowercase();
        if !needles.iter().any(|q| name.contains(q.as_str())) {
            continue;
        }
        let Some(s) = &p.staff else {
            println!("{}: no sheet", p.full_name);
            continue;
        };
        let tail_max = s.attributes.iter().take(26).copied().max().unwrap_or(0);
        println!(
            "{}: ca/pa {}/{} reps {}/{}/{} tendency-max {} uid {:?}",
            p.full_name,
            s.current_ability,
            s.potential_ability,
            s.home_reputation,
            s.current_reputation,
            s.world_reputation,
            tail_max,
            p.uid
        );
    }
    Ok(())
}
