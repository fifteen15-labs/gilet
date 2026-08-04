//! Measures how far into a person record their own identity block sits, over
//! the people the ascending chain already binds. The answer sets the window a
//! second binding pass can trust: a block further in than any real one is more
//! likely the *next* person's second object, which lives deep inside the
//! preceding record (Sterling's is 631 bytes before his own name).
//!
//! ```text
//! cargo run --release --example idgap -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: idgap <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let data = &main.data;
    let save = fm_save::Save::parse(&bytes)?;

    // For each bound person, find their own `[eid][uid][uid]` triple by
    // searching forward from their record prefix, and record the distance.
    let mut gaps: Vec<usize> = Vec::new();
    let mut unbound = 0usize;
    for person in &save.people {
        let (Some(eid), Some(uid)) = (person.eid, person.uid) else {
            unbound += 1;
            continue;
        };
        let mut pattern = Vec::with_capacity(12);
        pattern.extend_from_slice(&eid.to_le_bytes());
        pattern.extend_from_slice(&uid.to_le_bytes());
        pattern.extend_from_slice(&uid.to_le_bytes());
        let window = data
            .get(person.offset..person.offset.saturating_add(4096))
            .unwrap_or_default();
        if let Some(at) = window.windows(12).position(|w| w == pattern.as_slice()) {
            gaps.push(at);
        }
    }
    gaps.sort_unstable();

    let n = gaps.len();
    println!("{n} located identities, {unbound} people unbound");
    for pct in [50usize, 90, 95, 99, 100] {
        let i = (n.saturating_mul(pct) / 100).min(n.saturating_sub(1));
        println!("  p{pct:<4} {}", gaps.get(i).copied().unwrap_or(0));
    }
    for bound in [128usize, 192, 256, 384, 512, 1024] {
        let under = gaps.iter().filter(|&&g| g <= bound).count();
        let pct = 100.0 * f64::from(u32::try_from(under).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(n).unwrap_or(1).max(1));
        println!("  <= {bound:<5} {under} ({pct:.2}%)");
    }
    Ok(())
}
