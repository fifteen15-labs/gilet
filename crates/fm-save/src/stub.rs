//! Stub people: squad fillers without a full person record.
//!
//! Lower-league squads sign generated players on non-contract terms, and the
//! save stores them as ~33-byte entity stubs rather than person records:
//!
//! ```text
//! [01|02] 40 10  00 00 00 00  [eid u32] [uid u32] [uid u32]  07 ...
//! ```
//!
//! The doubled uid is the same identity shape person and club records use;
//! the `07` tag byte separates these entries from the identically-headed
//! non-player objects inside person records (`SAVE_FORMAT.md` §3). The
//! fields after the tag are not yet decoded — a name id has not been
//! confirmed, so a stub is a presence, not a profile: enough for a squad
//! list to show the member exists instead of silently dropping them.

/// A squad filler with no person record — entity identity only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stub {
    /// Byte offset of the entry within its frame.
    pub offset: usize,
    /// Person entity id — what squad lists reference.
    pub eid: u32,
    /// Persistent id, in the generated-player band.
    pub uid: u32,
}

/// Person entity ids stay comfortably below this, matching the squad walk.
const MAX_EID: u32 = 3_000_000;

/// Scans a frame for stub entries.
#[must_use]
pub fn scan_stubs(frame: &[u8]) -> Vec<Stub> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 20 <= frame.len() {
        if !matches!(frame.get(at), Some(0x01 | 0x02))
            || frame.get(at + 1..at + 7) != Some(&[0x40, 0x10, 0, 0, 0, 0][..])
        {
            at += 1;
            continue;
        }
        let (Some(eid), Some(uid), Some(uid2)) = (
            read_u32(frame, at + 7),
            read_u32(frame, at + 11),
            read_u32(frame, at + 15),
        ) else {
            at += 1;
            continue;
        };
        if uid != uid2
            || uid == 0
            || uid == u32::MAX
            || eid == 0
            || eid >= MAX_EID
            || frame.get(at + 19) != Some(&0x07)
        {
            at += 1;
            continue;
        }
        out.push(Stub { offset: at, eid, uid });
        at += 19;
    }
    out
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn entry(tag: u8, eid: u32, uid: u32, uid2: u32, kind: u8) -> Vec<u8> {
        let mut v = vec![tag, 0x40, 0x10, 0, 0, 0, 0];
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid2.to_le_bytes());
        v.push(kind);
        v.extend_from_slice(&[0; 13]);
        v
    }

    #[test]
    fn finds_stub_entries_and_rejects_decoys() {
        let mut buf = vec![0u8; 8];
        buf.extend(entry(0x02, 97260, 2_002_084_497, 2_002_084_497, 0x07));
        buf.extend(entry(0x01, 97261, 2_002_084_498, 2_002_084_498, 0x07));
        // Decoys: uid not doubled, wrong kind tag, zero uid, oversized eid.
        buf.extend(entry(0x02, 97262, 5, 6, 0x07));
        buf.extend(entry(0x02, 97263, 7, 7, 0x03));
        buf.extend(entry(0x02, 97264, 0, 0, 0x07));
        buf.extend(entry(0x02, 9_999_999, 8, 8, 0x07));

        let stubs = scan_stubs(&buf);
        assert_eq!(
            stubs.iter().map(|s| s.eid).collect::<Vec<_>>(),
            vec![97260, 97261]
        );
        assert_eq!(stubs.first().unwrap().offset, 8);
        assert_eq!(stubs.first().unwrap().uid, 2_002_084_497);
    }
}
