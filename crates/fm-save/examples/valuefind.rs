//! Checks Haaland's decoded personality against his editor sheet and hunts
//! his editor Transfer Value (£120,000,000) near his record in every
//! plausible encoding (`OPEN_PROBLEMS` §4).
//!
//! ```text
//! cargo run --release --example valuefind -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: valuefind <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;
    let Some(p) = save
        .people
        .iter()
        .find(|p| p.full_name.contains("Erling Braut Haaland"))
    else {
        return Ok(());
    };
    println!("personality {:?}", p.personality);
    println!(
        "ambition {:?} loyalty {:?} pressure {:?} professionalism {:?} sportsmanship {:?} temperament {:?} controversy {:?}",
        p.ambition(), p.loyalty(), p.pressure(), p.professionalism(),
        p.sportsmanship(), p.temperament(), p.controversy()
    );

    // Who owns each raw-120M hit: the nearest preceding person record.
    let mut people: Vec<(usize, &str)> =
        save.people.iter().map(|q| (q.offset, q.full_name.as_str())).collect();
    people.sort_unstable_by_key(|(o, _)| *o);
    let offsets: Vec<usize> = people.iter().map(|(o, _)| *o).collect();
    let pat = 120_000_000u32.to_le_bytes();
    for (i, w) in data.windows(4).enumerate() {
        if w != pat {
            continue;
        }
        let idx = offsets.partition_point(|&o| o <= i);
        let owner = idx
            .checked_sub(1)
            .and_then(|j| people.get(j))
            .map_or(("<none>", 0), |(o, n)| (*n, i - o));
        println!("120M @0x{i:x} nearest record: {} (+{})", owner.0, owner.1);
    }
    Ok(())
}
