//! The human manager's identity, from the `humans.dat` member.
//!
//! The member names no club. It carries the human's *person* entity id at a
//! fixed header slot, and the club is then whatever club that person manages
//! — `Person::club_eid` from the roster-seat binding. Verified on four
//! careers: Heybridge Swifts, Afan Lido and Port Talbot all resolve to their
//! own club, and an unemployed career resolves to no club, which is the
//! truth, not a gap.
//!
//! Layout, byte-identical across every save inspected:
//!
//! ```text
//! 00  03 01 74 61 64 2e   frame prefix, then "tad."
//! 06  15 00               member body tag
//! 08  [count u16]         how many humans — 1 in every available save
//! 0a  [eid u32]           the human manager's person entity id
//! 0e  0c 00 03 00 ...     avatar profile blob, constant across saves
//! ```
//!
//! The member also ends with a pair of 27-byte records holding the same eid
//! and its one-below object id, which corroborates the header but is not
//! required — a second human's layout is unverified, so only the first eid
//! is read.

/// Entity ids stay comfortably below this, matching the person scan.
const MAX_EID: u32 = 3_000_000;

/// The member body tag that opens every observed `humans.dat`.
const BODY_TAG: u16 = 0x0015;

/// Reads the first human's person entity id from a decompressed
/// `humans.dat` frame. `None` when the member does not carry the known
/// shape — an unrecognised layout must read as "no reading", not as a
/// guessed manager.
#[must_use]
pub fn scan_human(frame: &[u8]) -> Option<u32> {
    if frame.get(2..6) != Some(b"tad.") {
        return None;
    }
    let tag = u16::from_le_bytes([*frame.get(6)?, *frame.get(7)?]);
    if tag != BODY_TAG {
        return None;
    }
    let count = u16::from_le_bytes([*frame.get(8)?, *frame.get(9)?]);
    if count == 0 {
        return None;
    }
    let eid = u32::from_le_bytes([
        *frame.get(0x0a)?,
        *frame.get(0x0b)?,
        *frame.get(0x0c)?,
        *frame.get(0x0d)?,
    ]);
    (eid != 0 && eid < MAX_EID).then_some(eid)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn member(count: u16, eid: u32) -> Vec<u8> {
        let mut v = vec![0x03, 0x01];
        v.extend_from_slice(b"tad.");
        v.extend_from_slice(&BODY_TAG.to_le_bytes());
        v.extend_from_slice(&count.to_le_bytes());
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&[0x0c, 0x00, 0x03, 0x00]);
        v
    }

    #[test]
    fn reads_the_human_eid() {
        // The Heybridge Swifts career's real header values.
        assert_eq!(scan_human(&member(1, 109_683)), Some(109_683));
    }

    #[test]
    fn refuses_the_unknown() {
        assert_eq!(scan_human(&member(0, 109_683)), None, "no humans");
        assert_eq!(scan_human(&member(1, 0)), None, "null eid");
        assert_eq!(scan_human(b"xx"), None, "truncated");
        let mut wrong_tag = member(1, 109_683);
        wrong_tag[6] = 0x16;
        assert_eq!(scan_human(&wrong_tag), None, "unknown body tag");
    }
}
