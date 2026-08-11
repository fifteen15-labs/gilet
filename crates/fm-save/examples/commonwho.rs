//! Prints people whose record carries a common-name id, with the pool's
//! rendering beside the composed forename + surname — to confirm the pool
//! holds display names ("Juanito") before the UI starts preferring them.
//!
//! ```text
//! cargo run --release --example commonwho -- <save.fm> [limit]
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: commonwho <save.fm> [limit]");
        std::process::exit(2);
    };
    let limit: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(30);

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let Some(table) = fm_save::strings::scan_strings(&main.data) else {
        eprintln!("no string table");
        return Ok(());
    };
    let people = fm_save::person::scan_people(&main.data, &table);

    let with = people.iter().filter(|p| p.common_name_id.is_some()).count();
    println!(
        "{} of {} people carry a common-name id; pool holds {} strings\n",
        with,
        people.len(),
        table.common_names.len()
    );

    let mut shown = 0usize;
    for p in &people {
        let Some(id) = p.common_name_id else { continue };
        let rendered = table.common_names.get(&id);
        println!(
            "{:40} common id {:8} -> {}",
            p.full_name,
            id,
            rendered.map_or("(unresolved)", |s| s.as_str())
        );
        shown += 1;
        if shown >= limit {
            break;
        }
    }
    Ok(())
}
