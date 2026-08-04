//! Differential analysis across two saves of the same career.
//!
//! Current Ability moves as a career runs: young players gain, old players
//! decline. Potential Ability barely moves at all. Matching people by name
//! across two saves and sweeping every byte offset after the name turns that
//! into a search — no ground truth needed.
//!
//! ```text
//! cargo run --release --example diff -- earlier.fm later.fm
//! ```

// Research spike, not shipped code: a one-off analysis tool kept so the
// investigation can be re-run against a save pair with a wider in-game gap.
#![allow(clippy::many_single_char_names, clippy::too_many_lines, clippy::indexing_slicing)]

use std::collections::HashMap;

/// `surname_id` + middle + `full_name_length`, matching `person::PREFIX_LEN`.
const PREFIX: usize = 14;
/// How far past the name to sweep.
const WINDOW: usize = 3000;

/// Born this year or later: still developing, so ability should rise.
const YOUNG_FROM: u16 = 2004;
/// Born this year or earlier: past peak, so ability should fall.
const OLD_UNTIL: u16 = 1992;

struct Loaded {
    frame: Vec<u8>,
    people: Vec<fm_save::Person>,
}

fn load(path: &str) -> Result<Loaded, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let main = frames
        .into_iter()
        .max_by_key(|f| f.data.len())
        .ok_or("no frames in save")?;
    let strings = fm_save::strings::scan_strings(&main.data).ok_or("no string table in save")?;
    let people = fm_save::person::scan_people(&main.data, &strings);
    Ok(Loaded { frame: main.data, people })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(a_path), Some(b_path)) = (args.next(), args.next()) else {
        eprintln!("usage: diff <earlier.fm> <later.fm>");
        std::process::exit(2);
    };

    let a = load(&a_path)?;
    let b = load(&b_path)?;
    println!("earlier {} people, later {} people", a.people.len(), b.people.len());

    // Names are not unique in a big database; keep only unambiguous ones so a
    // mismatch cannot pollute the deltas.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for p in &a.people {
        *counts.entry(p.full_name.as_str()).or_default() += 1;
    }
    let mut b_index: HashMap<&str, &fm_save::Person> = HashMap::new();
    let mut b_counts: HashMap<&str, usize> = HashMap::new();
    for p in &b.people {
        *b_counts.entry(p.full_name.as_str()).or_default() += 1;
        b_index.insert(p.full_name.as_str(), p);
    }

    let mut pairs = Vec::new();
    for p in &a.people {
        let name = p.full_name.as_str();
        if counts.get(name) != Some(&1) || b_counts.get(name) != Some(&1) {
            continue;
        }
        if let Some(q) = b_index.get(name) {
            pairs.push((p, *q));
        }
    }
    println!("matched uniquely by name: {}", pairs.len());

    // Per offset: how often it changes, and which way for young vs old.
    let mut stats: Vec<Stat> = (0..WINDOW)
        .map(|_| Stat { min: u8::MAX, ..Stat::default() })
        .collect();

    for (p, q) in &pairs {
        let pa = p.offset + PREFIX + p.full_name.len();
        let qa = q.offset + PREFIX + q.full_name.len();
        let Some(year) = p.date_of_birth.map(|d| d.year) else {
            continue;
        };
        for k in 0..WINDOW {
            let (Some(&x), Some(&y)) = (a.frame.get(pa + k), b.frame.get(qa + k)) else {
                continue;
            };
            let Some(s) = stats.get_mut(k) else { continue };
            s.n += 1;
            s.min = s.min.min(x);
            s.max = s.max.max(x);
            if x != y {
                s.changed += 1;
            }
            let delta = i32::from(y) - i32::from(x);
            if year >= YOUNG_FROM {
                s.young_n += 1;
                s.young_delta += delta;
            } else if year <= OLD_UNTIL {
                s.old_n += 1;
                s.old_delta += delta;
            }
        }
    }

    {
        let mut top: Vec<(usize, f64, &Stat)> = stats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.n > 0)
            .map(|(k, s)| (k, 100.0 * f64::from(s.changed) / f64::from(s.n), s))
            .filter(|(_, pct, _)| *pct > 1.0)
            .collect();
        top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        println!("\nmost-changed offsets across the whole window:");
        println!("  off   changed%   youngD    oldD   min  max");
        for (k, pct, s) in top.iter().take(25) {
            let y = if s.young_n > 0 { f64::from(s.young_delta) / f64::from(s.young_n) } else { 0.0 };
            let o = if s.old_n > 0 { f64::from(s.old_delta) / f64::from(s.old_n) } else { 0.0 };
            println!("  +{k:<4} {pct:7.1}%  {y:+8.2} {o:+8.2}  {:>4} {:>4}", s.min, s.max);
        }
    }

    println!("\nraw sweep, first 64 offsets:");
    println!("  off  n      changed%  youngD    oldD    min  max");
    for (k, s) in stats.iter().enumerate().take(64) {
        if s.n == 0 { continue }
        let pct = 100.0 * f64::from(s.changed) / f64::from(s.n);
        let y = if s.young_n > 0 { f64::from(s.young_delta) / f64::from(s.young_n) } else { 0.0 };
        let o = if s.old_n > 0 { f64::from(s.old_delta) / f64::from(s.old_n) } else { 0.0 };
        println!("  +{k:<3} {:<6} {pct:7.1}%  {y:+8.2} {o:+8.2}   {:>3}  {:>3}", s.n, s.min, s.max);
    }

    println!("\noffsets where the young gain and the old decline (the CA signature):");
    println!("  off  changed%   young Δ    old Δ   range");
    let mut ranked: Vec<(usize, &Stat, f64, f64)> = stats
        .iter()
        .enumerate()
        .filter_map(|(k, s)| {
            if s.n == 0 || s.young_n == 0 || s.old_n == 0 {
                return None;
            }
            let y = f64::from(s.young_delta) / f64::from(s.young_n);
            let o = f64::from(s.old_delta) / f64::from(s.old_n);
            // Ability is 1-200 and must actually move.
            if s.max > 200 || s.changed * 5 < s.n {
                return None;
            }
            (y > 0.0 && o < 0.0).then_some((k, s, y, o))
        })
        .collect();
    ranked.sort_by(|l, r| (r.2 - r.3).partial_cmp(&(l.2 - l.3)).unwrap_or(std::cmp::Ordering::Equal));

    for (k, s, y, o) in ranked.iter().take(12) {
        let pct = 100.0 * f64::from(s.changed) / f64::from(s.n);
        println!("  +{k:<3} {pct:7.1}%  {y:+8.2} {o:+8.2}   {}..{}", s.min, s.max);
    }
    if ranked.is_empty() {
        println!("  (none)");
    }

    // Fields that barely move but hold a plausible ability value are the PA
    // candidates; PA is fixed for most players once a career is running.
    println!("\nstable 1-200 fields (Potential Ability candidates):");
    println!("  off  changed%   mean   range");
    let mut stable: Vec<(usize, &Stat)> = stats
        .iter()
        .enumerate()
        .filter(|(_, s)| s.n > 0 && s.max <= 200 && s.min >= 1 && s.changed * 100 < s.n * 3 && s.max - s.min > 60)
        .collect();
    stable.sort_by_key(|(_, s)| s.changed);
    for (k, s) in stable.iter().take(12) {
        let pct = 100.0 * f64::from(s.changed) / f64::from(s.n);
        println!("  +{k:<3} {pct:7.2}%  {:6.1}   {}..{}", f64::from(s.max + s.min) / 2.0, s.min, s.max);
    }
    Ok(())
}

#[derive(Default)]
struct Stat {
    n: u32,
    changed: u32,
    min: u8,
    max: u8,
    young_n: u32,
    young_delta: i32,
    old_n: u32,
    old_delta: i32,
}
