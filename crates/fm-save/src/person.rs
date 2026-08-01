use crate::date::Date;
use crate::strings::StringTable;

/// A person record: a player or member of staff.
///
/// Only fields confirmed against real data are modelled — see
/// `docs/SAVE_FORMAT.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    /// Byte offset of the record prefix (the forename id) within its frame.
    pub offset: usize,
    /// String-table ID of the forename, resolved against the forename pool.
    pub first_name_id: u32,
    /// String-table ID of the surname, resolved against the surname pool.
    pub surname_id: u32,
    /// String-table ID of the nickname, `None` when the player has none.
    /// Stored as `0xFFFF_FFFF` on disk.
    pub common_name_id: Option<u32>,
    /// Full name. Stored inline only when it differs from
    /// `"forename surname"`; otherwise composed from the string table, which
    /// is how "Virgil van Dijk" and every other plainly-named person is held.
    pub full_name: String,
    pub date_of_birth: Date,
    /// Nation identifier, e.g. 139 for England. Shares the numbering the club
    /// records use, so a club's nation and a player's match.
    pub nation_id: u16,
    /// The person's database entity id — what squad lists reference.
    /// `None` when no identity block was found in the record.
    pub eid: Option<u32>,
    /// The person's second identifier, repeated beside the entity id on disk.
    pub uid: Option<u32>,
    /// Entity id of the club whose first-team squad lists this person.
    /// Filled from the squad table by `Save::parse`, `None` for the unattached.
    pub club_eid: Option<u32>,
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

/// Bytes past the date of birth where the nation identifier sits.
const NATION_OFFSET: usize = 13;

/// A person in a football save is born within this window. Wider than strictly
/// necessary — the oldest staff and the youngest newgens both have to fit — but
/// tight enough to reject coincidental bytes that decode to a valid date.
const MIN_BIRTH_YEAR: u16 = 1920;
const MAX_BIRTH_YEAR: u16 = 2030;

/// `[first u32] 00 [surname u32] 00 [common u32] 00` — the record prefix.
const PREFIX_LEN: usize = 15;

/// Entity ids stay comfortably below this; a bigger value is noise.
const MAX_EID: u32 = 3_000_000;

/// Scans a frame for person records, starting where the string table ends.
///
/// The record prefix is three string-table references, each followed by a zero
/// byte, then a length-prefixed inline full name — with **zero length when the
/// full name is just "forename surname"** — then the date of birth:
///
/// `[first u32] 00 [surname u32] 00 [common u32] 00 [len u32] [name] [dob]`
///
/// The acceptance test is threefold: the forename id must resolve in the
/// forename pool, the surname id in the surname pool, and the bytes after the
/// name must decode to a plausible date of birth. A length prefix can never be
/// mistaken for a date (its high half is zero; a date's year half is 1920+),
/// so the zero-length case is unambiguous.
#[must_use]
pub fn scan_people(frame: &[u8], strings: &StringTable) -> Vec<Person> {
    let mut out = Vec::new();
    let mut at = strings.end_offset;

    while at + PREFIX_LEN + 8 <= frame.len() {
        match parse_at(frame, at, strings) {
            Some((person, next)) => {
                out.push(person);
                at = next;
            }
            None => at += 1,
        }
    }

    out
}

fn parse_at(frame: &[u8], at: usize, strings: &StringTable) -> Option<(Person, usize)> {
    if *frame.get(at + 4)? != 0 || *frame.get(at + 9)? != 0 || *frame.get(at + 14)? != 0 {
        return None;
    }
    let first_name_id = read_u32(frame, at)?;
    let surname_id = read_u32(frame, at + 5)?;
    let common_raw = read_u32(frame, at + 10)?;

    let forename = strings.forenames.get(&first_name_id)?;
    let surname = strings.surnames.get(&surname_id)?;

    let mut body = at + PREFIX_LEN;
    let name_len = read_u32(frame, body)? as usize;
    let full_name: String;
    if name_len == 0 {
        full_name = format!("{forename} {surname}");
        body += 4;
    } else if (MIN_NAME_LEN..=MAX_NAME_LEN).contains(&name_len) && read_u16(frame, body + 2)? == 0 {
        let raw = frame.get(body + 4..(body + 4).checked_add(name_len)?)?;
        let text = std::str::from_utf8(raw).ok()?;
        if text.chars().any(char::is_control) {
            return None;
        }
        full_name = text.to_owned();
        body += 4 + name_len;
    } else {
        return None;
    }

    // The final acceptance test: a real record carries a real date of birth.
    let year = read_u16(frame, body + 2)?;
    if !(MIN_BIRTH_YEAR..=MAX_BIRTH_YEAR).contains(&year) {
        return None;
    }
    let date_of_birth = Date::from_day_of_year(read_u16(frame, body)?, year)?;

    // Nationality sits further out, so a record truncated at the end of the
    // frame still parses — it is descriptive, not part of the acceptance test.
    let nation_id = read_u16(frame, body + NATION_OFFSET).unwrap_or_default();

    Some((
        Person {
            offset: at,
            first_name_id,
            surname_id,
            common_name_id: (common_raw != NO_COMMON_NAME).then_some(common_raw),
            full_name,
            date_of_birth,
            nation_id,
            eid: None,
            uid: None,
            club_eid: None,
            // Filled in by `Save::parse`, which matches blocks to people once
            // both scans have run.
            ability: None,
        },
        body + 4,
    ))
}

/// An identity block: `[eid u32][uid u32][uid u32]`, the uid repeated.
///
/// Every person record contains one, several hundred bytes past the name. The
/// eid is what squad lists reference; eids ascend strictly through the person
/// region, which is what separates real blocks from coincidental byte
/// patterns — see [`bind_identities`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub offset: usize,
    pub eid: u32,
    pub uid: u32,
}

/// Finds identity blocks and attaches them to people.
///
/// Candidates are every `[eid][uid][uid]` triple preceded by three zero bytes.
/// That shape recurs by chance in contract data, so the true blocks are picked
/// as the longest strictly-eid-ascending chain — person records are written in
/// entity-id order, which noise does not follow. Each person takes the first
/// chain block inside their record; the full chain is returned so that squad
/// references can also be resolved through blocks that share a record.
pub fn bind_identities(frame: &[u8], people: &mut [Person], start: usize) -> Vec<Identity> {
    let candidates = scan_triples(frame, start);
    let chain = longest_ascending(&candidates);

    let offsets: Vec<usize> = people.iter().map(|p| p.offset).collect();
    for id in &chain {
        let idx = offsets.partition_point(|&o| o <= id.offset);
        let Some(person) = idx.checked_sub(1).and_then(|i| people.get_mut(i)) else {
            continue;
        };
        if person.eid.is_none() {
            person.eid = Some(id.eid);
            person.uid = Some(id.uid);
        }
    }
    chain
}

fn scan_triples(frame: &[u8], start: usize) -> Vec<Identity> {
    let mut out = Vec::new();
    let mut at = start.max(3);
    while at + 12 <= frame.len() {
        let zeros = frame.get(at - 3..at).is_some_and(|b| b == [0, 0, 0]);
        if !zeros {
            at += 1;
            continue;
        }
        let (Some(eid), Some(a), Some(b)) = (
            read_u32(frame, at),
            read_u32(frame, at + 4),
            read_u32(frame, at + 8),
        ) else {
            break;
        };
        if a == b && a != 0 && a != u32::MAX && eid > 0 && eid < MAX_EID {
            out.push(Identity {
                offset: at,
                eid,
                uid: a,
            });
            at += 12;
        } else {
            at += 1;
        }
    }
    out
}

/// Longest strictly-increasing-by-eid subsequence, in file order.
fn longest_ascending(candidates: &[Identity]) -> Vec<Identity> {
    // Patience LIS with parent pointers.
    let mut tails_eid: Vec<u32> = Vec::new();
    let mut tails_idx: Vec<usize> = Vec::new();
    let mut prev: Vec<Option<usize>> = vec![None; candidates.len()];

    for (i, c) in candidates.iter().enumerate() {
        let k = tails_eid.partition_point(|&e| e < c.eid);
        if let Some(slot) = prev.get_mut(i) {
            *slot = k.checked_sub(1).and_then(|j| tails_idx.get(j).copied());
        }
        if k == tails_eid.len() {
            tails_eid.push(c.eid);
            tails_idx.push(i);
        } else if tails_eid.get(k).is_some_and(|&e| c.eid < e) {
            if let (Some(te), Some(ti)) = (tails_eid.get_mut(k), tails_idx.get_mut(k)) {
                *te = c.eid;
                *ti = i;
            }
        }
    }

    let mut chain = Vec::new();
    let mut cur = tails_idx.last().copied();
    while let Some(i) = cur {
        if let Some(c) = candidates.get(i) {
            chain.push(*c);
        }
        cur = prev.get(i).copied().flatten();
    }
    chain.reverse();
    chain
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
    use std::collections::HashMap;

    fn table() -> StringTable {
        let mut forenames = HashMap::new();
        forenames.insert(100, "Erling".to_owned());
        forenames.insert(101, "Virgil".to_owned());
        let mut surnames = HashMap::new();
        surnames.insert(200, "Haaland".to_owned());
        surnames.insert(201, "van Dijk".to_owned());
        StringTable {
            forenames,
            surnames,
            common_names: HashMap::new(),
            end_offset: 0,
        }
    }

    /// Builds a record in the on-disk shape: forename id, zero, surname id,
    /// zero, common-name id, zero, length-prefixed name (empty when the full
    /// name is composed), then day-of-year and year.
    fn record(first: u32, surname: u32, common: u32, name: Option<&str>, doy: u16, year: u16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&first.to_le_bytes());
        v.push(0);
        v.extend_from_slice(&surname.to_le_bytes());
        v.push(0);
        v.extend_from_slice(&common.to_le_bytes());
        v.push(0);
        match name {
            Some(n) => {
                v.extend_from_slice(&(n.len() as u32).to_le_bytes());
                v.extend_from_slice(n.as_bytes());
            }
            None => v.extend_from_slice(&0u32.to_le_bytes()),
        }
        v.extend_from_slice(&doy.to_le_bytes());
        v.extend_from_slice(&year.to_le_bytes());
        v
    }

    #[test]
    fn reads_a_record_with_an_inline_name() {
        // Haaland's real values: born day 203 of 2000, name stored inline
        // because "Erling Braut Haaland" is more than forename + surname.
        let buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        let found = scan_people(&buf, &table());
        assert_eq!(found.len(), 1);
        let p = found.first().unwrap();
        assert_eq!(p.full_name, "Erling Braut Haaland");
        assert_eq!(p.first_name_id, 100);
        assert_eq!(p.surname_id, 200);
        assert_eq!(p.common_name_id, None);
        assert_eq!((p.date_of_birth.day, p.date_of_birth.month), (21, 7));
    }

    #[test]
    fn composes_the_name_when_the_inline_length_is_zero() {
        // Van Dijk's real shape: full name is exactly forename + surname, so
        // the save stores no inline copy at all.
        let buf = record(101, 201, NO_COMMON_NAME, None, 189, 1991);
        let found = scan_people(&buf, &table());
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().full_name, "Virgil van Dijk");
    }

    #[test]
    fn rejects_ids_that_are_not_in_the_string_pools() {
        let buf = record(999, 200, NO_COMMON_NAME, None, 100, 2000);
        assert!(scan_people(&buf, &table()).is_empty());
    }

    #[test]
    fn keeps_people_who_have_a_nickname() {
        let buf = record(100, 200, 555, Some("Vinícius José de Oliveira Júnior"), 194, 2000);
        let found = scan_people(&buf, &table());
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().common_name_id, Some(555));
    }

    #[test]
    fn reads_the_nation_identifier() {
        // 139 is England; the field sits 13 bytes past the date of birth.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Bukayo Ayoyinka Saka"), 248, 2001);
        buf.extend_from_slice(&[0u8; 9]);
        buf.extend_from_slice(&139u16.to_le_bytes());
        let found = scan_people(&buf, &table());
        assert_eq!(found.first().unwrap().nation_id, 139);
        assert_eq!(found.first().unwrap().nation(), Some("England"));
    }

    #[test]
    fn rejects_an_implausible_date_of_birth() {
        let buf = record(100, 200, NO_COMMON_NAME, Some("Not A Person"), 400, 2000);
        assert!(scan_people(&buf, &table()).is_empty());
    }

    #[test]
    fn finds_several_records_in_sequence() {
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        let found = scan_people(&buf, &table());
        assert_eq!(found.len(), 2);
        assert_eq!(found.get(1).unwrap().full_name, "Virgil van Dijk");
    }

    fn identity_block(eid: u32, uid: u32) -> Vec<u8> {
        let mut v = vec![0, 0, 0];
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v
    }

    #[test]
    fn binds_identities_to_the_records_that_contain_them() {
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(50, 9_000_001));
        let second_at = buf.len();
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block(51, 9_000_002));

        let mut people = scan_people(&buf, &table());
        let chain = bind_identities(&buf, &mut people, 0);

        assert_eq!(chain.len(), 2);
        assert_eq!(people.first().unwrap().eid, Some(50));
        assert_eq!(people.get(1).unwrap().eid, Some(51));
        assert_eq!(people.get(1).unwrap().uid, Some(9_000_002));
        assert_eq!(people.get(1).unwrap().offset, second_at);
    }

    #[test]
    fn noise_that_breaks_the_ascending_order_is_dropped() {
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(50, 9_000_001));
        // A coincidental triple with a wildly out-of-order eid inside the
        // same record must not displace the real ones around it.
        buf.extend(identity_block(2_000_000, 123_456));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block(51, 9_000_002));

        let mut people = scan_people(&buf, &table());
        let chain = bind_identities(&buf, &mut people, 0);

        assert_eq!(chain.len(), 2, "the noise block should lose the chain race");
        assert_eq!(people.first().unwrap().eid, Some(50));
        assert_eq!(people.get(1).unwrap().eid, Some(51));
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let full = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        for cut in 0..full.len() {
            let _ = scan_people(full.get(..cut).unwrap(), &table());
        }
    }
}
