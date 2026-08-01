//! Hunts for the player attribute table.
//!
//! FM's technical attributes are 1-20 and stored contiguously, so a long run of
//! bytes all within 1..=20 is a strong signature — far stronger than anything
//! offset-based, and it does not assume the table sits near the person record.
//!
//! ```text
//! cargo run --release --example findattrs -- save.fm
//! ```

// Research spike, not shipped code: a one-off analysis tool kept so the
// investigation can be re-run against a save pair with a wider in-game gap.
#![allow(clippy::many_single_char_names, clippy::too_many_lines, clippy::indexing_slicing)]

/// Shortest run worth reporting. FM has roughly 35-50 attributes per player
/// depending on how the hidden ones are counted.
const MIN_RUN: usize = 24;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: findattrs <save.fm>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let main = frames
        .into_iter()
        .max_by_key(|f| f.data.len())
        .ok_or("no frames")?;
    let d = &main.data;
    println!("main frame: {} bytes", d.len());

    // Every maximal run of bytes in 1..=20.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for (i, &b) in d.iter().enumerate() {
        if (1..=20).contains(&b) {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            if i - s >= MIN_RUN {
                runs.push((s, i - s));
            }
        }
    }
    println!("runs of >= {MIN_RUN} bytes within 1..=20: {}", runs.len());

    // Cluster them: a real attribute table is thousands of runs at a regular
    // stride, not a handful scattered about.
    let mut by_len = std::collections::BTreeMap::new();
    for (_, len) in &runs {
        *by_len.entry(*len).or_insert(0usize) += 1;
    }
    println!("\nmost common run lengths:");
    let mut lens: Vec<_> = by_len.iter().collect();
    lens.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (len, count) in lens.iter().take(10) {
        println!("  length {len:>4}  x{count}");
    }

    println!("\nlargest clusters by region (2 MB buckets):");
    let mut buckets = std::collections::BTreeMap::new();
    for (at, _) in &runs {
        *buckets.entry(at / 2_000_000).or_insert(0usize) += 1;
    }
    let mut bs: Vec<_> = buckets.iter().collect();
    bs.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (bucket, count) in bs.iter().take(8) {
        println!("  {:>4} MB  {count} runs", **bucket * 2);
    }

    // Stride between consecutive runs in the densest region tells us the
    // per-player record size.
    if let Some((&dense, _)) = bs.first() {
        let local: Vec<usize> = runs
            .iter()
            .filter(|(at, _)| at / 2_000_000 == dense)
            .map(|(at, _)| *at)
            .collect();
        let mut strides = std::collections::BTreeMap::new();
        for w in local.windows(2) {
            if let ([a, b], ..) = (w, ()) {
                *strides.entry(b - a).or_insert(0usize) += 1;
            }
        }
        let mut ss: Vec<_> = strides.iter().collect();
        ss.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
        println!("\nstrides between runs in the {} MB region:", dense * 2);
        for (stride, count) in ss.iter().take(8) {
            println!("  {stride:>6} bytes  x{count}");
        }

        println!("\nfirst runs in that region:");
        for at in local.iter().take(4) {
            let end = (at + 64).min(d.len());
            let slice = d.get(*at..end).unwrap_or(&[]);
            println!("  @{at}: {:?}", &slice[..slice.len().min(48)]);
        }
    }
    Ok(())
}
