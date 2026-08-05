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

/// Smallest list worth trusting: short ascending runs occur by chance,
/// five strictly-ascending in-range u32s behind an exact count byte do not.
const MIN_LIST: usize = 5;

/// No single staff list is longer than this.
const MAX_LIST: usize = 120;

/// How far past a club record's head its body can run. Spans are also cut
/// at the next club's head, so the cap mostly matters for the last club —
/// but it must clear a real body: an aged club's staff lists sit 18KB past
/// the head (Port Talbot in a 2031 career), where a day-one club's sit
/// within 6KB.
const MAX_SPAN: usize = 64 * 1024;

/// One staff list found inside a club record's body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffList {
    /// Byte offset of the count byte within the frame.
    pub offset: usize,
    /// The employing club's entity id.
    pub club_eid: u32,
    /// The members' person eids, ascending as stored.
    pub eids: Vec<u32>,
}

/// Finds the backroom staff lists inside each club record's body.
///
/// The club record carries several count-prefixed person eid lists in its
/// body — FM's staff categories (Liverpool's day-one record holds runs of
/// 20, 35 and 39, Hulshoff in the 35; the manager is in none of them,
/// living in the roster table instead). Day-one lists are ascending and sit
/// a few KB past the head; an aged career shuffles them the way transfers
/// shuffle squad lists and pushes them deeper as the record grows (Port
/// Talbot's sit 18KB in by 2031, still inside the club's own span —
/// verified against the running career's staff screen). Order is therefore
/// not part of the shape: only a count byte and that many in-range entity
/// ids. The real acceptance lives with the caller, which must gate each
/// list on its members resolving to non-player people — four in five,
/// which random bytes or a run of some other entity kind cannot pass.
///
/// `clubs` is `(record offset, club eid)`, unsorted.
#[must_use]
pub fn scan_staff_lists(frame: &[u8], clubs: &[(usize, u32)]) -> Vec<StaffList> {
    let mut sorted: Vec<(usize, u32)> = clubs.to_vec();
    sorted.sort_unstable();

    let mut out = Vec::new();
    for (i, &(head, club_eid)) in sorted.iter().enumerate() {
        let end = sorted
            .get(i + 1)
            .map_or(head.saturating_add(MAX_SPAN), |&(next, _)| next)
            .min(head.saturating_add(MAX_SPAN))
            .min(frame.len());
        let mut at = head;
        while at < end.saturating_sub(4) {
            if let Some(eids) = read_list(frame, at, end) {
                let len = eids.len();
                out.push(StaffList {
                    offset: at,
                    club_eid,
                    eids,
                });
                at += 1 + len * 4;
            } else {
                at += 1;
            }
        }
    }
    out
}

/// Reads the count-prefixed run at `at`, staying inside `end`.
fn read_list(frame: &[u8], at: usize, end: usize) -> Option<Vec<u32>> {
    let count = usize::from(*frame.get(at)?);
    if !(MIN_LIST..=MAX_LIST).contains(&count) {
        return None;
    }
    if at + 1 + count * 4 > end {
        return None;
    }
    let mut eids = Vec::with_capacity(count);
    for i in 0..count {
        let eid = read_u32(frame, at + 1 + i * 4)?;
        if eid == 0 || eid >= MAX_EID {
            return None;
        }
        eids.push(eid);
    }
    Some(eids)
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

    fn list(count: u8, eids: &[u32]) -> Vec<u8> {
        let mut v = vec![count];
        for e in eids {
            v.extend_from_slice(&e.to_le_bytes());
        }
        v
    }

    #[test]
    fn finds_staff_lists_in_a_club_span() {
        // Liverpool's real day-one shape: ascending runs inside the record
        // body, Hulshoff (5837) among the members.
        let mut buf = vec![0u8; 16];
        buf.extend(list(5, &[126, 1432, 2003, 5837, 6753]));
        buf.extend(vec![0u8; 7]);
        buf.extend(list(5, &[2003, 3756, 3996, 4081, 4422]));
        let clubs = [(8usize, 366u32)];
        let found = scan_staff_lists(&buf, &clubs);
        assert_eq!(found.len(), 2);
        assert_eq!(found.first().unwrap().club_eid, 366);
        assert_eq!(found.first().unwrap().eids, vec![126, 1432, 2003, 5837, 6753]);
    }

    #[test]
    fn a_list_outside_every_club_span_is_ignored() {
        let mut buf = list(5, &[126, 1432, 2003, 5837, 6753]);
        buf.extend(vec![0u8; 32]);
        // The only club starts after the run ends.
        let clubs = [(buf.len() - 8, 366u32)];
        assert!(scan_staff_lists(&buf, &clubs).is_empty());
    }

    #[test]
    fn a_shuffled_aged_list_is_still_read() {
        // Staff turnover reorders the list the way transfers reorder squad
        // lists; order is not part of the shape.
        let mut buf = vec![0u8; 4];
        buf.extend(list(5, &[6753, 126, 5837, 1432, 2003]));
        let clubs = [(0usize, 366u32)];
        let found = scan_staff_lists(&buf, &clubs);
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().eids, vec![6753, 126, 5837, 1432, 2003]);
    }

    #[test]
    fn rejects_runs_with_out_of_range_ids() {
        let mut buf = vec![0u8; 4];
        buf.extend(list(5, &[126, 1432, MAX_EID, 5837, 6753]));
        let clubs = [(0usize, 366u32)];
        assert!(scan_staff_lists(&buf, &clubs).is_empty());
    }
}
