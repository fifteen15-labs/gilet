//! Multiset scan for a staff member's published coaching sheet
//! (`OPEN_PROBLEMS` §3b). Storage order is unknown, so instead of matching
//! the screen sequence this sorts each candidate window and compares value
//! multisets — order-blind, drift-tolerant.
//!
//! Emery's published FM 26.0.0 sheet is the target, in three groupings
//! (whole sheet, coaching screen, mental and knowledge groups) and two
//! scales (raw 1-100, display 1-20), across **every frame** in the archive —
//! staff data may live outside the main database member.
//!
//! ```text
//! cargo run --release --example staffscan -- <save.fm>
//! ```

/// Emery, FM 26.0.0, efem.club order: Attacking, Defending, Fitness, Mental,
/// Tactical, Technical, Set Pieces, Working With Youngsters, GK Shot
/// Stopping, GK Handling, GK Distribution, Adaptability, Determination,
/// Level of Discipline, Motivating, People Management, Judging Staff
/// Ability, Negotiating, Tactical Knowledge, Analysing Data, Judging Player
/// Ability, Judging Player Potential, Physiotherapy, Sports Science.
const PUB: [u8; 24] = [
    80, 78, 80, 36, 92, 82, 67, 81, 67, 10, 12, 22, 82, 100, 80, 79, 84, 82, 81, 13, 82, 82, 33,
    27,
];

/// The eleven values of the in-game coaching screen, in PUB order.
const COACHING: [u8; 11] = [80, 78, 80, 36, 92, 82, 67, 81, 67, 10, 12];
/// Adaptability, Determination, Discipline, Motivating, People Management.
const MENTAL: [u8; 5] = [22, 82, 100, 80, 79];
/// Judging Staff Ability, Negotiating, Tactical Knowledge, Analysing Data —
/// plus Judging Player Ability and Potential.
const KNOWLEDGE: [u8; 6] = [84, 82, 81, 13, 82, 82];

fn sorted(values: &[u8]) -> Vec<u8> {
    let mut v = values.to_vec();
    v.sort_unstable();
    v
}

/// L1 distance between two sorted multisets of equal length.
fn distance(a: &[u8], b: &[u8]) -> u32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| u32::from(x.abs_diff(y)))
        .sum()
}

struct Hit {
    offset: usize,
    dist: u32,
    window: Vec<u8>,
}

/// Scan windows of `target.len()` bytes covering each anchor position,
/// keeping any whose sorted multiset sits within `max_dist` of the target.
fn scan(data: &[u8], anchor: u8, target: &[u8], max_dist: u32) -> Vec<Hit> {
    let len = target.len();
    let mut hits: Vec<Hit> = Vec::new();
    let mut last_end = 0usize;
    for (a, _) in data.iter().enumerate().filter(|(_, &b)| b == anchor) {
        let lo = a.saturating_sub(len - 1).max(last_end);
        for start in lo..=a {
            let Some(w) = data.get(start..start + len) else {
                continue;
            };
            let d = distance(&sorted(w), target);
            if d <= max_dist {
                if let Some(last) = hits.last_mut() {
                    if start < last.offset + len {
                        if d < last.dist {
                            *last = Hit { offset: start, dist: d, window: w.to_vec() };
                        }
                        last_end = start;
                        continue;
                    }
                }
                hits.push(Hit { offset: start, dist: d, window: w.to_vec() });
                last_end = start;
            }
        }
    }
    hits
}

fn report(frame: usize, label: &str, hits: &[Hit]) {
    if hits.is_empty() {
        return;
    }
    println!("frame {frame} {label}: {} hit(s)", hits.len());
    for h in hits.iter().take(25) {
        let bytes: Vec<String> = h.window.iter().map(|b| format!("{b}")).collect();
        println!("  0x{:x} d={} [{}]", h.offset, h.dist, bytes.join(" "));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: staffscan <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    println!("{} frames", frames.len());

    let display = |v: &[u8]| -> Vec<u8> { v.iter().map(|&x| x.div_ceil(5)).collect() };

    for (i, frame) in frames.iter().enumerate() {
        let data = &frame.data;
        report(i, "sheet24 1-100 d<=40", &scan(data, 92, &sorted(&PUB), 40));
        report(i, "coaching11 1-100 d<=16", &scan(data, 92, &sorted(&COACHING), 16));
        report(i, "mental5 1-100 d<=6", &scan(data, 100, &sorted(&MENTAL), 6));
        report(i, "knowledge6 1-100 d<=6", &scan(data, 84, &sorted(&KNOWLEDGE), 6));
        report(i, "sheet24 1-20 d<=8", &scan(data, 20, &sorted(&display(&PUB)), 8));
        report(
            i,
            "coaching11 1-20 d<=4",
            &scan(data, 19, &sorted(&display(&COACHING)), 4),
        );
    }
    println!("done");
    Ok(())
}
