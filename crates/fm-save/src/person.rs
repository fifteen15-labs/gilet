use crate::date::Date;

/// A person record: a player or member of staff.
///
/// Only fields confirmed against real data are modelled — see
/// `docs/SAVE_FORMAT.md`. Club membership is absent because it is not located:
/// three separate approaches failed to find it, and a guess would be worse
/// than the gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// Byte offset of the record within its frame, for tracing back to disk.
    /// This points at `surname_id`; the record itself begins 5 bytes earlier,
    /// at `first_name_id`.
    pub offset: usize,
    /// String-table ID of the forename. Confirmed by lookup: Haaland's value,
    /// 217140, resolves to "Erling" in the string table.
    pub first_name_id: Option<u32>,
    /// String-table ID of the surname.
    pub surname_id: u32,
    /// String-table ID of the nickname, `None` when the player has none.
    /// Stored as `0xFFFF_FFFF` on disk.
    pub common_name_id: Option<u32>,
    /// Full name as stored inline, e.g. `"Erling Braut Haaland"`.
    pub full_name: String,
    pub date_of_birth: Date,
    /// Nation identifier, e.g. 139 for England. Shares the numbering the club
    /// records use, so a club's nation and a player's match.
    pub nation_id: u16,
    /// Ability and attributes, when this person has an attribute block.
    /// `None` means staff: only players carry one.
    pub ability: Option<crate::ability::Ability>,
}

impl Person {
    /// Whether this person is a player. Staff have no attribute block.
    #[must_use]
    pub fn is_player(&self) -> bool {
        self.ability.is_some()
    }

    /// The nation's name, for the identifiers confirmed so far.
    #[must_use]
    pub fn nation(&self) -> Option<&'static str> {
        nation_name(self.nation_id)
    }
}

/// Nation names for identifiers confirmed against known players.
///
/// The file does not store the name beside the identifier, so these were
/// verified two ways. Directly, from players whose nationality is not in doubt,
/// cross-checked against the nation the club records carry — German clubs and
/// Florian Wirtz both report 145, English clubs and Saka both report 139. And
/// by grouping every person by nation and reading the surnames.
/// A national squad's names are unmistakable — Zidane, Henry and Deschamps
/// fix 143; Davids, Reiziger and van Nistelrooij fix 158; Okocha, Kanu and
/// Amokachi fix 33; Donovan, Berhalter and Cherundolo fix 120.
///
/// Left unnamed where the surnames are Spanish-speaking but the specific
/// country is not clear from names alone — several identifiers share that
/// problem and a wrong flag is worse than a number.
#[must_use]
pub fn nation_name(id: u16) -> Option<&'static str> {
    match id {
        33 => Some("Nigeria"),
        108 => Some("Jamaica"),
        120 => Some("United States"),
        138 => Some("Denmark"),
        139 => Some("England"),
        143 => Some("France"),
        145 => Some("Germany"),
        158 => Some("Netherlands"),
        159 => Some("Northern Ireland"),
        160 => Some("Norway"),
        162 => Some("Portugal"),
        163 => Some("Republic of Ireland"),
        167 => Some("Scotland"),
        170 => Some("Spain"),
        171 => Some("Sweden"),
        175 => Some("Wales"),
        177 => Some("Australia"),
        187 => Some("Argentina"),
        189 => Some("Brazil"),
        190 => Some("Chile"),
        _ => None,
    }
}

const NO_COMMON_NAME: u32 = 0xFFFF_FFFF;
const MIN_NAME_LEN: usize = 2;
const MAX_NAME_LEN: usize = 64;

/// Bytes past the name where the nation identifier sits.
const NATION_OFFSET: usize = 13;

/// A person in a football save is born within this window. Wider than strictly
/// necessary — the oldest staff and the youngest newgens both have to fit — but
/// tight enough to reject coincidental bytes that decode to a valid date.
const MIN_BIRTH_YEAR: u16 = 1920;
const MAX_BIRTH_YEAR: u16 = 2030;

/// Bytes between `surname_id` and `full_name_length`: one unknown byte, the
/// common-name ID, one more unknown byte.
const MIDDLE_LEN: usize = 6;
/// `surname_id` + middle + `full_name_length`.
const PREFIX_LEN: usize = 4 + MIDDLE_LEN + 4;

/// Scans a decompressed frame for person records.
///
/// There is no record index, so this is a sliding scan whose acceptance test is
/// that the bytes *after* a plausible name decode to a plausible date of birth.
/// That check is what makes the scan trustworthy: random bytes almost never
/// yield both valid UTF-8 of a declared length and a day-of-year in 1..=366
/// paired with a sensible year.
///
/// Note the `common_name_id` field is read, not matched. Requiring the
/// `FF FF FF FF` "no nickname" value silently drops every player who has one —
/// Lamine Yamal and Vinícius Júnior both vanish that way.
#[must_use]
pub fn scan_people(frame: &[u8]) -> Vec<Person> {
    let mut out = Vec::new();
    let mut at = 0usize;

    while at + PREFIX_LEN + MIN_NAME_LEN <= frame.len() {
        match parse_at(frame, at) {
            Some(person) => {
                at = person.offset + PREFIX_LEN + person.full_name.len();
                out.push(person);
            }
            None => at += 1,
        }
    }

    out
}

fn parse_at(frame: &[u8], at: usize) -> Option<Person> {
    let surname_id = read_u32(frame, at)?;
    // The forename identifier sits 5 bytes before the surname one. Absent at
    // the very start of a frame, which is not a reason to reject the record.
    let first_name_id = at.checked_sub(5).and_then(|p| read_u32(frame, p));
    let common_raw = read_u32(frame, at + 5)?;
    let name_len = read_u32(frame, at + 4 + MIDDLE_LEN)? as usize;

    if !(MIN_NAME_LEN..=MAX_NAME_LEN).contains(&name_len) {
        return None;
    }

    let name_start = at + PREFIX_LEN;
    let raw = frame.get(name_start..name_start.checked_add(name_len)?)?;
    let full_name = std::str::from_utf8(raw).ok()?;
    if !is_person_name(full_name) {
        return None;
    }

    // The acceptance test: a real record is followed by a real date of birth.
    let after = name_start + name_len;
    let year = read_u16(frame, after + 2)?;
    if !(MIN_BIRTH_YEAR..=MAX_BIRTH_YEAR).contains(&year) {
        return None;
    }
    let date_of_birth = Date::from_day_of_year(read_u16(frame, after)?, year)?;

    // Nationality sits further out, so a record truncated at the end of the
    // frame still parses — it is descriptive, not part of the acceptance test.
    let nation_id = read_u16(frame, after + NATION_OFFSET).unwrap_or_default();

    Some(Person {
        offset: at,
        first_name_id,
        surname_id,
        common_name_id: (common_raw != NO_COMMON_NAME).then_some(common_raw),
        full_name: full_name.to_owned(),
        date_of_birth,
        nation_id,
        // Filled in by `Save::parse`, which matches blocks to people once both
        // scans have run.
        ability: None,
    })
}

/// Rejects control characters, padded strings and anything too sparse to be a
/// name. Names legitimately contain spaces, hyphens, apostrophes and non-ASCII
/// letters (`Mbappé`, `Nergård`, `Muñoz`), so the test stays permissive about
/// those while excluding the fragments a raw byte scan turns up — a stray
/// `"sps"`, or `"  d"` with leading padding.
fn is_person_name(s: &str) -> bool {
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return false;
    }
    let mut letters = 0usize;
    for c in s.chars() {
        if c.is_control() {
            return false;
        }
        if c.is_alphabetic() {
            letters += 1;
        }
    }
    // Real full names are several letters and, in this field, effectively
    // always more than one word.
    letters >= 4 && s.contains(char::is_whitespace)
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

    /// Builds a record in the on-disk shape: surname id, unknown byte,
    /// common-name id, unknown byte, length-prefixed name, then day-of-year and
    /// year.
    fn record(surname_id: u32, common: u32, name: &str, doy: u16, year: u16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&surname_id.to_le_bytes());
        v.push(0);
        v.extend_from_slice(&common.to_le_bytes());
        v.push(0);
        v.extend_from_slice(&(name.len() as u32).to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(&doy.to_le_bytes());
        v.extend_from_slice(&year.to_le_bytes());
        v
    }

    #[test]
    fn reads_the_nation_identifier() {
        // 139 is England, confirmed from Saka, Kane, Walker, Grealish and
        // Bellingham, and matching the nation the English clubs carry.
        // Pad out to the nation field, which sits 13 bytes past the name.
        let mut buf = record(1, NO_COMMON_NAME, "Bukayo Ayoyinka Saka", 248, 2001);
        buf.extend_from_slice(&[0u8; 9]);
        buf.extend_from_slice(&139u16.to_le_bytes());
        buf.extend_from_slice(&[0u8; 8]);
        let found = scan_people(&buf);
        assert_eq!(found.first().unwrap().nation_id, 139);
        assert_eq!(found.first().unwrap().nation(), Some("England"));
    }

    #[test]
    fn an_unconfirmed_nation_has_no_name() {
        assert_eq!(nation_name(1), None);
    }

    #[test]
    fn reads_a_record_with_no_nickname() {
        // Haaland's real values: surname id 0x6A311, born day 203 of 2000.
        let buf = record(0x0006_A311, NO_COMMON_NAME, "Erling Braut Haaland", 203, 2000);
        let found = scan_people(&buf);
        assert_eq!(found.len(), 1);
        let p = found.first().unwrap();
        assert_eq!(p.full_name, "Erling Braut Haaland");
        assert_eq!(p.surname_id, 0x0006_A311);
        assert_eq!(p.common_name_id, None);
        assert_eq!((p.date_of_birth.day, p.date_of_birth.month), (21, 7));
    }

    #[test]
    fn keeps_players_who_have_a_nickname() {
        // The bug this guards: matching on FF FF FF FF drops these entirely.
        let buf = record(1234, 999, "Vinícius José de Oliveira Júnior", 194, 2000);
        let found = scan_people(&buf);
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().common_name_id, Some(999));
    }

    #[test]
    fn decodes_multibyte_names_by_byte_length() {
        // "Mbappé" is 7 bytes but 6 chars; the length prefix counts bytes.
        let buf = record(1, NO_COMMON_NAME, "Kylian Mbappé Lottin", 354, 1998);
        let found = scan_people(&buf);
        assert_eq!(found.first().unwrap().full_name, "Kylian Mbappé Lottin");
    }

    #[test]
    fn rejects_an_implausible_date_of_birth() {
        // Day 400 does not exist, so this is not a record.
        let buf = record(1, NO_COMMON_NAME, "Not A Person", 400, 2000);
        assert!(scan_people(&buf).is_empty());
    }

    #[test]
    fn rejects_control_characters_in_a_name() {
        let buf = record(1, NO_COMMON_NAME, "bad\u{7}name", 100, 2000);
        assert!(scan_people(&buf).is_empty());
    }

    #[test]
    fn finds_several_records_in_sequence() {
        let mut buf = record(1, NO_COMMON_NAME, "Bukayo Ayoyinka Saka", 248, 2001);
        buf.extend(record(2, NO_COMMON_NAME, "Florian Richard Wirtz", 123, 2003));
        let found = scan_people(&buf);
        assert_eq!(found.len(), 2);
        assert_eq!(found.get(1).unwrap().date_of_birth.month, 5);
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let full = record(1, NO_COMMON_NAME, "Erling Braut Haaland", 203, 2000);
        for cut in 0..full.len() {
            let _ = scan_people(full.get(..cut).unwrap());
        }
    }
}
