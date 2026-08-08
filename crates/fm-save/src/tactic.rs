//! The human manager's active tactic, from the `tactics_man.dat` member.
//!
//! The member opens with the human's tactics-manager object, then the AI
//! teams' records and a set of preset formation templates. Three things are
//! decoded, each verified against a running Port Talbot career and the 19
//! named presets in three saves (21 records reproduce their formation names
//! from the masks exactly):
//!
//! * **The matchday selection** — 26 u32 person eids at a fixed offset,
//!   `FFFFFFFF` for an empty slot. The first eleven are the starting XI *in
//!   tactic-slot order* (proven 11-for-11 by position ratings, with the
//!   tactic's unusual GK DR DC DC DL AMC DM DM AMR AML ST ordering followed
//!   exactly); the rest are the bench, substitute keeper first. Fresh
//!   careers read all-`FF` — no selection yet is the save's truth.
//! * **The user tactic record** — anchored `22 42 00 1a 03 00 01 02`
//!   (`01` in the kind byte is a preset template, skipped), carrying a
//!   length-prefixed name in the clear, a style name behind an `ff` tag,
//!   and eleven slot pairs.
//! * **The slot masks** — each slot is a pair of `42 00 02 [mask u32] ff`
//!   blocks (in-possession and out-of-possession phase); the even-indexed
//!   block of each pair is the formation the tactic names. The mask is one
//!   bit per position, doubled central positions disambiguated by
//!   `+0x200000` (right of the pair) / `+0x100000` (left).
//!
//! Role and duty per slot sit in the block tails and are not yet fitted;
//! a tactic-fit search therefore keys on positions, which are decoded,
//! and says so.

/// One bit per position in a slot mask, in bit order.
const POSITION_BITS: [(u32, &str); 14] = [
    (1, "GK"),
    (1 << 2, "DR"),
    (1 << 3, "DL"),
    (1 << 4, "DC"),
    (1 << 5, "WBR"),
    (1 << 6, "WBL"),
    (1 << 7, "DM"),
    (1 << 8, "MR"),
    (1 << 9, "ML"),
    (1 << 10, "MC"),
    (1 << 11, "AMR"),
    (1 << 12, "AML"),
    (1 << 13, "AMC"),
    (1 << 14, "ST"),
];

/// The matchday selection array: capacity, and where it sits.
const SELECTION_AT: usize = 0x1d;
const SELECTION_CAPACITY: usize = 26;

/// A user tactic record's anchor: `22 42 00 1a 03 00 01` then kind `02`.
const TACTIC_ANCHOR: [u8; 8] = [0x22, 0x42, 0x00, 0x1a, 0x03, 0x00, 0x01, 0x02];

/// The first AI team record; user tactics only sit before it.
const AI_TEAM_TAG: [u8; 4] = [0x49, 0x20, 0x52, 0x42];

/// A slot block's head: `42 00 02 [mask u32] ff`.
const SLOT_HEAD: [u8; 3] = [0x42, 0x00, 0x02];

/// How many slot blocks a tactic carries: eleven slots, two phases each.
const SLOT_BLOCKS: usize = 22;

/// Longest plausible tactic or style name; anything longer is a misread.
const MAX_NAME: usize = 128;

/// The human's active tactic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tactic {
    /// The tactic's name, as the user typed it in FM.
    pub name: String,
    /// The style name behind the record's `ff` tag, when one is set —
    /// "Custom Gegenpress" and friends. Empty templates carry none.
    pub style: Option<String>,
    /// Eleven position names in slot order, from the in-possession masks,
    /// in the same vocabulary as `ability::POSITION_NAMES`.
    pub positions: Vec<String>,
    /// The starting XI's person entity ids, index-aligned with `positions`.
    /// Empty when the save has no selection stored yet.
    pub starters: Vec<u32>,
    /// The bench, substitute keeper first.
    pub bench: Vec<u32>,
}

/// Reads the human's tactic from a decompressed `tactics_man.dat` frame.
///
/// `None` when the member has no user tactic — a fresh career genuinely has
/// none, and an unrecognised layout must read the same way rather than
/// inventing a formation.
#[must_use]
pub fn scan_tactic(frame: &[u8]) -> Option<Tactic> {
    if frame.get(2..6) != Some(b"tad.") {
        return None;
    }

    // User tactics live in the human's object at the top of the frame; the
    // same anchor recurs in AI team records past the first team tag, so the
    // search stops there.
    let end = find(frame, &AI_TEAM_TAG, 0).unwrap_or(frame.len());
    let anchor = find(frame, &TACTIC_ANCHOR, 0).filter(|&at| at < end)?;

    let name_at = anchor + TACTIC_ANCHOR.len();
    let name = read_string(frame, name_at)?;
    let after_name = name_at + 4 + name.len();

    // The style sits behind the next `ff` length tag; `LLUN` (reversed NULL)
    // or an empty read means no style named.
    let style = find(frame, &[0xff], after_name)
        .and_then(|at| read_string(frame, at + 1))
        .filter(|s| !s.is_empty() && s != "LLUN");

    let mut positions = Vec::new();
    let mut at = after_name;
    let mut block = 0usize;
    while block < SLOT_BLOCKS {
        let head = find(frame, &SLOT_HEAD, at)?;
        let mask_at = head + SLOT_HEAD.len();
        let mask = read_u32(frame, mask_at)?;
        if frame.get(mask_at + 4) != Some(&0xff) {
            at = head + 1;
            continue;
        }
        // The even-indexed block of each pair is the formation as named;
        // the odd one is the other phase of play.
        if block.is_multiple_of(2) {
            positions.push(position_name(mask)?.to_owned());
        }
        block += 1;
        at = mask_at + 5;
    }
    if positions.len() != 11 {
        return None;
    }

    let mut selection = Vec::new();
    for i in 0..SELECTION_CAPACITY {
        let Some(eid) = read_u32(frame, SELECTION_AT + i * 4) else {
            break;
        };
        if eid == u32::MAX {
            break;
        }
        selection.push(eid);
    }
    let (starters, bench) = if selection.len() >= 11 {
        let bench = selection.split_off(11);
        (selection, bench)
    } else {
        // A partial selection answers no scouting question; keep the tactic
        // and report no XI rather than a seven-man one.
        (Vec::new(), Vec::new())
    };

    Some(Tactic { name, style, positions, starters, bench })
}

/// The position a slot mask names, sides of doubled pairs stripped.
fn position_name(mask: u32) -> Option<&'static str> {
    let base = mask & 0x000f_ffff;
    let hits: Vec<&str> = POSITION_BITS
        .iter()
        .filter(|(bit, _)| base & bit != 0)
        .map(|(_, name)| *name)
        .collect();
    // A slot is one position; a mask naming none or several is a misread.
    match hits.as_slice() {
        [one] => Some(one),
        _ => None,
    }
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    let tail = haystack.get(from..)?;
    tail.windows(needle.len()).position(|w| w == needle).map(|i| from + i)
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

/// A u32-length-prefixed string, refused when implausibly long or not UTF-8.
fn read_string(b: &[u8], at: usize) -> Option<String> {
    let len = read_u32(b, at)? as usize;
    if len > MAX_NAME {
        return None;
    }
    let bytes = b.get(at + 4..at.checked_add(4)?.checked_add(len)?)?;
    String::from_utf8(bytes.to_vec()).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn mask_names_each_position() {
        assert_eq!(position_name(1), Some("GK"));
        assert_eq!(position_name(1 << 4), Some("DC"));
        // Sides of a doubled pair are data about the pair, not the position.
        assert_eq!(position_name((1 << 4) | 0x0020_0000), Some("DC"));
        assert_eq!(position_name((1 << 4) | 0x0010_0000), Some("DC"));
        assert_eq!(position_name(1 << 14), Some("ST"));
        // Two bits set is not a position.
        assert_eq!(position_name((1 << 4) | (1 << 10)), None);
        assert_eq!(position_name(0), None);
    }
}
