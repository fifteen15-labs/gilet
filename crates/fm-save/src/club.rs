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
    /// The club's reputation, 0-10000 as the editor scales it, from the
    /// roster table (`backroom::scan_club_reputations`). `None` when the club
    /// has no roster row — an undecoded reputation, not a lowly one. Where a
    /// club fields several competition rows the highest is taken, which is its
    /// senior-league standing.
    pub reputation: Option<u16>,
}

/// The three bytes before the name length in the long-verified club shape:
/// the 0x10 flags byte and the `FF FF` that was read as a fixed signature.
/// None of the three is constant — the flags byte carries 0x00, 0x01, 0x10,
/// 0x11, 0x12, 0x14 and 0x30, and the pair behind it is per-club too
/// (Heybridge Swifts reads `10 FF 00`) — so the scan no longer anchors on it.
/// It is kept as the one shape trusted for a record whose entity head does
/// *not* validate: two consecutive length-prefixed strings is not a specific
/// enough test on its own, because the file also holds the commentary word
/// lists ("admirable", "amazing", ...).
const HEADLESS_TAIL: [u8; 3] = [0x10, 0xFF, 0xFF];

const MIN_NAME: usize = 3;
const MAX_NAME: usize = 64;
const MAX_SHORT: usize = 32;

/// Offsets back from the name-length field for the header values.
const CLUB_ID_BACK: usize = 10;
const NATION_ID_BACK: usize = 14;

/// Offsets back from the name-length field for the entity-id head:
/// `[eid][uid][uid] 00 [nation] [FFFFFFFF] [location] [nation] [club_id]`.
///
/// The field at [`NATION_LOCATION_BACK`] was long read as a third copy of the
/// nation id, and on 99.8% of clubs it is one — but it is a separate value:
/// the country the club sits in, where the other two are the pyramid it plays
/// in. Requiring all three to match dropped the entity head of every
/// cross-border club, and with it the squad link for their whole first team.
const EID_BACK: usize = 39;
const UID_BACK: usize = 35;
const UID2_BACK: usize = 31;
const HEAD_ZERO_BACK: usize = 27;
const NATION3_BACK: usize = 26;
const HEAD_FF_BACK: usize = 22;
const NATION_LOCATION_BACK: usize = 18;

/// The largest plausible nation id. The location field is no longer required
/// to equal the club's own nation, so it is bounded instead — that keeps the
/// head shape discriminating without asserting the two are the same country.
/// Real ids in a full database run to the low hundreds.
const MAX_NATION_ID: u32 = 10_000;

/// Scans a decompressed frame for club records.
///
/// Layout, reading backwards from the name length:
/// `.. u32 nation_id, u32 nation_id, u32 club_id, 3 bytes, 3 more bytes,
/// u32 name_len, name, u32 short_len, short_name`.
///
/// The anchor is the **entity head**, not those last three bytes. They were
/// read as `[flags] FF FF`, but the pair behind the flags is per-club as well:
/// Heybridge Swifts reads `10 FF 00`, and 592 head-validated clubs in that
/// save sit behind a pair that is not `FF FF`. A dropped club takes its squad
/// with it, so the user's own club showed nothing at all. A record whose head
/// does not validate must still carry [`HEADLESS_TAIL`].
#[must_use]
pub fn scan_clubs(frame: &[u8]) -> Vec<Club> {
    let mut out = Vec::new();
    let mut len_at = 0usize;

    while len_at + 8 < frame.len() {
        // A club is real when its entity head validates, whatever the three
        // bytes before the name length read; a headless record is only trusted
        // in the long-verified `10 FF FF` shape. Both are byte compares, and
        // they gate the string parsing: reading two length-prefixed strings at
        // every offset of a 285 MB frame costs a minute of wall clock for the
        // same answer.
        let head = parse_head(frame, len_at);
        if head.is_none() && !has_headless_tail(frame, len_at) {
            len_at += 1;
            continue;
        }
        match parse_at(frame, len_at, head) {
            Some(club) => {
                len_at = club.offset + club.name.len() + club.short_name.len() + 8;
                out.push(club);
            }
            None => len_at += 1,
        }
    }

    out
}

/// A club's boardroom, read from the run right after the short name:
/// `01 [u16] [u32 director-of-football eid] [flag] [count] [count x u32
/// board eids] 01`. Verified against reality on the 2035 index save — Leca
/// as Lens' sporting director with Oughourlian on the board, Viana at
/// Manchester City, Cavenagh chairing Rangers (`SAVE_FORMAT.md` §4). Only
/// clubs carrying this exact byte shape parse; the variants are unmapped
/// and yield nothing rather than a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boardroom {
    /// The club's entity id.
    pub club_eid: u32,
    /// The director of football's person eid. `None` when the seat is
    /// vacant (0 or `FFFFFFFF` on disk).
    pub dof_eid: Option<u32>,
    /// The board members' person eids, chair among them, order as stored.
    pub board_eids: Vec<u32>,
}

/// Person entity ids stay comfortably below this, matching the squad walk.
const MAX_PERSON_EID: u32 = 3_000_000;

/// No board list is longer than this in the exact shape.
const MAX_BOARD: usize = 8;

/// Reads each club's boardroom, where the exact shape holds.
///
/// The caller must still gate the result on the eids resolving to
/// non-player people — a byte shape alone must not hand a club a board.
#[must_use]
pub fn scan_boardrooms(frame: &[u8], clubs: &[Club]) -> Vec<Boardroom> {
    clubs
        .iter()
        .filter_map(|c| {
            let body = c
                .offset
                .checked_add(8 + c.name.len() + c.short_name.len())?;
            read_boardroom(frame, body, c.eid?)
        })
        .collect()
}

/// Parses the boardroom run at `body`, refusing anything but the exact
/// shape: the `01` sentinels at both ends, a plausible seat id or an
/// explicit vacant marker, and every board id in person-eid range.
fn read_boardroom(frame: &[u8], body: usize, club_eid: u32) -> Option<Boardroom> {
    if frame.get(body) != Some(&0x01) {
        return None;
    }
    let seat = read_u32(frame, body.checked_add(3)?)?;
    let dof_eid = match seat {
        0 | u32::MAX => None,
        e if e < MAX_PERSON_EID => Some(e),
        _ => return None,
    };
    let count = usize::from(*frame.get(body.checked_add(8)?)?);
    if count > MAX_BOARD {
        return None;
    }
    let mut board_eids = Vec::with_capacity(count);
    for i in 0..count {
        let eid = read_u32(frame, body.checked_add(9 + i * 4)?)?;
        if eid == 0 || eid >= MAX_PERSON_EID {
            return None;
        }
        board_eids.push(eid);
    }
    if frame.get(body.checked_add(9 + count * 4)?) != Some(&0x01) {
        return None;
    }
    Some(Boardroom {
        club_eid,
        dof_eid,
        board_eids,
    })
}

/// Whether the three bytes before the name length are the long-verified
/// headless shape.
fn has_headless_tail(frame: &[u8], len_at: usize) -> bool {
    len_at
        .checked_sub(HEADLESS_TAIL.len())
        .and_then(|from| frame.get(from..len_at))
        == Some(&HEADLESS_TAIL[..])
}

fn parse_at(frame: &[u8], len_at: usize, head: Option<(u32, u32)>) -> Option<Club> {
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

    // A club name starts with a capital or a digit — "1. FSV Mainz 05",
    // "1º de Agosto" and every other numbered club is as real as Arsenal,
    // and requiring a capital silently dropped three Bundesliga clubs with
    // their whole squads. The commentary lists this test exists to reject
    // are lowercase words.
    if !starts_upper_or_digit(&name) || !starts_upper_or_digit(&short_name) {
        return None;
    }

    let (eid, uid) = head.unzip();

    Some(Club {
        offset: len_at,
        club_id: read_u32(frame, len_at.checked_sub(CLUB_ID_BACK)?)?,
        nation_id: read_u32(frame, len_at.checked_sub(NATION_ID_BACK)?)?,
        eid,
        uid,
        name,
        short_name,
        // Filled after the roster table is scanned, keyed by club uid.
        reputation: None,
    })
}

/// Reads the `[eid][uid][uid]` entity head, validating the fixed shape around
/// it: the uid repeated, a zero byte, the nation id three times with
/// `FFFFFFFF` between the first two copies. A record without that exact shape
/// gets no entity id rather than a guessed one.
fn parse_head(frame: &[u8], len_at: usize) -> Option<(u32, u32)> {
    // Cheapest and most selective test first — this runs at every offset of
    // the frame, so the rest of the reads must not happen unless it passes.
    let ff = read_u32(frame, len_at.checked_sub(HEAD_FF_BACK)?)?;
    if ff != 0xFFFF_FFFF {
        return None;
    }
    let nation1 = read_u32(frame, len_at.checked_sub(NATION_ID_BACK)?)?;
    let location = read_u32(frame, len_at.checked_sub(NATION_LOCATION_BACK)?)?;
    let nation3 = read_u32(frame, len_at.checked_sub(NATION3_BACK)?)?;
    let zero = *frame.get(len_at.checked_sub(HEAD_ZERO_BACK)?)?;
    // Two of the three nation fields must agree — which two varies with how
    // the club crosses a border. The New Saints read pyramid == own nation
    // (both Wales) with an English location, and so does every UK
    // cross-border club; Vancouver Whitecaps read location == own nation
    // (both Canada) under a United States pyramid, and requiring the first
    // pair dropped all three Canadian MLS clubs with their squads. Any two
    // agreeing is still a strong shape over three independent u32s, and the
    // doubled uid, the zero byte and the FFFFFFFF carry the rest.
    if zero != 0 {
        return None;
    }
    let pairs_agree =
        nation1 == nation3 || nation1 == location || nation3 == location;
    if !pairs_agree {
        return None;
    }
    for field in [nation1, nation3, location] {
        if field == 0 || field > MAX_NATION_ID {
            return None;
        }
    }
    let uid = read_u32(frame, len_at.checked_sub(UID_BACK)?)?;
    let uid2 = read_u32(frame, len_at.checked_sub(UID2_BACK)?)?;
    let eid = read_u32(frame, len_at.checked_sub(EID_BACK)?)?;
    // Entity id zero is the null id, not a club: accepting it once gave
    // KF Tirana eid 0, which made every stray eid-0 row "a known club" and
    // cost Corinthians its employer ordinal to a duplicate drop.
    (uid == uid2 && uid != 0 && uid != u32::MAX && eid != 0 && eid != u32::MAX)
        .then_some((eid, uid))
}

fn starts_upper_or_digit(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
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
        record_tailed(369, 678, nation, club_id, name, short, HEADLESS_TAIL)
    }

    fn record_with_head(eid: u32, uid: u32, nation: u32, club_id: u32, name: &str, short: &str) -> Vec<u8> {
        record_tailed(eid, uid, nation, club_id, name, short, HEADLESS_TAIL)
    }

    /// A club playing in one nation's pyramid from a ground in another, which
    /// is what the location field at -18 records.
    fn record_cross_border(
        eid: u32,
        uid: u32,
        nation: u32,
        location: u32,
        club_id: u32,
        name: &str,
        short: &str,
    ) -> Vec<u8> {
        record_full(eid, uid, nation, location, club_id, name, short, HEADLESS_TAIL)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_tailed(
        eid: u32,
        uid: u32,
        nation: u32,
        club_id: u32,
        name: &str,
        short: &str,
        tail: [u8; 3],
    ) -> Vec<u8> {
        record_full(eid, uid, nation, nation, club_id, name, short, tail)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_full(
        eid: u32,
        uid: u32,
        nation: u32,
        location: u32,
        club_id: u32,
        name: &str,
        short: &str,
        tail: [u8; 3],
    ) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.push(0);
        v.extend_from_slice(&nation.to_le_bytes());
        v.extend_from_slice(&[0xFF; 4]);
        v.extend_from_slice(&location.to_le_bytes());
        v.extend_from_slice(&nation.to_le_bytes());
        v.extend_from_slice(&club_id.to_le_bytes());
        v.extend_from_slice(&[0x00, 0x0A, 0x00]);
        v.extend_from_slice(&tail);
        v.extend_from_slice(&(name.len() as u32).to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(&(short.len() as u32).to_le_bytes());
        v.extend_from_slice(short.as_bytes());
        v
    }

    #[test]
    fn a_cross_border_club_keeps_its_entity_head() {
        // The New Saints as a real save holds them: playing in the Welsh
        // pyramid (175) from a ground in England (139). Requiring the location
        // field to match the nation dropped this head, and with it the squad
        // link for the entire first team.
        let buf = record_cross_border(17656, 2_000_277_593, 175, 139, 1988, "The New Saints", "TNS");
        let found = scan_clubs(&buf);
        assert_eq!(found.len(), 1);
        let c = found.first().unwrap();
        assert_eq!(c.eid, Some(17656));
        assert_eq!(c.uid, Some(2_000_277_593));
        // The club's own nation is still the one it plays in, not where it sits.
        assert_eq!(c.nation_id, 175);
    }

    #[test]
    fn a_location_field_that_is_not_a_nation_id_still_fails_the_head() {
        // The relaxed check must not turn into no check: a pointer-shaped value
        // where the location belongs is not a cross-border club.
        let buf = record_cross_border(17656, 2_000_277_593, 175, 0xDEAD_BEEF, 1988, "Nowhere Town", "Nowhere");
        let found = scan_clubs(&buf);
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().eid, None);
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
    fn a_headless_record_without_the_trusted_tail_is_ignored() {
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        // Break both the head and the `10 FF FF` tail: nothing vouches for the
        // record, so two length-prefixed strings must not be enough.
        *buf.get_mut(4).unwrap() ^= 0xFF; // repeated uid
        let tail_at = buf.len() - 8 - 15 - 4 - 2;
        *buf.get_mut(tail_at).unwrap() = 0x11;
        assert!(scan_clubs(&buf).is_empty());
    }

    #[test]
    fn a_validated_head_carries_any_tail_bytes() {
        // Tottenham's real record reads 0x12 where most clubs read 0x10, and
        // Heybridge Swifts — the user's own club in a 26.2.0 save — reads
        // `10 FF 00` where the pair was assumed constant. Both heads validate,
        // and dropping the club dropped its whole squad with it.
        for tail in [[0x12, 0xFF, 0xFF], [0x10, 0xFF, 0x00], [0x10, 0x00, 0x00], [0x11, 0x00, 0x00]] {
            let buf = record_tailed(418, 727, 139, 1040, "Tottenham Hotspur", "Spurs", tail);
            let found = scan_clubs(&buf);
            assert_eq!(found.len(), 1, "tail {tail:02x?}");
            assert_eq!(found.first().unwrap().eid, Some(418));
            assert_eq!(found.first().unwrap().uid, Some(727));
        }
    }

    #[test]
    fn a_headless_record_needs_the_trusted_tail() {
        // Without a validating head the two-string tail also matches word
        // lists, so an unknown flags byte is not enough to admit the record.
        let mut buf = record_tailed(418, 727, 139, 1040, "Tottenham Hotspur", "Spurs", [0x12, 0xFF, 0xFF]);
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

    /// The boardroom run as it sits after the short name: `01 [u16] [dof]
    /// [flag] [count] [board eids] 01`.
    fn boardroom_bytes(dof: u32, board: &[u32]) -> Vec<u8> {
        let mut v = vec![0x01, 0x22, 0x00];
        v.extend_from_slice(&dof.to_le_bytes());
        v.push(0x02);
        v.push(u8::try_from(board.len()).unwrap());
        for e in board {
            v.extend_from_slice(&e.to_le_bytes());
        }
        v.push(0x01);
        v
    }

    #[test]
    fn reads_a_boardroom_behind_a_club() {
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        buf.extend(boardroom_bytes(2178, &[8242, 11039]));
        let clubs = scan_clubs(&buf);
        let rooms = scan_boardrooms(&buf, &clubs);
        assert_eq!(rooms.len(), 1);
        let room = rooms.first().unwrap();
        assert_eq!(room.club_eid, 369);
        assert_eq!(room.dof_eid, Some(2178));
        assert_eq!(room.board_eids, vec![8242, 11039]);
    }

    #[test]
    fn a_vacant_seat_reads_none_not_zero() {
        for vacant in [0u32, u32::MAX] {
            let mut buf = record(139, 1075, "Manchester City", "Man City");
            buf.extend(boardroom_bytes(vacant, &[8242]));
            let rooms = scan_boardrooms(&buf, &scan_clubs(&buf));
            assert_eq!(rooms.first().unwrap().dof_eid, None, "seat {vacant:#x}");
        }
    }

    #[test]
    fn a_variant_shape_yields_no_boardroom() {
        // A seat id past person-eid range is not the exact shape; nothing
        // must be guessed from it.
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        buf.extend(boardroom_bytes(50_000_000, &[8242]));
        assert!(scan_boardrooms(&buf, &scan_clubs(&buf)).is_empty());

        // A broken closing sentinel is refused too.
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        let mut run = boardroom_bytes(2178, &[8242]);
        *run.last_mut().unwrap() = 0x00;
        buf.extend(run);
        assert!(scan_boardrooms(&buf, &scan_clubs(&buf)).is_empty());
    }

    #[test]
    fn a_club_without_an_eid_gets_no_boardroom() {
        // Headless-tail record parses as a club but carries no entity id;
        // a boardroom keyed to nothing is unusable and must not surface.
        let mut buf = record(139, 1075, "Manchester City", "Man City");
        *buf.get_mut(4).unwrap() ^= 0xFF; // break the repeated uid
        buf.extend(boardroom_bytes(2178, &[8242]));
        let clubs = scan_clubs(&buf);
        assert_eq!(clubs.first().unwrap().eid, None);
        assert!(scan_boardrooms(&buf, &clubs).is_empty());
    }
}
