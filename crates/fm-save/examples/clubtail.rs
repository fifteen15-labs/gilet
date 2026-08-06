//! What the six bytes between the club id and the name length actually hold.
//!
//! `club::scan_clubs` anchors on `FF FF` immediately before the name length,
//! with the byte before that read as per-club flags. This probe drops that
//! assumption entirely: it finds every position whose *entity head* validates
//! (uid repeated, zero byte, nation twice with `FFFFFFFF` between, a bounded
//! location) and whose tail is a plausible name/short-name pair, then
//! tabulates the six bytes the anchor covers.
//!
//! ```text
//! cargo run --release --example clubtail -- <save.fm> [name-fragment]
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::BTreeMap;

const EID_BACK: usize = 39;
const UID_BACK: usize = 35;
const UID2_BACK: usize = 31;
const HEAD_ZERO_BACK: usize = 27;
const NATION3_BACK: usize = 26;
const HEAD_FF_BACK: usize = 22;
const NATION_LOCATION_BACK: usize = 18;
const NATION_ID_BACK: usize = 14;
const CLUB_ID_BACK: usize = 10;

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_text(frame: &[u8], at: usize, len: usize) -> Option<String> {
    let raw = frame.get(at..at.checked_add(len)?)?;
    let text = std::str::from_utf8(raw).ok()?;
    if text.chars().any(char::is_control) || !text.chars().any(char::is_alphabetic) {
        return None;
    }
    Some(text.to_owned())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: clubtail <save.fm> [fragment]");
    let fragment = args.next().map(|f| f.to_lowercase());

    let bytes = std::fs::read(&path).expect("read save");
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let main = frames.iter().max_by_key(|f| f.data.len()).expect("main frame");
    let frame = &main.data;

    let mut tail: BTreeMap<(u8, u8, u8), usize> = BTreeMap::new();
    let mut mid: BTreeMap<(u8, u8, u8), usize> = BTreeMap::new();
    let mut found = 0usize;
    let mut odd = 0usize;

    for len_at in EID_BACK..frame.len().saturating_sub(80) {
        // Entity head, exactly as club::parse_head validates it.
        let Some(nation1) = read_u32(frame, len_at - NATION_ID_BACK) else { continue };
        let Some(nation3) = read_u32(frame, len_at - NATION3_BACK) else { continue };
        let Some(location) = read_u32(frame, len_at - NATION_LOCATION_BACK) else { continue };
        let Some(ff) = read_u32(frame, len_at - HEAD_FF_BACK) else { continue };
        if nation1 != nation3 || ff != 0xFFFF_FFFF || frame[len_at - HEAD_ZERO_BACK] != 0 {
            continue;
        }
        if location == 0 || location > 10_000 {
            continue;
        }
        let (Some(uid), Some(uid2), Some(eid)) = (
            read_u32(frame, len_at - UID_BACK),
            read_u32(frame, len_at - UID2_BACK),
            read_u32(frame, len_at - EID_BACK),
        ) else {
            continue;
        };
        if uid != uid2 || uid == 0 || uid == u32::MAX {
            continue;
        }

        // Tail: name and short name.
        let Some(name_len) = read_u32(frame, len_at).map(|n| n as usize) else { continue };
        if !(3..=64).contains(&name_len) {
            continue;
        }
        let Some(name) = read_text(frame, len_at + 4, name_len) else { continue };
        let short_at = len_at + 4 + name_len;
        let Some(short_len) = read_u32(frame, short_at).map(|n| n as usize) else { continue };
        if !(2..=32).contains(&short_len) {
            continue;
        }
        let Some(short) = read_text(frame, short_at + 4, short_len) else { continue };
        if !name.starts_with(char::is_uppercase) || !short.starts_with(char::is_uppercase) {
            continue;
        }

        found += 1;
        let t = (frame[len_at - 3], frame[len_at - 2], frame[len_at - 1]);
        let m = (frame[len_at - 6], frame[len_at - 5], frame[len_at - 4]);
        *tail.entry(t).or_default() += 1;
        *mid.entry(m).or_default() += 1;

        // The records the `FF FF` anchor used to drop, by name: junk here
        // would mean the relaxed anchor bought clubs that are not clubs.
        if (t.1, t.2) != (0xFF, 0xFF) && odd < 30 {
            odd += 1;
            println!("dropped-by-FFFF  tail {:02x} {:02x} {:02x}  {name}  /  {short}", t.0, t.1, t.2);
        }

        if fragment.as_ref().is_some_and(|f| name.to_lowercase().contains(f.as_str())) {
            let club_id = read_u32(frame, len_at - CLUB_ID_BACK).unwrap_or(0);
            println!(
                "0x{len_at:x}  eid {eid:<7} uid {uid:<12} nation {nation1:<5} loc {location:<5} club_id {club_id:<7} \
                 mid {:02x} {:02x} {:02x}  tail {:02x} {:02x} {:02x}   {name}  /  {short}",
                m.0, m.1, m.2, t.0, t.1, t.2
            );
        }
    }

    println!("\n{found} head-validated club-shaped records");
    println!("\ntail bytes at [-3 -2 -1] (flags, then the assumed FF FF):");
    let mut rows: Vec<_> = tail.into_iter().collect();
    rows.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for ((a, b, c), n) in rows.iter().take(25) {
        println!("  {a:02x} {b:02x} {c:02x}   {n}");
    }
    println!("\nbytes at [-6 -5 -4]:");
    let mut rows: Vec<_> = mid.into_iter().collect();
    rows.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
    for ((a, b, c), n) in rows.iter().take(15) {
        println!("  {a:02x} {b:02x} {c:02x}   {n}");
    }
}
