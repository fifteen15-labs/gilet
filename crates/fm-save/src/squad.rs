//! The squad table: which people play for which club.
//!
//! One record per club, in club entity-id order:
//!
//! ```text
//! [club_eid u32] [00 x10] [u32] [club_uid u32] [club_uid u32]
//!   ... variable fields ...
//!   FF FF FF FF [count u16] [count x u32 person_eid]
//!   [captain_eid u32] [vice_captain_eid u32]
//!   ... trailing fields ...
//! ```
//!
//! A record head is only accepted when its uid equals the uid the *club
//! table* carries for the same entity id — two independent tables agreeing is
//! what makes the walk trustworthy. The player list is the first
//! `FF FF FF FF`-marked count block whose entries look like person entity ids;
//! entries are stored ascending with new signings appended at the tail, which
//! is the shape test that rejects coincidental count bytes.

use std::collections::HashMap;

/// One club's first-team squad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Squad {
    /// Byte offset of the record within its frame.
    pub offset: usize,
    /// Entity id of the club, matching [`crate::Club::eid`].
    pub club_eid: u32,
    /// Person entity ids of the squad, matching [`crate::Person::eid`].
    pub player_eids: Vec<u32>,
    /// Person entity id of the club captain, when one is set.
    pub captain_eid: Option<u32>,
    /// Person entity id of the vice-captain, when one is set.
    pub vice_captain_eid: Option<u32>,
}

/// Longest squad list a record may declare. Real first-team squads top out in
/// the forties; anything larger is a misread count.
const MAX_SQUAD: usize = 80;

/// Person entity ids stay comfortably below this.
const MAX_EID: u32 = 3_000_000;

/// A record body never runs further than this before the next head.
const MAX_RECORD: usize = 6_000;

/// Scans a frame for the squad table.
///
/// `club_ids` are the `(eid, uid)` pairs read from the club table; only heads
/// matching a known pair are accepted, and of those only the longest run of
/// strictly ascending entity ids — the table itself — is kept.
#[must_use]
pub fn scan_squads(frame: &[u8], club_ids: &[(u32, u32)]) -> Vec<Squad> {
    let by_eid: HashMap<u32, u32> = club_ids.iter().copied().collect();

    let mut heads: Vec<(usize, u32)> = Vec::new();
    let mut at = 0usize;
    while at + 26 <= frame.len() {
        if let Some(eid) = head_at(frame, at, &by_eid) {
            heads.push((at, eid));
            at += 26;
        } else {
            at += 1;
        }
    }

    let table = longest_ascending_run(&heads);

    let mut out = Vec::new();
    for (i, &(offset, club_eid)) in table.iter().enumerate() {
        let end = table
            .get(i + 1)
            .map_or(frame.len(), |&(next, _)| next)
            .min(offset + MAX_RECORD);
        let (player_eids, captain_eid, vice_captain_eid) = read_list(frame, offset + 26, end);
        if player_eids.is_empty() {
            continue;
        }
        out.push(Squad {
            offset,
            club_eid,
            player_eids,
            captain_eid,
            vice_captain_eid,
        });
    }
    out
}

/// `[eid][00 x10][u32][uid][uid]` where `(eid, uid)` matches the club table.
/// The zero run is tested first: it fails within a byte or two almost
/// everywhere, which is what keeps a whole-frame scan cheap.
fn head_at(frame: &[u8], at: usize, by_eid: &HashMap<u32, u32>) -> Option<u32> {
    if frame.get(at + 4..at + 14)? != [0u8; 10] {
        return None;
    }
    let eid = read_u32(frame, at)?;
    let want_uid = *by_eid.get(&eid)?;
    let uid = read_u32(frame, at + 18)?;
    let uid2 = read_u32(frame, at + 22)?;
    (uid == want_uid && uid2 == want_uid).then_some(eid)
}

/// The longest subsequence of heads whose entity ids strictly ascend, in file
/// order — patience LIS. False heads are isolated; the table is thousands
/// long, so it wins the race and stray patterns fall out.
fn longest_ascending_run(heads: &[(usize, u32)]) -> Vec<(usize, u32)> {
    let mut tails_eid: Vec<u32> = Vec::new();
    let mut tails_idx: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; heads.len()];

    for (i, &(_, eid)) in heads.iter().enumerate() {
        let k = tails_eid.partition_point(|&e| e < eid);
        if let Some(slot) = prev.get_mut(i) {
            *slot = k.checked_sub(1).and_then(|j| tails_idx.get(j).copied());
        }
        if k == tails_eid.len() {
            tails_eid.push(eid);
            tails_idx.push(i);
        } else if tails_eid.get(k).is_some_and(|&e| eid < e) {
            if let (Some(te), Some(ti)) = (tails_eid.get_mut(k), tails_idx.get_mut(k)) {
                *te = eid;
                *ti = i;
            }
        }
    }

    let mut chain = Vec::new();
    let mut cur = tails_idx.last().copied();
    while let Some(i) = cur {
        if let Some(h) = heads.get(i) {
            chain.push(*h);
        }
        cur = prev.get(i).copied().flatten();
    }
    chain.reverse();
    chain
}

/// Finds the player list between `from` and `end`: `FF FF FF FF`, a u16
/// count, then that many person entity ids, mostly ascending. Returns the
/// list and the captain pair that follows it.
fn read_list(frame: &[u8], from: usize, end: usize) -> (Vec<u32>, Option<u32>, Option<u32>) {
    let mut at = from;
    while at + 6 < end {
        if frame.get(at..at + 4) != Some(&[0xFF; 4]) {
            at += 1;
            continue;
        }
        let Some(count) = read_u16(frame, at + 4).map(usize::from) else {
            break;
        };
        if !(1..=MAX_SQUAD).contains(&count) {
            at += 1;
            continue;
        }
        let list_at = at + 6;
        let Some(list_end) = list_at.checked_add(count * 4) else {
            break;
        };
        if list_end > end {
            at += 1;
            continue;
        }
        let mut eids = Vec::with_capacity(count);
        for i in 0..count {
            match read_u32(frame, list_at + i * 4) {
                Some(v) if v > 0 && v < MAX_EID => eids.push(v),
                _ => break,
            }
        }
        if eids.len() != count {
            at += 1;
            continue;
        }
        let captain = read_u32(frame, list_end).filter(|&v| v != u32::MAX && v != 0);
        let vice = read_u32(frame, list_end + 4).filter(|&v| v != u32::MAX && v != 0);
        // Two shapes prove the list real. A fresh save writes it ascending
        // with signings appended; a decade of transfers destroys that order
        // entirely, but the captain and vice that follow are still members of
        // the list. Either signal is accepted.
        let captain_linked = captain.is_some_and(|v| eids.contains(&v))
            || vice.is_some_and(|v| eids.contains(&v));
        if !mostly_ascending(&eids) && !captain_linked {
            at += 1;
            continue;
        }
        return (eids, captain, vice);
    }
    (Vec::new(), None, None)
}

/// The stored list ascends except for signings appended at the tail, so the
/// first few entries are the shape test: nearly all must increase.
fn mostly_ascending(eids: &[u32]) -> bool {
    let pairs = eids.len().saturating_sub(1).min(6);
    if pairs == 0 {
        return true;
    }
    let ascending = eids
        .windows(2)
        .take(pairs)
        .filter(|w| matches!(w, [a, b] if a < b))
        .count();
    ascending >= pairs - 1
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn record(eid: u32, uid: u32, players: &[u32], captain: u32, vice: u32) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&[0u8; 10]);
        v.extend_from_slice(&(eid + 131).to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&[0x0A, 0x00, 0x03]); // opaque mid-record bytes
        v.extend_from_slice(&[0xFF; 4]);
        v.extend_from_slice(&(players.len() as u16).to_le_bytes());
        for p in players {
            v.extend_from_slice(&p.to_le_bytes());
        }
        v.extend_from_slice(&captain.to_le_bytes());
        v.extend_from_slice(&vice.to_le_bytes());
        v.extend_from_slice(&[0u8; 16]);
        v
    }

    #[test]
    fn reads_a_squad_with_its_captains() {
        // Shaped like Manchester City's real record: eid 369, uid 678,
        // ascending list, captain and vice at the tail.
        let clubs = [(369u32, 678u32), (370, 679)];
        let mut buf = record(369, 678, &[5292, 6961, 10241, 14042, 14078], 14042, 14078);
        buf.extend(record(370, 679, &[3780, 12236], 12236, 5779));

        let squads = scan_squads(&buf, &clubs);
        assert_eq!(squads.len(), 2);
        let city = squads.first().unwrap();
        assert_eq!(city.club_eid, 369);
        assert_eq!(city.player_eids, vec![5292, 6961, 10241, 14042, 14078]);
        assert_eq!(city.captain_eid, Some(14042));
        assert_eq!(city.vice_captain_eid, Some(14078));
    }

    #[test]
    fn an_unset_captain_reads_as_none() {
        let clubs = [(369u32, 678u32), (370, 679)];
        let mut buf = record(369, 678, &[100, 200], u32::MAX, u32::MAX);
        buf.extend(record(370, 679, &[300, 400], 300, u32::MAX));
        let squads = scan_squads(&buf, &clubs);
        assert_eq!(squads.first().unwrap().captain_eid, None);
        assert_eq!(squads.get(1).unwrap().vice_captain_eid, None);
    }

    #[test]
    fn rejects_a_head_whose_uid_disagrees_with_the_club_table() {
        let clubs = [(369u32, 678u32)];
        let buf = record(369, 999, &[100, 200], 100, 200);
        assert!(scan_squads(&buf, &clubs).is_empty());
    }

    #[test]
    fn rejects_a_shuffled_list_whose_captains_are_strangers() {
        // Neither ascending nor captain-linked: not a squad list.
        let clubs = [(369u32, 678u32), (370, 679)];
        let mut buf = record(369, 678, &[900, 700, 500, 300, 100, 50, 20], 12345, 54321);
        buf.extend(record(370, 679, &[300, 400], 300, 400));
        let squads = scan_squads(&buf, &clubs);
        assert!(squads.iter().all(|s| s.club_eid != 369));
    }

    #[test]
    fn accepts_a_shuffled_list_when_the_captain_is_a_member() {
        // A decade of transfers destroys the stored order — Liverpool's real
        // 2035 list starts 24359, 15005, 10164 — but the captain that follows
        // is still in the list, which is the signal that keeps it.
        let clubs = [(366u32, 675u32)];
        let buf = record(366, 675, &[24359, 15005, 10164, 24635, 44162], 10164, 44162);
        let squads = scan_squads(&buf, &clubs);
        assert_eq!(squads.len(), 1);
        assert_eq!(squads.first().unwrap().captain_eid, Some(10164));
    }

    #[test]
    fn keeps_the_longest_ascending_run_of_heads() {
        // A stray head-shaped pattern with an out-of-order eid must not derail
        // the walk through the real table.
        let clubs = [(10u32, 11u32), (20, 21), (30, 31), (5, 6)];
        let mut buf = record(10, 11, &[100, 200], 100, 200);
        buf.extend(record(20, 21, &[300, 400], 300, 400));
        buf.extend(record(5, 6, &[500, 600], 500, 600)); // breaks the order
        buf.extend(record(30, 31, &[700, 800], 700, 800));
        let squads = scan_squads(&buf, &clubs);
        let eids: Vec<u32> = squads.iter().map(|s| s.club_eid).collect();
        assert_eq!(eids, vec![10, 20, 30]);
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let clubs = [(369u32, 678u32)];
        let full = record(369, 678, &[100, 200, 300], 100, 200);
        for cut in 0..full.len() {
            let _ = scan_squads(full.get(..cut).unwrap(), &clubs);
        }
    }
}
