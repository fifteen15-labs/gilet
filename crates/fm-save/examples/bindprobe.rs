//! Says why a given `[eid][uid][uid]` triple did or did not name the person
//! whose record contains it — the diagnostic for `bind_identities`' second
//! pass (`OPEN_PROBLEMS.md` §3b).
//!
//! ```text
//! cargo run --release --example bindprobe -- <save.fm> <eid> <uid>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(path), Some(eid), Some(uid)) = (args.get(1), args.get(2), args.get(3)) else {
        eprintln!("usage: bindprobe <save.fm> <eid> <uid>");
        std::process::exit(2);
    };
    let eid: u32 = eid.parse()?;
    let uid: u32 = uid.parse()?;

    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    let mut pattern = Vec::with_capacity(12);
    pattern.extend_from_slice(&eid.to_le_bytes());
    pattern.extend_from_slice(&uid.to_le_bytes());
    pattern.extend_from_slice(&uid.to_le_bytes());
    let hits: Vec<usize> = data
        .windows(12)
        .enumerate()
        .filter(|(_, w)| *w == pattern.as_slice())
        .map(|(i, _)| i)
        .collect();
    println!("{} occurrence(s) of the triple", hits.len());

    let mut offsets: Vec<(usize, &str)> =
        save.people.iter().map(|p| (p.offset, p.full_name.as_str())).collect();
    offsets.sort_unstable_by_key(|(o, _)| *o);

    for at in hits {
        let zeros = at
            .checked_sub(3)
            .and_then(|s| data.get(s..at))
            .is_some_and(|b| b == [0, 0, 0]);
        let header = at.checked_sub(7).is_some_and(|h| {
            data.get(h).is_some_and(|&b| b <= 0x02) && data.get(h + 1) == Some(&0x40)
        });
        let idx = offsets.partition_point(|(o, _)| *o <= at);
        let owner = idx.checked_sub(1).and_then(|i| offsets.get(i));
        println!("\n  at 0x{at:x}");
        println!("    three zero bytes before: {zeros}");
        println!("    object header at -7:     {header}");
        match owner {
            Some((o, name)) => println!("    record owner: {name} at 0x{o:x}, gap {}", at - o),
            None => println!("    record owner: none"),
        }
    }

    let eid_owner = save.people.iter().find(|p| p.eid == Some(eid));
    let uid_owner = save.people.iter().find(|p| p.uid == Some(uid));
    println!(
        "\n  eid {eid} bound to: {}",
        eid_owner.map_or("nobody", |p| p.full_name.as_str())
    );
    println!(
        "  uid {uid} bound to: {}",
        uid_owner.map_or("nobody", |p| p.full_name.as_str())
    );
    Ok(())
}
