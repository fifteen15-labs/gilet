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
    /// Whether this person is a woman, inferred from the forename pool.
    /// `None` when the save gives no basis for the split — see
    /// [`female_forename_boundary`].
    pub female: Option<bool>,
    /// The eight hidden personality attributes, 1-20, in storage order:
    /// Adaptability, Ambition, Loyalty, Pressure, Professionalism,
    /// Sportsmanship, Temperament, Controversy. Slots 0, 4 and 7 were
    /// confirmed against in-game staff screens; the rest fell to the
    /// pre-game editor, whose sheet for Guardiola (20, 20, 15, 18, 20, 16,
    /// 14, 8) matches his run exactly with every ambiguous value distinct.
    /// `None` when the run does not parse.
    pub personality: Option<[u8; 8]>,
    /// Weekly wage in the save's display currency, from the contract block.
    /// `None` when no contract parses — the unemployed and the retired.
    pub wage: Option<u32>,
    /// Contract expiry date, when the contract block carries one.
    pub contract_until: Option<Date>,
    /// Ability and attributes, when this person has an attribute block.
    /// `None` means staff: only players carry one.
    pub ability: Option<crate::ability::Ability>,
    /// Non-player attributes — the editor's "All Attributes" sheet — read from
    /// the entity object one eid below this person's own. `None` when no such
    /// object carries a block, which is most players.
    pub staff: Option<crate::staff::Staff>,
}

impl Person {
    /// Whether this person is a player. Staff have no attribute block.
    #[must_use]
    pub fn is_player(&self) -> bool {
        self.ability.is_some()
    }

    /// Hidden Adaptability, 1-20. Verified against staff report screens,
    /// where the attribute is visible: Elite reads 20, Outstanding 19,
    /// Good 13 — and against the pre-game editor (Guardiola 20).
    #[must_use]
    pub fn adaptability(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[0])
    }

    /// Hidden Ambition, 1-20. Pinned by the pre-game editor: Guardiola's
    /// editor sheet reads Ambition 20, and his slot 1 is 20 while slot 2
    /// carries his distinct Loyalty 15. (This slot was earlier misread as
    /// Loyalty from a screen where both were high.)
    #[must_use]
    pub fn ambition(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[1])
    }

    /// Hidden Loyalty, 1-20. Pinned by the editor: Guardiola Loyalty 15,
    /// unique in his run at slot 2.
    #[must_use]
    pub fn loyalty(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[2])
    }

    /// Hidden Pressure, 1-20. Editor: Guardiola 18, unique at slot 3.
    #[must_use]
    pub fn pressure(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[3])
    }

    /// Hidden Professionalism, 1-20 — the attribute that drives development.
    /// A "Model Professional" reads 20 here, a "Model Citizen" 16.
    #[must_use]
    pub fn professionalism(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[4])
    }

    /// Hidden Sportsmanship, 1-20. Editor: Guardiola 16, unique at slot 5.
    #[must_use]
    pub fn sportsmanship(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[5])
    }

    /// Hidden Temperament, 1-20. Editor: Guardiola 14, unique at slot 6.
    #[must_use]
    pub fn temperament(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[6])
    }

    /// Hidden Controversy, 1-20; almost everyone is low.
    #[must_use]
    pub fn controversy(&self) -> Option<u8> {
        self.personality.as_ref().map(|p| p[7])
    }

    /// The nation's name, for the identifiers confirmed so far.
    #[must_use]
    pub fn nation(&self) -> Option<&'static str> {
        nation_name(self.nation_id)
    }
}

/// Nation names for identifiers confirmed against known players.
///
/// The file does not store the name beside the identifier — every occurrence
/// of "England" in the database frame is the *surname* England — so these were
/// derived by grouping every person by nation identifier and reading the best
/// players in each group. A national squad is unmistakable: Courtois, De
/// Bruyne, Lukaku and Tielemans fix 131; Modrić, Gvardiol and Kovačić fix 135;
/// Kvaratskhelia and Mamardashvili fix 144; Son, Kim Min-Jae and Lee Kang-In
/// fix 80. Cross-checked against the nation the club records carry, which uses
/// the same numbering — German clubs and Wirtz both report 145, English clubs
/// and Saka both report 139.
///
/// Identifiers are left unnamed rather than guessed wherever the best players
/// in the group do not settle the country — mostly small groups of fewer than
/// twenty people. A wrong flag is worse than a number.
///
/// Note 0 is a real identifier (Algeria: Mahrez, Bennacer, Bensebaïni), not a
/// missing value.
#[must_use]
#[allow(clippy::too_many_lines)] // one match arm per named nation, data not logic
pub fn nation_name(id: u16) -> Option<&'static str> {
    match id {
        0 => Some("Algeria"),
        1 => Some("Angola"),
        2 => Some("Benin"),
        4 => Some("Burkina Faso"),
        5 => Some("Burundi"),
        6 => Some("Cameroon"),
        7 => Some("Cape Verde"),
        8 => Some("Central African Republic"),
        11 => Some("Egypt"),
        12 => Some("Equatorial Guinea"),
        14 => Some("Gabon"),
        15 => Some("Gambia"),
        16 => Some("Ghana"),
        17 => Some("Guinea"),
        18 => Some("Guinea-Bissau"),
        19 => Some("Ivory Coast"),
        20 => Some("Kenya"),
        22 => Some("Liberia"),
        23 => Some("Libya"),
        25 => Some("Malawi"),
        26 => Some("Mali"),
        27 => Some("Mauritania"),
        29 => Some("Morocco"),
        30 => Some("Mozambique"),
        33 => Some("Nigeria"),
        35 => Some("São Tomé and Príncipe"),
        36 => Some("Senegal"),
        38 => Some("Sierra Leone"),
        39 => Some("Somalia"),
        40 => Some("South Africa"),
        41 => Some("Sudan"),
        43 => Some("Tanzania"),
        44 => Some("Congo"),
        45 => Some("Togo"),
        46 => Some("Tunisia"),
        47 => Some("Uganda"),
        48 => Some("DR Congo"),
        49 => Some("Zambia"),
        50 => Some("Zimbabwe"),
        55 => Some("China"),
        56 => Some("Hong Kong"),
        57 => Some("India"),
        58 => Some("Indonesia"),
        59 => Some("Iran"),
        60 => Some("Iraq"),
        61 => Some("Japan"),
        64 => Some("Kazakhstan"),
        66 => Some("Kyrgyzstan"),
        68 => Some("Lebanon"),
        76 => Some("Pakistan"),
        78 => Some("Saudi Arabia"),
        80 => Some("South Korea"),
        81 => Some("Sri Lanka"),
        82 => Some("Syria"),
        83 => Some("Chinese Taipei"),
        84 => Some("Tajikistan"),
        85 => Some("Thailand"),
        86 => Some("Philippines"),
        88 => Some("United Arab Emirates"),
        89 => Some("Uzbekistan"),
        90 => Some("Vietnam"),
        92 => Some("Antigua and Barbuda"),
        94 => Some("Barbados"),
        96 => Some("Bermuda"),
        97 => Some("Canada"),
        99 => Some("Costa Rica"),
        101 => Some("Dominica"),
        102 => Some("El Salvador"),
        103 => Some("Grenada"),
        104 => Some("Guatemala"),
        105 => Some("Guyana"),
        106 => Some("Haiti"),
        107 => Some("Honduras"),
        108 => Some("Jamaica"),
        109 => Some("Mexico"),
        110 => Some("Curaçao"),
        112 => Some("Panama"),
        114 => Some("Saint Lucia"),
        115 => Some("Saint Kitts and Nevis"),
        116 => Some("Cayman Islands"),
        117 => Some("Suriname"),
        119 => Some("Trinidad and Tobago"),
        120 => Some("United States"),
        126 => Some("Albania"),
        128 => Some("Armenia"),
        129 => Some("Austria"),
        130 => Some("Azerbaijan"),
        131 => Some("Belgium"),
        133 => Some("Bosnia and Herzegovina"),
        134 => Some("Bulgaria"),
        135 => Some("Croatia"),
        136 => Some("Cyprus"),
        137 => Some("Czech Republic"),
        138 => Some("Denmark"),
        139 => Some("England"),
        140 => Some("Estonia"),
        141 => Some("Faroe Islands"),
        142 => Some("Finland"),
        143 => Some("France"),
        144 => Some("Georgia"),
        145 => Some("Germany"),
        146 => Some("Greece"),
        147 => Some("Hungary"),
        148 => Some("Iceland"),
        149 => Some("Israel"),
        150 => Some("Italy"),
        151 => Some("Latvia"),
        153 => Some("Lithuania"),
        154 => Some("Luxembourg"),
        155 => Some("North Macedonia"),
        156 => Some("Malta"),
        157 => Some("Moldova"),
        158 => Some("Netherlands"),
        159 => Some("Northern Ireland"),
        160 => Some("Norway"),
        161 => Some("Poland"),
        162 => Some("Portugal"),
        163 => Some("Republic of Ireland"),
        164 => Some("Romania"),
        165 => Some("Russia"),
        167 => Some("Scotland"),
        168 => Some("Slovakia"),
        169 => Some("Slovenia"),
        170 => Some("Spain"),
        171 => Some("Sweden"),
        172 => Some("Switzerland"),
        173 => Some("Turkey"),
        174 => Some("Ukraine"),
        175 => Some("Wales"),
        176 => Some("Serbia"),
        177 => Some("Australia"),
        180 => Some("New Zealand"),
        187 => Some("Argentina"),
        189 => Some("Brazil"),
        190 => Some("Chile"),
        191 => Some("Colombia"),
        192 => Some("Ecuador"),
        193 => Some("Paraguay"),
        194 => Some("Peru"),
        195 => Some("Uruguay"),
        196 => Some("Venezuela"),
        207 => Some("Montserrat"),
        212 => Some("Dominican Republic"),
        216 => Some("Gibraltar"),
        219 => Some("Kosovo"),
        226 => Some("Guadeloupe"),
        227 => Some("Martinique"),
        234 => Some("Comoros"),
        236 => Some("Timor-Leste"),
        247 => Some("Montenegro"),
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
    let personality = find_personality(frame, body, nation_id);

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
            female: None,
            personality,
            wage: None,
            contract_until: None,
            // Filled in by `Save::parse`, which matches blocks to people once
            // both scans have run.
            ability: None,
            staff: None,
        },
        body + 4,
    ))
}

/// How far past the date of birth the personality marker can sit.
const PERSONALITY_WINDOW: usize = 160;

/// Finds the eight hidden personality attributes.
///
/// They follow a repeat of the nation identifier as a `u16` and six zero
/// bytes; the eight bytes after that are each 1-20. (A one-byte count —
/// citizenships, by the look of it — precedes the repeated nation, but it
/// varies and is not part of the match.) Repeating the record's own nation
/// is what makes the match safe: a chance window is never this record's
/// nation, six zeros and eight in-range bytes at once. Absent on some
/// records (human-manager avatars use a different layout), which yields
/// `None` rather than a guess.
fn find_personality(frame: &[u8], body: usize, nation_id: u16) -> Option<[u8; 8]> {
    for at in body..body + PERSONALITY_WINDOW {
        if read_u16(frame, at) != Some(nation_id) {
            continue;
        }
        if frame.get(at + 2..at + 8) != Some(&[0u8; 6][..]) {
            continue;
        }
        let run = frame.get(at + 8..at + 16)?;
        if run.iter().all(|&b| (1..=20).contains(&b)) {
            return <[u8; 8]>::try_from(run).ok();
        }
    }
    None
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

/// How far past the record prefix a person's own identity block sits.
///
/// Measured on a day-one save (`idgap` example) over the 62,584 identities the
/// ascending chain binds: median 145 bytes, 99th percentile 402, furthest
/// 1,180 — and 512 already covers 99.80%. The bound is what keeps the
/// out-of-order pass honest, because a record can also contain the *next*
/// person's second object, and that sits late in the span: Sterling's is 631
/// bytes before his own name. Binding it would name someone with another
/// person's ids.
const IDENTITY_WINDOW: usize = 512;

/// Finds identity blocks and attaches them to people.
///
/// Candidates are every `[eid][uid][uid]` triple preceded by three zero bytes.
/// That shape recurs by chance in contract data, so the true blocks are picked
/// as the longest strictly-eid-ascending chain — person records are written in
/// entity-id order, which noise does not follow. Each person takes the first
/// chain block inside their record.
///
/// Staff eids sit out of that order, so the chain drops them and pure-staff
/// records read `eid: None` — which left every fit against their attribute
/// block owned by a span guess rather than a matching id pair
/// (`OPEN_PROBLEMS.md` §3b). A second pass therefore takes the leftovers by
/// *shape*: see [`bind_out_of_order`].
///
/// Both passes are returned together, in file order, so squad references can
/// also be resolved through blocks that share a record.
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

    let mut bound = bind_out_of_order(&candidates, people, &offsets);
    bound.extend_from_slice(&chain);
    bound.sort_unstable_by_key(|id| id.offset);
    bound
}

/// Names the people the ascending chain could not, from the candidates it left
/// behind.
///
/// The chain proves a block by its position in an ordered sequence. That test
/// is unavailable here, so the block has to prove itself:
///
/// 1. **It is inside the record and near its front** — within
///    [`IDENTITY_WINDOW`], which excludes a neighbour's second object.
/// 2. **Its ids are unclaimed.** An eid or uid already *bound to a person*
///    belongs to them, not to this one. The test is against the people, not
///    against the chain: the chain also holds reference blocks that name
///    somebody else's entity from inside a third person's record, and those
///    name nobody — letting them reserve an id would lock the real owner out.
///    That is what kept Nikolić, Cornelli and Pfannenstiel unbound on the
///    first attempt.
/// 3. **It is the first such block in the record.** Candidates arrive in file
///    order and a person who already has an eid is skipped, so a record
///    holding two never picks the later one.
///
/// A record with nothing that passes keeps `None`. Missing an id is recoverable
/// downstream; a wrong one is not.
fn bind_out_of_order(
    candidates: &[Identity],
    people: &mut [Person],
    offsets: &[usize],
) -> Vec<Identity> {
    let entity_ids: std::collections::HashSet<u32> = people.iter().filter_map(|p| p.eid).collect();
    let database_ids: std::collections::HashSet<u32> =
        people.iter().filter_map(|p| p.uid).collect();

    let mut bound = Vec::new();
    for cand in candidates {
        if entity_ids.contains(&cand.eid) || database_ids.contains(&cand.uid) {
            continue;
        }
        let idx = offsets.partition_point(|&o| o <= cand.offset);
        let Some(i) = idx.checked_sub(1) else { continue };
        if offsets
            .get(i)
            .is_none_or(|&owner| cand.offset.saturating_sub(owner) > IDENTITY_WINDOW)
        {
            continue;
        }
        let Some(person) = people.get_mut(i) else { continue };
        if person.eid.is_some() {
            continue;
        }
        person.eid = Some(cand.eid);
        person.uid = Some(cand.uid);
        bound.push(*cand);
    }
    bound
}

/// Whether an `[eid][uid][uid]` triple is preceded by an entity object header:
/// a type byte of 0-2 and then `0x40`, seven bytes back.
fn has_object_header(frame: &[u8], at: usize) -> bool {
    let Some(head) = at.checked_sub(7) else {
        return false;
    };
    frame.get(head).is_some_and(|&b| b <= 0x02) && frame.get(head + 1) == Some(&0x40)
}

/// How far before the record prefix the contract block can sit.
const CONTRACT_WINDOW: usize = 220;

/// Latest plausible contract expiry year; beyond this is a misread.
const MAX_CONTRACT_YEAR: u16 = 2060;

/// Earliest plausible contract expiry year. Free agents carry a sentinel of
/// the null date's era (2 January 1900), which is "no expiry", not a date.
const MIN_CONTRACT_YEAR: u16 = 1950;

/// Reads each person's contract from the block preceding their record.
///
/// The block is anchored on the person's own entity id:
///
/// ```text
/// [eid u32] [u32] [00 00 00 00] [wage u32] 01 xx 00 [FF FF FF FF]
/// ```
///
/// with the contract expiry earlier in the block, as a date pair following a
/// run of eight `FF` bytes. Verified against FM Scout's Haaland figures
/// (£450,000 a week until 30/6/2034) and an in-game report in an aged save
/// (Musiala, £392,499 inside the scouted £350K-£425K band, until 30/6/2037).
/// Non-round wages are foreign-currency contracts converted to the display
/// currency. A person whose block does not match keeps `None` — the
/// unemployed and the retired genuinely have no contract to read.
pub fn bind_contracts(frame: &[u8], people: &mut [Person]) {
    for person in people.iter_mut() {
        let Some(eid) = person.eid else { continue };
        let lo = person.offset.saturating_sub(CONTRACT_WINDOW);
        let Some(p) = rfind_u32(frame, eid, lo, person.offset) else {
            continue;
        };
        if frame.get(p + 8..p + 12) != Some(&[0, 0, 0, 0][..]) {
            continue;
        }
        if frame.get(p + 16) != Some(&0x01)
            || frame.get(p + 18) != Some(&0x00)
            || frame.get(p + 19..p + 23) != Some(&[0xFF; 4][..])
        {
            continue;
        }
        let wage = read_u32(frame, p + 12);

        // Expiry: the date pair after the last 8xFF run before the anchor.
        let until = frame
            .get(lo..p)
            .and_then(|w| {
                w.windows(8).enumerate().rev().find(|(_, run)| run == &[0xFF; 8]).map(|(i, _)| lo + i)
            })
            .and_then(|q| {
                let day = read_u16(frame, q + 8)?;
                let year = read_u16(frame, q + 10)?;
                if !(MIN_CONTRACT_YEAR..=MAX_CONTRACT_YEAR).contains(&year) {
                    return None;
                }
                Date::from_day_of_year(day, year)
            });

        // A zero wage with no expiry is a free agent's sentinel row, not a
        // contract; a zero wage *with* an expiry is a real amateur deal.
        if wage == Some(0) && until.is_none() {
            continue;
        }
        person.wage = wage;
        person.contract_until = until;
    }
}

/// The gap between squad-median forename ids must be at least this wide to
/// count as the male/female split. A save without women's football has only
/// the scatter of one population, whose gaps are far smaller.
const MIN_GENDER_GAP: u32 = 5_000;

/// Derives the forename id at which the female name pool begins.
///
/// FM's forename pool stores female names as its tail: in the reference save
/// every known man's forename id is at most 246,372 and every known woman's
/// at least 246,373 ("Kaur", the exclusively female Sikh patronymic, opens
/// the female block). The exact number varies per database, so it is derived
/// from the save itself: each squad is single-gender, so squad *median*
/// forename ids form two clusters, and the widest gap between adjacent
/// medians is the boundary. Returns `None` when no gap is wide enough —
/// a save without women's football — in which case gender is unknown, not
/// assumed.
#[must_use]
pub fn female_forename_boundary(people: &[Person], squads: &[crate::squad::Squad]) -> Option<u32> {
    let by_eid: std::collections::HashMap<u32, u32> = people
        .iter()
        .filter_map(|p| Some((p.eid?, p.first_name_id)))
        .collect();

    let mut medians = Vec::new();
    for s in squads {
        let mut ids: Vec<u32> = s
            .player_eids
            .iter()
            .filter_map(|e| by_eid.get(e).copied())
            .collect();
        if ids.len() < 5 {
            continue;
        }
        ids.sort_unstable();
        if let Some(&m) = ids.get(ids.len() / 2) {
            medians.push(m);
        }
    }
    medians.sort_unstable();

    let (gap, boundary) = medians
        .windows(2)
        .filter_map(|w| match w {
            [a, b] => Some((b - a, a + (b - a) / 2)),
            _ => None,
        })
        .max()?;
    (gap >= MIN_GENDER_GAP).then_some(boundary)
}

/// Sets `Person::female` from the derived boundary.
pub fn bind_gender(people: &mut [Person], boundary: Option<u32>) {
    let Some(b) = boundary else { return };
    for p in people.iter_mut() {
        p.female = Some(p.first_name_id >= b);
    }
}

/// Last occurrence of a little-endian `u32` in `frame[lo..hi]`.
fn rfind_u32(frame: &[u8], value: u32, lo: usize, hi: usize) -> Option<usize> {
    let needle = value.to_le_bytes();
    let window = frame.get(lo..hi)?;
    window
        .windows(4)
        .enumerate()
        .rev()
        .find(|(_, w)| *w == needle)
        .map(|(i, _)| lo + i)
}

/// Finds every `[eid][uid][uid]` triple preceded by three zero bytes, minus
/// the shadows.
///
/// Reading the eid one byte early also passes this test — an entity object
/// header ends in zero bytes, so the short read gives `eid << 8` with the
/// repeated uid still lining up. The shadow is a valid-looking candidate
/// whenever `eid << 8` stays under [`MAX_EID`], i.e. below eid 11,718.
/// Accepting the shadow consumed the twelve bytes the true block needed, which
/// is how Nikolić (5156), Cornelli (1389) and Pfannenstiel (1858) went unnamed
/// while Fradley (20130) and Hutton (33829) survived — their shifted eids
/// overflow the bound. A hit is therefore dropped when the very next offset
/// carries a triple proven by an entity object header, `[type 00-02][0x40]`
/// seven bytes back.
///
/// The header cannot simply be *required* of every candidate: on a day-one
/// save that leaves 26,089 people unnamed against 1,091, so plenty of real
/// identity blocks are written without one. It only breaks the tie.
fn scan_triples(frame: &[u8], start: usize) -> Vec<Identity> {
    let mut out = Vec::new();
    let mut at = start.max(3);
    while at + 12 <= frame.len() {
        let zeros = frame.get(at - 3..at).is_some_and(|b| b == [0, 0, 0]);
        if !zeros {
            at += 1;
            continue;
        }
        let Some((eid, uid)) = read_triple(frame, at) else {
            at += 1;
            continue;
        };
        // A shadow sits exactly one byte in front of the block it hides, so
        // if the next offset carries a header-proven triple this hit is that
        // short read — step onto the real one instead of over it.
        if has_object_header(frame, at + 1) && read_triple(frame, at + 1).is_some() {
            at += 1;
            continue;
        }
        out.push(Identity {
            offset: at,
            eid,
            uid,
        });
        at += 12;
    }
    out
}

/// Reads `[eid][uid][uid]` at `at`, if the three words have that shape.
fn read_triple(frame: &[u8], at: usize) -> Option<(u32, u32)> {
    let eid = read_u32(frame, at)?;
    let a = read_u32(frame, at + 4)?;
    let b = read_u32(frame, at + 8)?;
    (a == b && a != 0 && a != u32::MAX && eid > 0 && eid < MAX_EID).then_some((eid, a))
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
    fn reads_the_hidden_personality_run() {
        // Emery's real shape: nation, then the 03-nation-zeros marker, then
        // eight 1-20 values with Professionalism 20 at slot 4.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Unai Emery Etxegoien"), 307, 1971);
        buf.extend_from_slice(&[0u8; 9]);
        buf.extend_from_slice(&170u16.to_le_bytes());
        buf.extend_from_slice(&[0x01, 0x06, 0x00, 0x00, 0x00]);
        buf.push(0x03);
        buf.extend_from_slice(&170u16.to_le_bytes());
        buf.extend_from_slice(&[0u8; 6]);
        buf.extend_from_slice(&[13, 15, 18, 12, 20, 16, 10, 8]);
        let found = scan_people(&buf, &table());
        let p = found.first().unwrap();
        assert_eq!(p.personality, Some([13, 15, 18, 12, 20, 16, 10, 8]));
        assert_eq!(p.adaptability(), Some(13));
        assert_eq!(p.ambition(), Some(15));
        assert_eq!(p.loyalty(), Some(18));
        assert_eq!(p.pressure(), Some(12));
        assert_eq!(p.professionalism(), Some(20));
        assert_eq!(p.sportsmanship(), Some(16));
        assert_eq!(p.temperament(), Some(10));
        assert_eq!(p.controversy(), Some(8));
    }

    #[test]
    fn a_record_without_the_marker_has_no_personality() {
        let buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        assert_eq!(scan_people(&buf, &table()).first().unwrap().personality, None);
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

    /// An identity as it sits on disk: the seven-byte entity object header —
    /// type byte, `0x40`, flags, then four zeros — and the triple after it.
    fn identity_block(eid: u32, uid: u32) -> Vec<u8> {
        let mut v = vec![0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00];
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
    fn a_low_eid_is_not_swallowed_by_the_shadow_hit_before_it() {
        // Reading the eid one byte early gives `eid << 8` with the repeated
        // uid still lining up, and that shadow passes every candidate test
        // while `eid << 8` stays under MAX_EID. Nikolić's real numbers: eid
        // 5156 shifts to 1,320,960, well inside the bound.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(5156, 5_790_125));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);

        let bound = people.first().unwrap();
        assert_eq!(bound.eid, Some(5156), "the shadow at eid << 8 must not win");
        assert_eq!(bound.uid, Some(5_790_125));
    }

    #[test]
    fn an_out_of_order_identity_still_names_its_record() {
        // Staff eids do not follow the ascending order the chain relies on,
        // so the second record's block loses the chain race and has to be
        // taken on shape instead.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(40_000, 9_000_001));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block(1858, 434_431));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);

        assert_eq!(people.first().unwrap().eid, Some(40_000));
        assert_eq!(people.get(1).unwrap().eid, Some(1858), "descending eid, same record");
        assert_eq!(people.get(1).unwrap().uid, Some(434_431));
    }

    #[test]
    fn an_identity_deep_in_the_record_is_refused() {
        // A record can hold the *next* person's second object, which sits far
        // past the front. Binding it would name someone with another person's
        // ids, so the out-of-order pass leaves anything beyond the window
        // alone. Two ascending records first, so the chain has a run of its
        // own and the deep block is genuinely out of order.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(40_000, 9_000_001));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block(41_000, 9_000_002));
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(std::iter::repeat_n(0x11u8, IDENTITY_WINDOW + 16));
        buf.extend(identity_block(1858, 434_431));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);

        assert_eq!(people.first().unwrap().eid, Some(40_000));
        assert_eq!(people.get(1).unwrap().eid, Some(41_000));
        assert_eq!(people.get(2).unwrap().eid, None, "too deep to be their own");
    }

    #[test]
    fn an_out_of_order_pass_never_reuses_a_bound_id() {
        // The chain names the first two records. The third repeats a uid the
        // chain already used — a reference, not an identity — and must be
        // refused rather than give two people the same id.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(40_000, 9_000_001));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block(41_000, 9_000_002));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block(1858, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);

        assert_eq!(people.first().unwrap().eid, Some(40_000));
        assert_eq!(people.get(1).unwrap().eid, Some(41_000));
        assert_eq!(people.get(2).unwrap().eid, None, "the uid is already spoken for");
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

    /// Builds a contract block for `eid`: expiry after an 8xFF run, then the
    /// eid-anchored wage row, as found on disk before the record prefix.
    fn contract(eid: u32, wage: u32, expiry_doy: u16, expiry_year: u16) -> Vec<u8> {
        let mut v = vec![0u8; 8];
        v.extend_from_slice(&[0xFF; 8]);
        v.extend_from_slice(&expiry_doy.to_le_bytes());
        v.extend_from_slice(&expiry_year.to_le_bytes());
        v.extend_from_slice(&[0u8; 12]);
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&501u32.to_le_bytes());
        v.extend_from_slice(&[0u8; 4]);
        v.extend_from_slice(&wage.to_le_bytes());
        v.extend_from_slice(&[0x01, 0x0B, 0x00]);
        v.extend_from_slice(&[0xFF; 4]);
        v.extend_from_slice(&[0u8; 6]);
        v
    }

    #[test]
    fn reads_the_wage_and_expiry_from_the_contract_block() {
        // Haaland's real figures: £450K a week until 30 June 2034.
        let mut buf = contract(50, 450_000, 181, 2034);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        let p = people.first().unwrap();
        assert_eq!(p.wage, Some(450_000));
        let until = p.contract_until.unwrap();
        assert_eq!((until.year, until.month, until.day), (2034, 6, 30));
    }

    #[test]
    fn no_contract_block_means_no_wage() {
        // Retired and unemployed people have nothing before their record.
        let mut buf = vec![0u8; 64];
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().wage, None);
        assert_eq!(people.first().unwrap().contract_until, None);
    }

    #[test]
    fn a_free_agents_sentinel_row_is_not_a_contract() {
        // Free agents carry wage 0 with the null-era expiry (2 January 1900).
        // Showing that as a contract is what made club-less players look
        // employed in the UI.
        let mut buf = contract(50, 0, 2, 1900);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().wage, None);
        assert_eq!(people.first().unwrap().contract_until, None);
    }

    #[test]
    fn an_amateur_deal_keeps_its_zero_wage() {
        // A real expiry with no wage is an amateur contract, not a sentinel.
        let mut buf = contract(50, 0, 151, 2027);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().wage, Some(0));
        assert_eq!(people.first().unwrap().contract_until.map(|d| d.year), Some(2027));
    }

    #[test]
    fn a_wage_row_with_the_wrong_shape_is_not_a_contract() {
        // Same eid nearby but without the 01-xx-00-FFFFFFFF tail.
        let mut buf = vec![0u8; 16];
        buf.extend_from_slice(&50u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 24]);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().wage, None);
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let full = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        for cut in 0..full.len() {
            let _ = scan_people(full.get(..cut).unwrap(), &table());
        }
    }
}
