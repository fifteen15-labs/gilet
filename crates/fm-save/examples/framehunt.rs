//! Scans the whole main frame for u32 values inside a narrow range and
//! prints each hit with the nearest person record behind it — for finding
//! where a known money figure (a release clause converted at the game's
//! internal rate) actually lives.
//!
//! ```text
//! cargo run --release --example framehunt -- <save.fm> 51852000 51852700
//! ```

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(min), Some(max)) = (
        args.next(),
        args.next().and_then(|s| s.parse::<u32>().ok()),
        args.next().and_then(|s| s.parse::<u32>().ok()),
    ) else {
        eprintln!("usage: framehunt <save.fm> <min> <max>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(frame) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &frame.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut offsets: Vec<(usize, &str)> =
        save.people.iter().map(|p| (p.offset, p.full_name.as_str())).collect();
    offsets.sort_unstable();

    let mut hits = 0usize;
    for at in 0..data.len().saturating_sub(4) {
        let Some(v) = read_u32(data, at) else { continue };
        if v < min || v > max {
            continue;
        }
        hits += 1;
        if hits > 40 {
            continue;
        }
        let idx = offsets.partition_point(|&(o, _)| o <= at);
        let behind = idx
            .checked_sub(1)
            .and_then(|i| offsets.get(i))
            .map_or(("-", 0), |&(o, n)| (n, at - o));
        let ahead = offsets
            .get(idx)
            .map_or(("-", 0), |&(o, n)| (n, o - at));
        println!(
            "{at:#x}: {v}  ({} +{} behind | {} in {} ahead)",
            behind.0, behind.1, ahead.0, ahead.1
        );
    }
    println!("total hits: {hits}");
    Ok(())
}
