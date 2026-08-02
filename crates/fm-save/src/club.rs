/// A club as stored in the save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Club {
    /// Byte offset of the record within its frame, used as a stable row key.
    pub offset: usize,
    /// The club's own identifier, e.g. 1075 for Manchester City. Note this is
    /// *not* what other tables reference — see [`Club::eid`] — and Manchester
    /// United carries the same 1075, so it is probably a city or region id.
    pub club_id: u32,
    /// Nation identifier — 139 for the English clubs, 145 for the German ones.
    /// Not yet resolved to a nation name.
    pub nation_id: u32,
    /// The club's database entity id — what the squad table references.
    /// Manchester City is 369, Arsenal 293. `None` when the record head does
    /// not carry the validated `[eid][uid][uid]` shape.
    pub eid: Option<u32>,
    /// The club's second identifier, repeated beside the entity id on disk.
    /// The squad table repeats it, which is how squad records are validated.
    pub uid: Option<u32>,
    /// Full name, e.g. `"Manchester City"`.
    pub name: String,
    /// Short name as FM displays it in tables, e.g. `"Man City"`.
    pub short_name: String,
}

/// Marks the end of a club header, immediately before the name length. The
/// byte before it is a per-club flags byte, not a constant: real saves carry
/// 0x00, 0x01, 0x10, 0x11, 0x12, 0x14 and 0x30 (Tottenham and Chelsea sit at
/// 0x12), so anchoring on `10 FF FF` silently dropped those clubs — and with
/// them their squads, which is how contracted first-teamers showed no club.
const SIGNATURE: [u8; 2] = [0xFF, 0xFF];

/// The flags byte a record must carry when its entity head does *not*
/// validate. A validated `[eid][uid][uid]` head is proof enough on its own;
/// without one, only the long-verified 0x10 shape is trusted, because the
/// two-string tail alone also matches commentary word lists elsewhere in the
/// file ("admirable", "amazing", ...).
const HEADLESS_FLAGS: u8 = 0x10;

const MIN_NAME: usize = 3;
const MAX_NAME: usize = 64;
const MAX_SHORT: usize = 32;

/// Offsets back from the name-length field for the header values.
const CLUB_ID_BACK: usize = 10;
const NATION_ID_BACK: usize = 14;

/// Offsets back from the name-length field for the entity-id head:
/// `[eid][uid][uid] 00 [nation] [FFFFFFFF] [nation] [nation] [club_id]`.
const EID_BACK: usize = 39;
const UID_BACK: usize = 35;
const UID2_BACK: usize = 31;
const HEAD_ZERO_BACK: usize = 27;
const NATION3_BACK: usize = 26;
const HEAD_FF_BACK: usize = 22;
const NATION2_BACK: usize = 18;

/// Scans a decompressed frame for club records.
///
/// Layout, reading backwards from the name length:
/// `.. u32 nation_id, u32 nation_id, u32 club_id, 3 bytes, 0x10 0xFF 0xFF,
/// u32 name_len, name, u32 short_len, short_name`.
#[must_use]
pub fn scan_clubs(frame: &[u8]) -> Vec<Club> {
    let mut out = Vec::new();
    let mut at = 0usize;

    while at + SIGNATURE.len() + 8 < frame.len() {
        if at == 0 || frame.get(at..at + SIGNATURE.len()) != Some(&SIGNATURE[..]) {
            at += 1;
            continue;
        }
        let len_at = at + SIGNATURE.len();
        let flags = frame.get(at.wrapping_sub(1)).copied();
        match parse_at(frame, len_at) {
            // A club is real when its entity head validates, whatever the
            // flags byte reads; a headless record is only trusted in the
            // long-verified 0x10 shape.
            Some(club) if club.eid.is_some() || flags == Some(HEADLESS_FLAGS) => {
                at = club.offset + club.name.len() + club.short_name.len() + 8;
                out.push(club);
            }
            _ => at += 1,
        }
    }

    out
}

fn parse_at(frame: &[u8], len_at: usize) -> Option<Club> {
    let name_len = read_u32(frame, len_at)? as usize;
    if !(MIN_NAME..=MAX_NAME).contains(&name_len) {
        return None;
    }
    let name_start = len_at + 4;
    let name = read_text(frame, name_start, name_len)?;

    let short_at = name_start + name_len;
    let short_len = read_u32(frame, short_at)? as usize;
    if !(2..=MAX_SHORT).contains(&short_len) {
        return None;
    }
    let short_name = read_text(frame, short_at + 4, short_len)?;

    // A club name starts with a capital; the commentary lists are lowercase.
    if !starts_upper(&name) || !starts_upper(&short_name) {
        return None;
    }

    let (eid, uid) = parse_head(frame, len_at).unzip();

    Some(Club {
        offset: len_at,
        club_id: read_u32(frame, len_at.checked_sub(CLUB_ID_BACK)?)?,
        nation_id: read_u32(frame, len_at.checked_sub(NATION_ID_BACK)?)?,
        eid,
        uid,
        name,
        short_name,
    })
}

/// Reads the `[eid][uid][uid]` entity head, validating the fixed shape around
/// it: the uid repeated, a zero byte, the nation id three times with
/// `FFFFFFFF` between the first two copies. A record without that exact shape
/// gets no entity id rather than a guessed one.
fn parse_head(frame: &[u8], len_at: usize) -> Option<(u32, u32)> {
    let nation1 = read_u32(frame, len_at.checked_sub(NATION_ID_BACK)?)?;
    let nation2 = read_u32(frame, len_at.checked_sub(NATION2_BACK)?)?;
    let nation3 = read_u32(frame, len_at.checked_sub(NATION3_BACK)?)?;
    let ff = read_u32(frame, len_at.checked_sub(HEAD_FF_BACK)?)?;
    let zero = *frame.get(len_at.checked_sub(HEAD_ZERO_BACK)?)?;
    if nation1 != nation2 || nation2 != nation3 || ff != 0xFFFF_FFFF || zero != 0 {
        return None;
    }
    let uid = read_u32(frame, len_at.checked_sub(UID_BACK)?)?;
    let uid2 = read_u32(frame, len_at.checked_sub(UID2_BACK)?)?;
    let eid = read_u32(frame, len_at.checked_sub(EID_BACK)?)?;
    (uid == uid2 && uid != 0 && uid != u32::MAX).then_some((eid, uid))
}

fn starts_upper(s: &str) -> bool {
    s.chars().next().is_some_and(char::is_uppercase)
}

fn read_text(frame: &[u8], at: usize, len: usize) -> Option<String> {
    let raw = frame.get(at..at.checked_add(len)?)?;
    let text = std::str::from_utf8(raw).ok()?;
    if text.chars().any(char::is_control) || !text.chars().any(char::is_alphabetic) {
        return None;
    }
    Some(text.to_owned())
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn record(nation: u32, club_id: u32, name: &str, short: &str) -> Vec<u8> {
        record_flagged(369, 678, nation, club_id, name, short, HEADLESS_FLAGS)
    }

    fn record_with_head(eid: u32, uid: u32, nation: u32, club_id: u32, name: &str, short: &str) -> Vec<u8> {
        record_flagged(eid, uid, nation, club_id, name, short, HEADLESS_FLAGS)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_flagged(eid: u32, uid: u32, nation: u32, club_id: u32, name: &str, short: &str, flags: u8) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.push(0);
        v.extend_from_slice(&nation.to_le_bytes());
        v.extend_from_slice(&[0xFF; 4]);
        v.extend_from_slice(&nation.to_le_bytes());
        v.extend_from_slice(&nation.to_le_bytes());
        v.extend_from_slice(&club_id.to_le_bytes());
        v.extend_from_slice(&[0x00, 0x0A, 0x00]);
        v.push(flags);
        v.extend_from_slice(&SIGNATURE);
        v.extend_from_slice(&(name.len() as u32).to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(&(short.len() as u32).to_le_bytes());
        v.extend_from_slice(short.as_bytes());
        v
    }

    #[test]
    fn reads_a_club_with_its_ids() {
        // Manchester City's real values in the reference save.
        let buf = record(139, 1075, "Manchester City", "Man City");
        let found = scan_clubs(&buf);
        assert_eq!(found.len(), 1);
        let c = found.first().unwrap();
        assert_eq!(c.name, "Manchester City");
        assert_eq!(c.short_name, "Man City");
        assert_eq!(c.club_id, 1075);
        assert_eq!(c.nation_id, 139);
        assert_eq!(c.eid, Some(369));
        assert_eq!(c.uid, Some(678));
    }

    #[test]
    fn a_record_without_the_head_shape_gets_no_entity_id() {
        // Break the repeated-uid invariant; eid must come back None, not junk.
        let mut buf = record_with_head(369, 678, 139, 1075, "Manchester City", "Man City");
        *buf.get_mut(4).unwrap() ^= 0xFF;
        let found = scan_clubs(&buf);
        assert_eq!(found.first().unwrap().eid, None);
        assert_eq!(found.first().unwrap().uid, None);
    }

    #[test]
    fn reads_accented_club_names() {
        let buf = record(145, 541, "Club Atlético Boca Juniors", "Boca");
        assert_eq!(scan_clubs(&buf).first().unwrap().name, "Club Atlético Boca Juniors");
    }

    #[test]
    fn rejects_lowercase_word_pairs() {
        // The commentary word lists are the main false positive: without the
        // capital-letter test they yield thousands of ("admirable", "amazing").
        let buf = record(1, 1, "admirable", "amazing");
        assert!(scan_clubs(&buf).is_empty());
    }

    #[test]
    fn ignores_bytes_without_the_signature() {
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        // Break the FF FF pair; the record must no longer be recognised.
        let sig_at = buf.len() - 8 - 15 - 4 - 2;
        *buf.get_mut(sig_at).unwrap() = 0x11;
        assert!(scan_clubs(&buf).is_empty());
    }

    #[test]
    fn a_validated_head_carries_any_flags_byte() {
        // Tottenham's real record reads 0x12 where most clubs read 0x10; the
        // entity head still validates, and dropping the club dropped its whole
        // squad — Sarr and Bissouma showed no club despite live contracts.
        let buf = record_flagged(418, 727, 139, 1040, "Tottenham Hotspur", "Spurs", 0x12);
        let found = scan_clubs(&buf);
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().eid, Some(418));
        assert_eq!(found.first().unwrap().uid, Some(727));
    }

    #[test]
    fn a_headless_record_needs_the_trusted_flags_byte() {
        // Without a validating head the two-string tail also matches word
        // lists, so an unknown flags byte is not enough to admit the record.
        let mut buf = record_flagged(418, 727, 139, 1040, "Tottenham Hotspur", "Spurs", 0x12);
        *buf.get_mut(4).unwrap() ^= 0xFF; // break the repeated uid
        assert!(scan_clubs(&buf).is_empty());
    }

    #[test]
    fn finds_several_clubs_in_sequence() {
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        buf.extend(record(139, 1040, "Arsenal", "Arsenal"));
        let found = scan_clubs(&buf);
        assert_eq!(found.len(), 2);
        assert_eq!(found.get(1).unwrap().club_id, 1040);
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let full = record(139, 1075, "Manchester City", "Man City");
        for cut in 0..full.len() {
            let _ = scan_clubs(full.get(..cut).unwrap());
        }
    }
}
