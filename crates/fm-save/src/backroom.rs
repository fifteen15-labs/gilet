//! The club roster table's manager slot — which club a manager works for.
//!
//! Staff never appear in the squad table (§6d), and no employer id sits
//! anywhere near a staff person's record. A dedicated per-club table holds
//! the link, found 4 August 2026 by sweeping a known manager's eid against
//! his club's doubled uid:
//!
//! ```text
//! [eid2 u32] [club uid u32] [club uid u32]  0a 00  [3 bytes] [u32] [u32]
//! [FF FF FF FF] [u32] [manager eid u32 | FF FF FF FF] ...
//! ```
//!
//! One entry per club, in `eid2` order (a separate entity id for the entry
//! itself). The doubled uid is the club's, exactly as squad records carry
//! it. The slot two u32s past the `FF` run held Slot for Liverpool, Arteta
//! for Arsenal and Guardiola for Manchester City on a day-one save, and
//! Iraola for Liverpool in a 2030 career — vacant slots read `FF FF FF FF`
//! (17,207 of 18,857 clubs on day one; 1,646 carry a manager).
//!
//! Everything else in the entry is left alone on purpose: the u32 before
//! the manager resolves to implausible people when read as a person eid,
//! the words after it vary per entry (big clubs carry an extra one), and
//! the count-prefixed list further in is the club's *player* registration
//! list, already covered by the squad table. Only the manager slot is
//! verified, so only the manager slot is read — assistants, physios and
//! the rest of the backroom have no decoded employer yet
//! (`OPEN_PROBLEMS.md` §3c).

/// One club's manager link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClubManager {
    /// Byte offset of the roster entry within its frame.
    pub offset: usize,
    /// The employing club's entity id, resolved through the doubled uid.
    pub club_eid: u32,
    /// The manager's person eid.
    pub manager_eid: u32,
}

/// Person entity ids stay comfortably below this, matching the squad walk.
const MAX_EID: u32 = 3_000_000;

/// Scans a frame for roster entries with a filled manager slot, validated
/// against club `(eid, uid)` pairs the same way squad records are.
#[must_use]
pub fn scan_managers(frame: &[u8], club_ids: &[(u32, u32)]) -> Vec<ClubManager> {
    let by_uid: std::collections::HashMap<u32, u32> =
        club_ids.iter().map(|&(eid, uid)| (uid, eid)).collect();

    let mut out = Vec::new();
    let mut at = 4usize;
    while at + 37 <= frame.len() {
        let Some(entry) = read_entry(frame, at, &by_uid) else {
            at += 1;
            continue;
        };
        at = entry.offset.saturating_add(37);
        if entry.manager_eid != u32::MAX {
            out.push(entry);
        }
    }
    out
}

/// Reads the roster entry whose doubled club uid starts at `at`, if the
/// structure holds. `at` is the first uid; the entry starts four bytes
/// earlier. A vacant manager slot is returned as `u32::MAX` so the scan can
/// still step over the whole entry.
fn read_entry(
    frame: &[u8],
    at: usize,
    by_uid: &std::collections::HashMap<u32, u32>,
) -> Option<ClubManager> {
    let uid = read_u32(frame, at)?;
    if read_u32(frame, at + 4)? != uid || uid == 0 || uid == u32::MAX {
        return None;
    }
    let club_eid = *by_uid.get(&uid)?;

    let entry = at.checked_sub(4)?;
    if frame.get(entry + 12..entry + 14)? != [0x0A, 0x00] {
        return None;
    }
    if frame.get(entry + 25..entry + 29)? != [0xFF; 4] {
        return None;
    }
    let manager_eid = read_u32(frame, entry + 33)?;
    if manager_eid != u32::MAX && (manager_eid == 0 || manager_eid >= MAX_EID) {
        return None;
    }
    Some(ClubManager {
        offset: entry,
        club_eid,
        manager_eid,
    })
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Builds an entry as it sits on disk. Liverpool's real day-one shape:
    /// entity 497, uid 675 doubled, Slot (2058) in the manager slot.
    fn entry(uid: u32, manager: u32) -> Vec<u8> {
        let mut v = 497u32.to_le_bytes().to_vec();
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&[0x0A, 0x00, 0x01, 0xC4, 0x22]);
        v.extend_from_slice(&0x0439u32.to_le_bytes());
        v.extend_from_slice(&0x0439u32.to_le_bytes());
        v.extend_from_slice(&[0xFF; 4]);
        v.extend_from_slice(&67u32.to_le_bytes());
        v.extend_from_slice(&manager.to_le_bytes());
        v.extend_from_slice(&[0x00; 8]);
        v
    }

    const CLUBS: [(u32, u32); 1] = [(366, 675)];

    #[test]
    fn reads_a_filled_manager_slot() {
        let mut buf = vec![0u8; 8];
        buf.extend(entry(675, 2058));
        let found = scan_managers(&buf, &CLUBS);
        assert_eq!(found.len(), 1);
        let m = found.first().unwrap();
        assert_eq!(m.club_eid, 366);
        assert_eq!(m.manager_eid, 2058);
        assert_eq!(m.offset, 8);
    }

    #[test]
    fn a_vacant_slot_yields_nothing() {
        let buf = entry(675, u32::MAX);
        assert!(scan_managers(&buf, &CLUBS).is_empty());
    }

    #[test]
    fn rejects_decoys() {
        let mut buf = Vec::new();
        // A uid no club owns.
        buf.extend(entry(9999, 2058));
        // A manager eid out of range.
        buf.extend(entry(675, MAX_EID));
        // The uid pair broken.
        let mut bad = entry(675, 2058);
        *bad.get_mut(8).unwrap() ^= 0xFF;
        buf.extend(bad);
        assert!(scan_managers(&buf, &CLUBS).is_empty());
    }

    #[test]
    fn survives_truncation_anywhere() {
        let buf = entry(675, 2058);
        for cut in 0..buf.len() {
            let _ = scan_managers(buf.get(..cut).unwrap(), &CLUBS);
        }
    }
}
