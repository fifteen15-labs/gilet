//! Non-player attributes: the sheet the pre-game editor calls "All
//! Attributes", as the save stores it.
//!
//! A person carries two entity objects. The one inside their name record holds
//! their identity; a *second* object, one entity id lower, holds their
//! non-player data — five u16s and then a 54-value block. Sterling's pair is
//! eid 8401 then 8402, and Fradley's sheet sits under 20129 while he is 20130.
//! Reading the block off the object that shares a person's own eid therefore
//! reads the *next* person's sheet, which is what defeated every earlier
//! attempt at this (`docs/OPEN_PROBLEMS.md` §3b).
//!
//! The block is the editor's flat 52-item list at slots 2-53, behind a
//! two-byte header. The first twenty-six items — the tendency half, Attacking
//! through Width — are stored on the raw 1-20 scale. The rest, the coaching
//! and knowledge half, are stored times five, with a per-person drift of a
//! point or two. Verified against editor sheets for Marko Nikolić (thirteen
//! raw-exact, eleven more within two of raw times five) and Daniel Fradley
//! (all fifteen of his non-blank items within two).

/// Values in the block, including the two header bytes.
pub const BLOCK_LEN: usize = 54;

/// Attributes in the editor's list, which fill the block after the header.
pub const ATTRIBUTE_COUNT: usize = 52;

/// Slot the first attribute occupies: `s0` is a small enum and `s1` reads 12
/// in 97.7% of blocks, neither of them data.
const FIRST_SLOT: usize = 2;

/// Attributes up to this index are stored raw; the rest are stored times five.
/// The split falls exactly where the editor's list turns from tendencies to
/// coaching, between Width and Coaching.
const RAW_ITEMS: usize = 26;

const SCALE: u8 = 5;

/// Reputation is scaled to 10000 in the editor, from a 0-200 value.
const REPUTATION_SCALE: u16 = 50;

/// Highest plausible reputation, and the ability ceiling.
const MAX_REPUTATION: u16 = 10_000;
const MAX_ABILITY: u16 = 200;

/// How far past the tag byte the five u16s can start. Most objects carry them
/// immediately; some hold a preamble of 8-byte rows first, which is why a
/// fixed stride cannot be used.
const FIELD_SEARCH: usize = 64;

/// Filler between the five u16s and the block.
const FILLER: usize = 8;

/// The editor's list, in storage order. Two entries the editor itself leaves
/// blank are `None` here rather than guessed at.
const ATTRIBUTE_NAMES: [Option<&str>; ATTRIBUTE_COUNT] = [
    Some("Attacking"),
    Some("Business"),
    Some("Coaching Technique"),
    Some("Directness"),
    Some("Authority"),
    Some("Free Roles"),
    Some("Interference"),
    Some("Marking"),
    Some("Offside"),
    Some("Patience"),
    Some("Trigger Press"),
    Some("Resources"),
    Some("Working With Youngsters"),
    Some("Determination"),
    Some("Buying Players"),
    Some("Mind Games"),
    Some("Sitting Back"),
    Some("Use Of Play-Maker"),
    Some("Use Of Subs"),
    Some("Depth"),
    Some("Fluidity"),
    Some("Flexibility"),
    Some("Hardness Of Training"),
    Some("Squad Rotation"),
    Some("Tempo"),
    Some("Width"),
    Some("Coaching"),
    Some("Coaching Goalkeeping"),
    Some("Judging Player Ability"),
    Some("Judging Player Potential"),
    Some("People Management"),
    Some("Motivating"),
    Some("Physiotherapy"),
    Some("Tactical Knowledge"),
    Some("Coaching Attacking"),
    Some("Coaching Defending"),
    Some("Coaching Fitness"),
    Some("Coaching Possession"),
    Some("Coaching Technical"),
    Some("Coaching Tactical"),
    Some("Dirtiness Allowance"),
    Some("Coaching GK Handling"),
    Some("Coaching GK Distribution"),
    Some("Versatility"),
    Some("Judging Player Data"),
    None,
    None,
    Some("Sports Science"),
    Some("Eccentricity"),
    Some("Negotiating"),
    Some("Judging Staff Ability"),
    Some("Coaching Set Pieces"),
];

/// The editor's name for an attribute index, where it has one.
#[must_use]
pub fn attribute_name(index: usize) -> Option<&'static str> {
    ATTRIBUTE_NAMES.get(index).copied().flatten()
}

/// One person's non-player data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staff {
    /// Offset of the identity triple the block sits behind.
    pub offset: usize,
    /// Entity id of the object, one below the person's own.
    pub eid: u32,
    /// The object's database id.
    pub uid: u32,
    /// Reputation in the person's home nation, 0-200.
    pub home_reputation: u16,
    /// Reputation where they currently work, 0-200.
    pub current_reputation: u16,
    /// Worldwide reputation, 0-200.
    pub world_reputation: u16,
    /// Non-player Current Ability, 0-200.
    pub current_ability: u16,
    /// Non-player Potential Ability, 0-200.
    pub potential_ability: u16,
    /// The 52 attributes, all converted to the 1-20 scale the editor shows.
    pub attributes: [u8; ATTRIBUTE_COUNT],
}

impl Staff {
    /// An attribute by the editor's name, case-sensitive.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<u8> {
        let index = ATTRIBUTE_NAMES.iter().position(|n| *n == Some(name))?;
        self.attributes.get(index).copied()
    }
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

/// Entity ids stay comfortably below this, matching the person scan.
const MAX_EID: u32 = 3_000_000;

/// Finds every attribute block in a frame, each behind an identity triple.
///
/// The anchor is the `[eid][uid][uid]` triple itself, accepted with either
/// an entity object header — a type byte of 0-2, then `0x40`, seven bytes
/// back — or the three zero bytes every identity block carries. The header
/// cannot be required: plenty of records write the identity without one
/// (`person.rs` counts 26,089 on a day-one save), and the sheet behind a
/// headerless identity was invisible to the old header-first scan — Arne
/// Slot's CA-165 sheet among them, behind Verberne's headerless triple.
///
/// The five u16s are then found by searching forward for a reading where
/// both abilities are sane and the 54 bytes eight past them are all 1-100.
/// A bare run of small numbers is no signature, but 54 consecutive non-zero
/// bytes under 101 will not occur by chance.
#[must_use]
pub fn scan_staff(frame: &[u8]) -> Vec<Staff> {
    let mut out = Vec::new();
    let mut at = 3usize;
    while at + 17 <= frame.len() {
        let Some(found) = read_object(frame, at) else {
            at += 1;
            continue;
        };
        at = found.offset.saturating_add(BLOCK_LEN);
        out.push(found);
    }
    out
}

/// Reads the block behind the identity triple at `at`, if there is one.
fn read_object(frame: &[u8], at: usize) -> Option<Staff> {
    let headed = at >= 7
        && frame.get(at - 7).is_some_and(|&b| b <= 0x02)
        && frame.get(at - 6) == Some(&0x40);
    let zeroed = frame.get(at.checked_sub(3)?..at) == Some(&[0u8; 3][..]);
    if !headed && !zeroed {
        return None;
    }
    let (eid, u1, u2) = (
        read_u32(frame, at)?,
        read_u32(frame, at + 4)?,
        read_u32(frame, at + 8)?,
    );
    if u1 != u2 || u1 == 0 || u1 == u32::MAX || eid == 0 || eid >= MAX_EID {
        return None;
    }
    // The sheet-bearing object is tagged `01` after the triple; other tags
    // (a compact entry's `10`, a reference's flags) carry no block.
    if frame.get(at + 12) != Some(&0x01) {
        return None;
    }

    (at + 13..at + 13 + FIELD_SEARCH).find_map(|fields| {
        let vals: Vec<u16> = (0..5).filter_map(|i| read_u16(frame, fields + i * 2)).collect();
        let (&home, &current, &world, &ca, &pa) = (
            vals.first()?,
            vals.get(1)?,
            vals.get(2)?,
            vals.get(3)?,
            vals.get(4)?,
        );
        if home > MAX_REPUTATION || current > MAX_REPUTATION || world > MAX_REPUTATION {
            return None;
        }
        if ca == 0 || ca > MAX_ABILITY || pa < ca || pa > MAX_ABILITY {
            return None;
        }
        let start = fields + 10 + FILLER;
        let block = frame.get(start..start.checked_add(BLOCK_LEN)?)?;
        if block.iter().any(|&b| b == 0 || b > 100) {
            return None;
        }

        let mut attributes = [0u8; ATTRIBUTE_COUNT];
        for (index, slot) in attributes.iter_mut().enumerate() {
            let raw = *block.get(index + FIRST_SLOT)?;
            // The coaching half is stored times five; rounding to nearest
            // recovers the editor's value through the small per-save drift.
            *slot = if index < RAW_ITEMS {
                raw
            } else {
                ((raw + SCALE / 2) / SCALE).max(1)
            };
        }

        Some(Staff {
            offset: at,
            eid,
            uid: u1,
            home_reputation: home / REPUTATION_SCALE,
            current_reputation: current / REPUTATION_SCALE,
            world_reputation: world / REPUTATION_SCALE,
            current_ability: ca,
            potential_ability: pa,
            attributes,
        })
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Builds an object as it sits on disk: header, ids, tag, an optional
    /// preamble, the five u16s, eight bytes of filler, then the block.
    fn object(eid: u32, uid: u32, fields: [u16; 5], block: [u8; BLOCK_LEN], preamble: usize) -> Vec<u8> {
        let mut v = vec![0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00];
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.push(0x01);
        v.extend(std::iter::repeat_n(0xEEu8, preamble));
        for f in fields {
            v.extend_from_slice(&f.to_le_bytes());
        }
        v.extend(std::iter::repeat_n(0x11u8, FILLER));
        v.extend_from_slice(&block);
        v
    }

    fn block_with(pairs: &[(usize, u8)]) -> [u8; BLOCK_LEN] {
        let mut b = [7u8; BLOCK_LEN];
        b[0] = 33;
        b[1] = 12;
        for &(slot, value) in pairs {
            b[slot] = value;
        }
        b
    }

    #[test]
    fn reads_an_object_and_names_its_attributes() {
        // Nikolić's real numbers: reputation 6250/6500/4500 home/current/world,
        // CA 130, PA 145, Authority 17 raw at s6, Coaching Defending 16 stored
        // as 78 at s37.
        let buf = object(
            5155,
            5_789_646,
            [6250, 6500, 4500, 130, 145],
            block_with(&[(6, 17), (37, 78)]),
            0,
        );
        let found = scan_staff(&buf);
        assert_eq!(found.len(), 1);
        let s = found.first().unwrap();
        assert_eq!((s.eid, s.uid), (5155, 5_789_646));
        assert_eq!(
            (s.home_reputation, s.current_reputation, s.world_reputation),
            (125, 130, 90)
        );
        assert_eq!((s.current_ability, s.potential_ability), (130, 145));
        assert_eq!(s.get("Authority"), Some(17), "the raw half is not rescaled");
        assert_eq!(s.get("Coaching Defending"), Some(16), "78 rounds back to 16");
    }

    #[test]
    fn finds_a_block_behind_a_headerless_identity() {
        // Slot's real shape: the identity in front of his sheet (Verberne's,
        // eid 2057) carries no object header, only the zero bytes — the old
        // header-first scan missed it and Slot showed no sheet at all.
        let with_header = object(
            2057,
            601_116,
            [8250, 8250, 6750, 165, 175],
            block_with(&[]),
            0,
        );
        let mut buf = vec![0u8; 3];
        buf.extend_from_slice(&with_header[7..]);
        let found = scan_staff(&buf);
        assert_eq!(found.len(), 1);
        let s = found.first().unwrap();
        assert_eq!((s.eid, s.uid), (2057, 601_116));
        assert_eq!((s.current_ability, s.potential_ability), (165, 175));
    }

    #[test]
    fn a_bare_triple_with_no_anchor_is_refused() {
        // No header and no zero bytes in front: the triple alone is not
        // enough, however plausible the block behind it.
        let with_header = object(
            2057,
            601_116,
            [8250, 8250, 6750, 165, 175],
            block_with(&[]),
            0,
        );
        let mut buf = vec![0xAAu8; 3];
        buf.extend_from_slice(&with_header[7..]);
        assert!(scan_staff(&buf).is_empty());
    }

    #[test]
    fn finds_the_fields_behind_a_preamble() {
        // Nikolić's object carries 8-byte rows between the tag and the fields,
        // so a fixed stride reads garbage.
        let buf = object(
            5155,
            5_789_646,
            [6250, 6500, 4500, 130, 145],
            block_with(&[(6, 17)]),
            24,
        );
        let found = scan_staff(&buf);
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().current_ability, 130);
    }

    #[test]
    fn rejects_an_impossible_ability_pair() {
        let buf = object(
            5155,
            5_789_646,
            [6250, 6500, 4500, 200, 100],
            block_with(&[]),
            0,
        );
        assert!(scan_staff(&buf).is_empty(), "potential below current is a misread");
    }

    #[test]
    fn rejects_a_block_holding_a_zero() {
        let mut block = block_with(&[]);
        block[40] = 0;
        let buf = object(5155, 5_789_646, [6250, 6500, 4500, 130, 145], block, 0);
        assert!(scan_staff(&buf).is_empty());
    }

    #[test]
    fn survives_truncation_anywhere() {
        let buf = object(
            5155,
            5_789_646,
            [6250, 6500, 4500, 130, 145],
            block_with(&[]),
            0,
        );
        for cut in 0..buf.len() {
            let _ = scan_staff(buf.get(..cut).unwrap());
        }
    }

    #[test]
    fn the_two_unnamed_editor_rows_stay_unnamed() {
        assert_eq!(attribute_name(0), Some("Attacking"));
        assert_eq!(attribute_name(25), Some("Width"));
        assert_eq!(attribute_name(45), None);
        assert_eq!(attribute_name(46), None);
        assert_eq!(attribute_name(51), Some("Coaching Set Pieces"));
        assert_eq!(attribute_name(ATTRIBUTE_COUNT), None);
    }
}
