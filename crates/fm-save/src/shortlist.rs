//! The human manager's in-game shortlists, from the `scout_man.dat` member.
//!
//! Decoded 2 August 2026 against a probe save whose shortlists were known
//! (`ZZPROBE` = van Dijk, Wirtz, Salah). Each shortlist is a record:
//!
//! ```text
//! ... record head ...
//! FF FF FF FF  01 01 00  6c 07          header run (6c 07 = year 1900)
//! [u32 len] [name bytes]                the shortlist's name; len 0 = unnamed
//! ... filter block ("frlp"/"tlif") ...
//! "rSrP" 00 [u32 count]                 the player list
//! per entry, 22 bytes:
//!   02                                  entity type tag
//!   u32 person eid
//!   u32 date added — high u16 is the year, low u16 undecoded
//!   u32 0
//!   01 00 6c 07                         null date (day 1, year 1900)
//!   FF FF FF FF
//!   00
//! ... 75-entry column-id list for the shortlist view ...
//! ```
//!
//! The scan anchors on `rSrP` for the list and takes the record's name from
//! the last header run before it. Records without an `rSrP` block — scouting
//! focuses carry the same head — contribute nothing, which is correct.

/// One in-game shortlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameShortlist {
    /// The name the user gave it; `None` for the unnamed default list.
    pub name: Option<String>,
    /// Members as person entity ids, matching [`crate::Person::eid`].
    pub person_eids: Vec<u32>,
}

/// `"rSrP"`, the tag before each player list.
const LIST_TAG: [u8; 4] = *b"rSrP";

/// `01 01 00` + year 1900: the run that ends a record head, with the name
/// immediately after.
const NAME_ANCHOR: [u8; 5] = [0x01, 0x01, 0x00, 0x6c, 0x07];

/// Stored size of one player entry.
const ENTRY_LEN: usize = 22;

/// Entity type tag opening every player entry.
const PERSON_TAG: u8 = 0x02;

/// Person entity ids stay comfortably below this (same bound as the squad
/// table).
const MAX_EID: u32 = 3_000_000;

/// More players than any real shortlist; a larger count is a misread.
const MAX_LIST: usize = 5_000;

/// A shortlist name longer than this is a misread, not a name.
const MAX_NAME: usize = 64;

/// Scans the decompressed `scout_man.dat` frame for the human's shortlists.
///
/// Empty lists are kept — an empty named shortlist is still a shortlist — and
/// a block whose entries do not all parse is dropped whole rather than
/// half-read.
#[must_use]
pub fn scan_shortlists(frame: &[u8]) -> Vec<GameShortlist> {
    let mut out = Vec::new();
    let mut span_start = 0usize;
    let mut at = 0usize;

    while let Some(found) = find(frame, &LIST_TAG, at) {
        if let Some(eids) = read_list(frame, found + LIST_TAG.len()) {
            out.push(GameShortlist {
                name: read_name(frame, span_start, found),
                person_eids: eids,
            });
        }
        span_start = found;
        at = found + LIST_TAG.len();
    }
    out
}

fn find(frame: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    frame
        .get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| from + p)
}

/// `00 [u32 count]` then `count` entries of [`ENTRY_LEN`] bytes.
fn read_list(frame: &[u8], at: usize) -> Option<Vec<u32>> {
    if frame.get(at) != Some(&0x00) {
        return None;
    }
    let count = read_u32(frame, at + 1)? as usize;
    if count > MAX_LIST {
        return None;
    }
    let mut eids = Vec::with_capacity(count);
    let mut entry = at + 5;
    for _ in 0..count {
        if frame.get(entry) != Some(&PERSON_TAG) {
            return None;
        }
        let eid = read_u32(frame, entry + 1)?;
        if eid == 0 || eid >= MAX_EID {
            return None;
        }
        eids.push(eid);
        entry = entry.checked_add(ENTRY_LEN)?;
    }
    Some(eids)
}

/// The name after the last [`NAME_ANCHOR`] between `from` and `to` — the head
/// of the record whose list starts at `to`. Earlier anchors in the span belong
/// to listless records and are skipped by taking the last.
fn read_name(frame: &[u8], from: usize, to: usize) -> Option<String> {
    let span = frame.get(from..to)?;
    let anchor = span
        .windows(NAME_ANCHOR.len())
        .rposition(|w| w == NAME_ANCHOR)?;
    let len_at = from + anchor + NAME_ANCHOR.len();
    let len = read_u32(frame, len_at)? as usize;
    if len == 0 || len > MAX_NAME {
        return None;
    }
    let start = len_at + 4;
    let bytes = frame.get(start..start.checked_add(len)?)?;
    let name = std::str::from_utf8(bytes).ok()?;
    Some(name.to_owned())
}

/// The byte span of one list inside the frame: the count field and the
/// entries that follow it, plus what the list holds.
struct ListSpan {
    /// Offset of the u32 count.
    count_at: usize,
    /// Offset just past the last entry — where a new one goes.
    end: usize,
    eids: Vec<u32>,
}

/// Locates the named list's `rSrP` block the same way [`scan_shortlists`]
/// walks them, so read and write agree about which list is which.
fn find_list(frame: &[u8], name: Option<&str>) -> Option<ListSpan> {
    let mut span_start = 0usize;
    let mut at = 0usize;
    while let Some(found) = find(frame, &LIST_TAG, at) {
        let list_at = found + LIST_TAG.len();
        if let Some(eids) = read_list(frame, list_at) {
            if read_name(frame, span_start, found).as_deref() == name {
                let count_at = list_at + 1;
                return Some(ListSpan {
                    count_at,
                    end: count_at + 4 + eids.len() * ENTRY_LEN,
                    eids,
                });
            }
        }
        span_start = found;
        at = found + LIST_TAG.len();
    }
    None
}

/// The date-added field for a new entry: FM's masked day-of-year pair
/// (`SAVE_FORMAT.md` §1c) — day of year in the low nine bits, then the year.
/// The high seven bits vary in the wild (0, 13 and 41 observed) and are not
/// understood; zero is written because it is an attested value, not a guess
/// at the field's meaning.
#[must_use]
pub fn date_added_bytes(date: crate::Date) -> [u8; 4] {
    let [d0, d1] = (date.day_of_year() & 0x01FF).to_le_bytes();
    let [y0, y1] = date.year.to_le_bytes();
    [d0, d1, y0, y1]
}

/// One serialised player entry. The date comes from the caller — normally
/// [`date_added_bytes`] of the save's own current date.
fn entry_bytes(eid: u32, date: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ENTRY_LEN);
    out.push(PERSON_TAG);
    out.extend_from_slice(&eid.to_le_bytes());
    out.extend_from_slice(&date);
    out.extend_from_slice(&[0u8; 4]);
    out.extend_from_slice(&[0x01, 0x00, 0x6c, 0x07]); // null date
    out.extend_from_slice(&[0xFF; 4]);
    out.push(0x00);
    out
}

/// Returns the frame with `eid` appended to the named list (`None` = the
/// unnamed default list), or unchanged bytes when the player is already on
/// it. `None` when no such list exists.
///
/// New entries go at the tail, which is where FM itself appends.
#[must_use]
pub fn add_entry(frame: &[u8], name: Option<&str>, eid: u32, date: [u8; 4]) -> Option<Vec<u8>> {
    let list = find_list(frame, name)?;
    if list.eids.contains(&eid) {
        return Some(frame.to_vec());
    }
    let count = u32::try_from(list.eids.len() + 1).ok()?;
    let mut out = Vec::with_capacity(frame.len() + ENTRY_LEN);
    out.extend_from_slice(frame.get(..list.count_at)?);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(frame.get(list.count_at + 4..list.end)?);
    out.extend_from_slice(&entry_bytes(eid, date));
    out.extend_from_slice(frame.get(list.end..)?);
    Some(out)
}

/// Returns the frame with `eid` removed from the named list, unchanged bytes
/// when the player is not on it, or `None` when no such list exists.
#[must_use]
pub fn remove_entry(frame: &[u8], name: Option<&str>, eid: u32) -> Option<Vec<u8>> {
    let list = find_list(frame, name)?;
    let Some(index) = list.eids.iter().position(|&e| e == eid) else {
        return Some(frame.to_vec());
    };
    let count = u32::try_from(list.eids.len() - 1).ok()?;
    let entry_at = list.count_at + 4 + index * ENTRY_LEN;
    let mut out = Vec::with_capacity(frame.len() - ENTRY_LEN);
    out.extend_from_slice(frame.get(..list.count_at)?);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(frame.get(list.count_at + 4..entry_at)?);
    out.extend_from_slice(frame.get(entry_at + ENTRY_LEN..)?);
    Some(out)
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn entry(eid: u32) -> Vec<u8> {
        let mut v = vec![PERSON_TAG];
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&0x07EA_1A9Fu32.to_le_bytes()); // date added
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&[0x01, 0x00, 0x6c, 0x07]); // null date
        v.extend_from_slice(&[0xFF; 4]);
        v.push(0x00);
        v
    }

    fn record(name: &str, eids: &[u32]) -> Vec<u8> {
        let mut v = vec![0x1e, 0x00, 0x06, 0xf8, 0x43, 0x01, 0x00, 0x01];
        v.extend_from_slice(&[0xFF; 4]);
        v.extend_from_slice(&NAME_ANCHOR);
        v.extend_from_slice(&(name.len() as u32).to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(b"\x00\x00\x01frlptlif\x00"); // filter block stand-in
        v.extend_from_slice(&LIST_TAG);
        v.push(0x00);
        v.extend_from_slice(&(eids.len() as u32).to_le_bytes());
        for &e in eids {
            v.extend(entry(e));
        }
        v.extend_from_slice(&[0u8; 8]);
        v
    }

    #[test]
    fn reads_named_shortlists_with_their_members() {
        // Shaped like the probe save: ZZPROBE = van Dijk, Wirtz, Salah.
        let mut buf = b"\x03\x01tad.\x1f\x00".to_vec();
        buf.extend(record("ZZPROBE", &[13264, 20989, 21563]));
        buf.extend(record("WirtzNew", &[20989, 6421, 21563]));

        let lists = scan_shortlists(&buf);
        assert_eq!(lists.len(), 2);
        assert_eq!(lists.first().unwrap().name.as_deref(), Some("ZZPROBE"));
        assert_eq!(lists.first().unwrap().person_eids, vec![13264, 20989, 21563]);
        assert_eq!(lists.get(1).unwrap().name.as_deref(), Some("WirtzNew"));
        assert_eq!(lists.get(1).unwrap().person_eids, vec![20989, 6421, 21563]);
    }

    #[test]
    fn an_unnamed_list_reads_as_none() {
        let buf = record("", &[32525]);
        let lists = scan_shortlists(&buf);
        assert_eq!(lists.len(), 1);
        let list = lists.first().unwrap();
        assert_eq!(list.name, None);
        assert_eq!(list.person_eids, vec![32525]);
    }

    #[test]
    fn an_empty_list_is_kept() {
        let lists = scan_shortlists(&record("Targets", &[]));
        assert_eq!(lists.len(), 1);
        assert_eq!(lists.first().unwrap().name.as_deref(), Some("Targets"));
        assert!(lists.first().unwrap().person_eids.is_empty());
    }

    #[test]
    fn a_listless_record_between_two_lists_does_not_steal_the_name() {
        // A scouting focus carries the same head but no rSrP; the second
        // list's name must come from its own head, not the focus's.
        let mut buf = record("First", &[100]);
        let mut focus = vec![0x1e, 0x00, 0x06, 0xf8, 0x43, 0x01];
        focus.extend_from_slice(&NAME_ANCHOR);
        focus.extend_from_slice(&0u32.to_le_bytes());
        focus.extend_from_slice(b"\x01flpntlif\x00manP\x00\x00");
        buf.extend(focus);
        buf.extend(record("Second", &[200]));

        let lists = scan_shortlists(&buf);
        assert_eq!(lists.len(), 2);
        assert_eq!(lists.get(1).unwrap().name.as_deref(), Some("Second"));
    }

    #[test]
    fn rejects_a_block_whose_entries_do_not_parse() {
        // A stray rSrP over non-entry bytes must not produce a phantom list.
        let mut buf = b"noise ".to_vec();
        buf.extend_from_slice(&LIST_TAG);
        buf.extend_from_slice(&[0x00, 0x02, 0x00, 0x00, 0x00]); // count 2
        buf.extend_from_slice(&[0x55; 44]); // wrong type tags
        assert!(scan_shortlists(&buf).is_empty());
    }

    #[test]
    fn tolerates_truncation_anywhere() {
        let full = record("ZZPROBE", &[13264, 20989]);
        for cut in 0..full.len() {
            let _ = scan_shortlists(full.get(..cut).unwrap());
        }
    }

    const DATE: [u8; 4] = [0x9F, 0x1A, 0xEA, 0x07];

    #[test]
    fn add_entry_appends_and_bumps_the_count() {
        let mut buf = record("First", &[100]);
        buf.extend(record("Second", &[200, 300]));

        let edited = add_entry(&buf, Some("Second"), 999, DATE).unwrap();
        let lists = scan_shortlists(&edited);
        assert_eq!(lists.first().unwrap().person_eids, vec![100]);
        assert_eq!(lists.get(1).unwrap().person_eids, vec![200, 300, 999]);
        // Everything outside the edited list is untouched.
        assert_eq!(edited.len(), buf.len() + ENTRY_LEN);
    }

    #[test]
    fn adding_a_player_already_listed_changes_nothing() {
        let buf = record("First", &[100, 200]);
        assert_eq!(add_entry(&buf, Some("First"), 200, DATE).unwrap(), buf);
    }

    #[test]
    fn add_entry_to_a_missing_list_is_none() {
        let buf = record("First", &[100]);
        assert!(add_entry(&buf, Some("Absent"), 999, DATE).is_none());
        assert!(add_entry(&buf, None, 999, DATE).is_none());
    }

    #[test]
    fn remove_entry_splices_out_the_member() {
        let mut buf = record("First", &[100]);
        buf.extend(record("Second", &[200, 300, 400]));

        let edited = remove_entry(&buf, Some("Second"), 300).unwrap();
        let lists = scan_shortlists(&edited);
        assert_eq!(lists.get(1).unwrap().person_eids, vec![200, 400]);
        assert_eq!(remove_entry(&edited, Some("Second"), 999).unwrap(), edited);
    }

    #[test]
    fn edits_target_the_unnamed_list_only_when_asked_to() {
        let mut buf = record("", &[100]);
        buf.extend(record("Named", &[200]));

        let edited = add_entry(&buf, None, 999, DATE).unwrap();
        let lists = scan_shortlists(&edited);
        assert_eq!(lists.first().unwrap().person_eids, vec![100, 999]);
        assert_eq!(lists.get(1).unwrap().person_eids, vec![200]);
    }

    #[test]
    fn a_round_trip_add_then_remove_restores_the_bytes() {
        let buf = record("First", &[100, 200]);
        let added = add_entry(&buf, Some("First"), 999, DATE).unwrap();
        assert_eq!(remove_entry(&added, Some("First"), 999).unwrap(), buf);
    }

    #[test]
    fn date_added_bytes_carries_the_day_in_the_low_nine_bits() {
        // 8 June 2026 is day 159 — the value the probe save's own entries
        // carry in their low nine bits.
        let date = crate::Date { year: 2026, month: 6, day: 8 };
        let bytes = date_added_bytes(date);
        let [b0, b1, b2, b3] = bytes;
        let day_half = u16::from_le_bytes([b0, b1]);
        assert_eq!(day_half & 0x01FF, 159);
        assert_eq!(u16::from_le_bytes([b2, b3]), 2026);
    }
}
