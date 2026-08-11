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
    /// The common name, resolved from the common-name pool when the record
    /// references one — "Raúl", "Rivaldo", and the surname-first orderings
    /// FM displays for East Asian players ("Yoon Jung-Hwan"). `None` when the
    /// record has no reference or the id does not resolve. FM displays this
    /// over the full name everywhere, so [`Person::display_name`] does too.
    pub common_name: Option<String>,
    /// `None` for compact entries, which carry no birth date at all.
    pub date_of_birth: Option<Date>,
    /// Nation identifier, e.g. 139 for England. Shares the numbering the club
    /// records use, so a club's nation and a player's match. `None` when the
    /// record does not carry one — compact entries, and records truncated at
    /// the end of the frame.
    pub nation_id: Option<u16>,
    /// The person's database entity id — what squad lists reference.
    /// `None` when no identity block was found in the record.
    pub eid: Option<u32>,
    /// The person's second identifier, repeated beside the entity id on disk.
    pub uid: Option<u32>,
    /// Entity id of the club whose first-team squad lists this person.
    /// Filled from the squad table by `Save::parse`, `None` for the unattached.
    pub club_eid: Option<u32>,
    /// Which of the club's squad lists bound `club_eid` — first team, B team,
    /// youth, or a senior list outside the loaded leagues standing in for a
    /// first team the game never materialised. `None` for anyone the squad
    /// table did not place: staff, the unattached, and B/youth-registered
    /// people whose club came from the backroom lists instead. A person can
    /// genuinely sit in more than one of a club's lists (a youth player also
    /// named among the B squad); this holds whichever list is most senior,
    /// see [`crate::squad::SquadKind::seniority`].
    pub squad_level: Option<crate::squad::SquadKind>,
    /// Whether this person is a woman, read from the save's own record: bit
    /// 0x10 of the type byte that opens their identity object's header, seven
    /// bytes before the entity id. Verified by squad purity — across a
    /// day-one, a 2030 and a 2035 save not one squad mixes the bit — where
    /// the retired forename-pool inference misfiled whole foreign squads.
    /// `None` only when the person's identity block was never found.
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
    /// Minimum fee release clause in the save's display currency, from the
    /// type-0x26 row of the contract's money list. Verified against
    /// published FM26 figures: Pedri's €1B buyout reads 864,206,784 — the
    /// game's own conversion — and the €60M La Liga default lands byte-for
    /// byte on a fleet of players. `None` when the row reads the unset
    /// sentinel, the list is absent, or no contract parses at all — no
    /// clause decoded, which is not proof none exists.
    pub release_clause: Option<u32>,
    /// Ability and attributes, when this person has an attribute block.
    /// `None` means staff: only players carry one.
    pub ability: Option<crate::ability::Ability>,
    /// Non-player attributes — the editor's "All Attributes" sheet — read from
    /// the entity object one eid below this person's own. `None` when no such
    /// object carries a block, which is most players.
    pub staff: Option<crate::staff::Staff>,
    /// Game reputation, bound from the player line behind the same
    /// one-eid-below object the staff sheet uses — and only where the line's
    /// CA/PA pair repeats this person's parsed ability exactly, so a
    /// misattributed line cannot bind. `None` is undecoded, not obscurity.
    pub reputation: Option<Reputation>,
    /// True for a compact entry: aged saves fold people who have left the
    /// loaded game world down to a name reference and an identity, so only
    /// `full_name`, `eid` and `uid` are real. Everything else is genuinely
    /// absent from the save, not undecoded.
    pub compact: bool,
    /// The person's decoded place in a club's backroom — the manager seat,
    /// or the department whose staff list names them. `None` for players,
    /// the unemployed, and anyone bound through a list outside the
    /// department triple.
    pub staff_role: Option<crate::backroom::Role>,
    /// Whether a national side's squad list names this person — the
    /// representative rows of the squad table
    /// ([`crate::squad::scan_representative_squads`]). `false` means no
    /// decoded list names them, which is not proof they are uncapped: the
    /// save only materialises the selections it has needed so far.
    pub in_national_squad: bool,
}

/// A person's three game reputations, on the editor's raw 0-200 scale — the
/// same scale the staff sheet's reputations use. The save stores them ×50
/// (0-10000); Haaland's day-one line reads back his editor page exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reputation {
    /// Standing in the person's home nation.
    pub home: u16,
    /// Standing where they currently play.
    pub current: u16,
    /// Worldwide standing — the one that decides who takes your call.
    pub world: u16,
}

impl Person {
    /// Whether this person is a player. Staff have no attribute block.
    #[must_use]
    pub fn is_player(&self) -> bool {
        self.ability.is_some()
    }

    /// The name FM displays: the common name when the person has one
    /// ("Juanito", "Raúl"), the full name otherwise.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.common_name.as_deref().unwrap_or(&self.full_name)
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
        self.nation_id.and_then(nation_name)
    }
}

/// Nation names for identifiers confirmed against the save's own records.
///
/// The file does not store the name beside the identifier — every occurrence
/// of "England" in the database frame is the *surname* England — so the first
/// batch was derived by grouping every person by nation identifier and reading
/// the best players in each group. A national squad is unmistakable: Courtois,
/// De Bruyne, Lukaku and Tielemans fix 131; Modrić, Gvardiol and Kovačić fix
/// 135; Kvaratskhelia and Mamardashvili fix 144; Son, Kim Min-Jae and Lee
/// Kang-In fix 80.
///
/// **Club records name the rest outright.** They carry the same numbering —
/// German clubs and Wirtz both report 145 — and unlike people they store their
/// names in the clear. "Ba FC, Labasa FC, Lautoka FC, Nadi FC" is Fiji and
/// nothing else, where a squad of Fijian-sounding players only suggests it. A
/// save with every league loaded named 78 identifiers this way at once, most of
/// them the small federations a squad-reading pass could never settle.
///
/// The numbering corroborates them: it runs **broadly alphabetical within each
/// confederation**, on FM's own older names — 43 Tanzania, 44 The Congo, 45
/// Togo, 46 Tunisia, 47 Uganda, 48 Zaire, 49 Zambia; 62 Jordan, 63 Kampuchea,
/// 64 Kazakhstan; 85 Thailand, 86 The Philippines, 87 Turkmenistan; 117
/// Suriname, 118 The Bahamas, 119 Trinidad and Tobago. Every name the clubs
/// gave lands where that ordering puts it. It is corroboration and not proof —
/// 114 Saint Lucia sits ahead of 115 Saint Kitts and Nevis — so where the two
/// disagree the clubs win.
///
/// Reading the clubs also **corrected 116**, which was Cayman Islands here on
/// the strength of its players' names. Its clubs are Avenues United, Layou FC
/// and North Leeward Predators: Saint Vincent and the Grenadines. Cayman is 98,
/// where Bodden Town FC and Scholars International play.
///
/// Identifiers are left unnamed rather than guessed where neither settles the
/// country: 213 is four British Army regimental sides, 123 a single East German
/// club, and 238 three people with no clubs at all. A wrong flag is worse than
/// a number.
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
        3 => Some("Botswana"),
        4 => Some("Burkina Faso"),
        5 => Some("Burundi"),
        6 => Some("Cameroon"),
        7 => Some("Cape Verde"),
        8 => Some("Central African Republic"),
        9 => Some("Chad"),
        10 => Some("Djibouti"),
        11 => Some("Egypt"),
        12 => Some("Equatorial Guinea"),
        13 => Some("Ethiopia"),
        14 => Some("Gabon"),
        15 => Some("Gambia"),
        16 => Some("Ghana"),
        17 => Some("Guinea"),
        18 => Some("Guinea-Bissau"),
        19 => Some("Ivory Coast"),
        20 => Some("Kenya"),
        21 => Some("Lesotho"),
        22 => Some("Liberia"),
        23 => Some("Libya"),
        24 => Some("Madagascar"),
        25 => Some("Malawi"),
        26 => Some("Mali"),
        27 => Some("Mauritania"),
        28 => Some("Mauritius"),
        29 => Some("Morocco"),
        30 => Some("Mozambique"),
        31 => Some("Namibia"),
        32 => Some("Niger"),
        33 => Some("Nigeria"),
        34 => Some("Rwanda"),
        35 => Some("São Tomé and Príncipe"),
        36 => Some("Senegal"),
        37 => Some("Seychelles"),
        38 => Some("Sierra Leone"),
        39 => Some("Somalia"),
        40 => Some("South Africa"),
        41 => Some("Sudan"),
        42 => Some("Eswatini"),
        43 => Some("Tanzania"),
        44 => Some("Congo"),
        45 => Some("Togo"),
        46 => Some("Tunisia"),
        47 => Some("Uganda"),
        48 => Some("DR Congo"),
        49 => Some("Zambia"),
        50 => Some("Zimbabwe"),
        51 => Some("Afghanistan"),
        52 => Some("Bahrain"),
        53 => Some("Bangladesh"),
        54 => Some("Brunei"),
        55 => Some("China"),
        56 => Some("Hong Kong"),
        57 => Some("India"),
        58 => Some("Indonesia"),
        59 => Some("Iran"),
        60 => Some("Iraq"),
        61 => Some("Japan"),
        62 => Some("Jordan"),
        63 => Some("Cambodia"),
        64 => Some("Kazakhstan"),
        65 => Some("Kuwait"),
        66 => Some("Kyrgyzstan"),
        67 => Some("Laos"),
        68 => Some("Lebanon"),
        69 => Some("Macau"),
        70 => Some("Malaysia"),
        71 => Some("Maldives"),
        72 => Some("Myanmar"),
        73 => Some("Nepal"),
        74 => Some("North Korea"),
        75 => Some("Oman"),
        76 => Some("Pakistan"),
        77 => Some("Qatar"),
        78 => Some("Saudi Arabia"),
        79 => Some("Singapore"),
        80 => Some("South Korea"),
        81 => Some("Sri Lanka"),
        82 => Some("Syria"),
        83 => Some("Chinese Taipei"),
        84 => Some("Tajikistan"),
        85 => Some("Thailand"),
        86 => Some("Philippines"),
        87 => Some("Turkmenistan"),
        88 => Some("United Arab Emirates"),
        89 => Some("Uzbekistan"),
        90 => Some("Vietnam"),
        91 => Some("Yemen"),
        92 => Some("Antigua and Barbuda"),
        93 => Some("Aruba"),
        94 => Some("Barbados"),
        95 => Some("Belize"),
        96 => Some("Bermuda"),
        97 => Some("Canada"),
        98 => Some("Cayman Islands"),
        99 => Some("Costa Rica"),
        100 => Some("Cuba"),
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
        111 => Some("Nicaragua"),
        112 => Some("Panama"),
        113 => Some("Puerto Rico"),
        114 => Some("Saint Lucia"),
        115 => Some("Saint Kitts and Nevis"),
        116 => Some("Saint Vincent and the Grenadines"),
        117 => Some("Suriname"),
        118 => Some("Bahamas"),
        119 => Some("Trinidad and Tobago"),
        120 => Some("United States"),
        126 => Some("Albania"),
        127 => Some("Andorra"),
        128 => Some("Armenia"),
        129 => Some("Austria"),
        130 => Some("Azerbaijan"),
        131 => Some("Belgium"),
        132 => Some("Belarus"),
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
        152 => Some("Liechtenstein"),
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
        166 => Some("San Marino"),
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
        178 => Some("Cook Islands"),
        179 => Some("Fiji"),
        180 => Some("New Zealand"),
        181 => Some("Papua New Guinea"),
        182 => Some("Solomon Islands"),
        183 => Some("Tahiti"),
        184 => Some("Tonga"),
        185 => Some("Vanuatu"),
        186 => Some("Samoa"),
        187 => Some("Argentina"),
        188 => Some("Bolivia"),
        189 => Some("Brazil"),
        190 => Some("Chile"),
        191 => Some("Colombia"),
        192 => Some("Ecuador"),
        193 => Some("Paraguay"),
        194 => Some("Peru"),
        195 => Some("Uruguay"),
        196 => Some("Venezuela"),
        197 => Some("Palestine"),
        201 => Some("American Samoa"),
        202 => Some("Mongolia"),
        203 => Some("Guam"),
        204 => Some("Eritrea"),
        205 => Some("Anguilla"),
        206 => Some("British Virgin Islands"),
        207 => Some("Montserrat"),
        208 => Some("US Virgin Islands"),
        209 => Some("Turks and Caicos Islands"),
        210 => Some("New Caledonia"),
        211 => Some("Bhutan"),
        212 => Some("Dominican Republic"),
        215 => Some("Kiribati"),
        216 => Some("Gibraltar"),
        217 => Some("Bonaire"),
        218 => Some("Crimea"),
        219 => Some("Kosovo"),
        225 => Some("French Guiana"),
        226 => Some("Guadeloupe"),
        227 => Some("Martinique"),
        228 => Some("Sint Maarten"),
        229 => Some("Saint Martin"),
        230 => Some("Réunion"),
        231 => Some("Mayotte"),
        232 => Some("Wallis and Futuna"),
        233 => Some("Saint Pierre and Miquelon"),
        234 => Some("Comoros"),
        236 => Some("Timor-Leste"),
        239 => Some("Zanzibar"),
        240 => Some("South Sudan"),
        242 => Some("Micronesia"),
        243 => Some("Northern Mariana Islands"),
        244 => Some("Tuvalu"),
        247 => Some("Montenegro"),
        249 => Some("Saint Barthélemy"),
        _ => None,
    }
}

const NO_COMMON_NAME: u32 = 0xFFFF_FFFF;
const MIN_NAME_LEN: usize = 2;
const MAX_NAME_LEN: usize = 64;

/// Bytes past the date of birth where the nation identifier sits.
const NATION_OFFSET: usize = 13;

/// A nation identifier this large is not a nation, so the bytes that produced
/// it were not a person.
///
/// FM's nation table runs to about 250 entries: the highest identifier any
/// person or club carries across the saves here is 249, Saint-Barthélemy. The
/// records that read past it read *far* past it — 1280, 8704, 45209 — because
/// they are not records at all but other tables' bytes that happen to satisfy
/// the prefix. On a day-one save the ceiling drops 1,043 of them, none of which
/// has a club, and it takes with them every person whose date of birth put them
/// outside a footballing lifetime: the 9-year-olds and the 109-year-old that
/// were the visible symptom. The bound is set well clear of both numbers so a
/// nation FM adds later is not mistaken for noise.
const MAX_NATION_ID: u16 = 512;

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
    // frame still parses — an absent nation is descriptive, not disqualifying.
    // One that reads past the end of FM's nation table is another matter: it
    // says these bytes are some other table's, and the record goes.
    let nation_id = read_u16(frame, body + NATION_OFFSET);
    if nation_id.is_some_and(|n| n > MAX_NATION_ID) {
        return None;
    }
    let personality = nation_id.and_then(|n| find_personality(frame, body, n));

    Some((
        Person {
            offset: at,
            first_name_id,
            surname_id,
            common_name_id: (common_raw != NO_COMMON_NAME).then_some(common_raw),
            common_name: (common_raw != NO_COMMON_NAME)
                .then(|| strings.common_names.get(&common_raw).cloned())
                .flatten(),
            full_name,
            date_of_birth: Some(date_of_birth),
            nation_id,
            eid: None,
            uid: None,
            club_eid: None,
            squad_level: None,
            female: None,
            personality,
            wage: None,
            contract_until: None,
            release_clause: None,
            // Filled in by `Save::parse`, which matches blocks to people once
            // both scans have run.
            ability: None,
            staff: None,
            reputation: None,
            compact: false,
            staff_role: None,
            in_national_squad: false,
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

/// Length of one compact person entry.
const COMPACT_LEN: usize = 30;

/// Scans a frame for compact person entries:
///
/// ```text
/// 10 00 [forename id u32] [surname id u32] 01 [type] 40 [flags] 04 00 00 00 [eid][uid][uid]
/// ```
///
/// Aged saves fold people who have left the loaded game world — retired, or
/// moved beyond the simulated leagues — down to this: a name reference and an
/// entity object, sitting in the person table in eid order between full
/// records but carrying no record prefix of their own. Kylian Mbappé is
/// stored this way in a 2035 save (976 entries, every name resolving); a
/// day-one save has none. The type byte is 0-2 plus the informational bits —
/// the gender flag among them — and the flags byte varies, exactly as on the
/// other entity object headers (`SAVE_FORMAT.md` §3).
///
/// Acceptance is the doubled uid plus **both name ids resolving in their
/// pools** — the same test a full record must pass — so a chance `10 00` in
/// record data does not fabricate a person. Everything beyond name and
/// identity is genuinely absent, so the person is marked [`Person::compact`]
/// and every other field stays `None`.
#[must_use]
pub fn scan_compact(frame: &[u8], strings: &StringTable, start: usize) -> Vec<Person> {
    let mut out = Vec::new();
    let mut at = start;
    while at.saturating_add(COMPACT_LEN) <= frame.len() {
        match compact_at(frame, at, strings) {
            Some(person) => {
                out.push(person);
                at += COMPACT_LEN;
            }
            None => at += 1,
        }
    }
    out
}

fn compact_at(frame: &[u8], at: usize, strings: &StringTable) -> Option<Person> {
    if frame.get(at)? != &0x10 || frame.get(at + 1)? != &0x00 {
        return None;
    }
    if frame.get(at + 10)? != &0x01 {
        return None;
    }
    // The type byte carries the gender flag and the aged-save 0x20 bit on
    // top of its 0-2 value; testing it raw rejected every compact woman.
    let type_byte = *frame.get(at + 11)?;
    if type_byte & !TYPE_INFO_BITS > 0x02 || frame.get(at + 12)? != &0x40 {
        return None;
    }
    if frame.get(at + 14..at + 18)? != [0x04, 0x00, 0x00, 0x00] {
        return None;
    }
    let eid = read_u32(frame, at + 18)?;
    let uid = read_u32(frame, at + 22)?;
    if read_u32(frame, at + 26)? != uid || uid == 0 || uid == u32::MAX {
        return None;
    }
    if eid == 0 || eid >= MAX_EID {
        return None;
    }
    let first_name_id = read_u32(frame, at + 2)?;
    let surname_id = read_u32(frame, at + 6)?;
    let forename = strings.forenames.get(&first_name_id)?;
    let surname = strings.surnames.get(&surname_id)?;

    Some(Person {
        offset: at,
        first_name_id,
        surname_id,
        common_name_id: None,
        common_name: None,
        full_name: format!("{forename} {surname}"),
        date_of_birth: None,
        nation_id: None,
        eid: Some(eid),
        uid: Some(uid),
        club_eid: None,
        squad_level: None,
        female: Some(type_byte & FEMALE_BIT != 0),
        personality: None,
        wage: None,
        contract_until: None,
        release_clause: None,
        ability: None,
        staff: None,
        reputation: None,
        compact: true,
        staff_role: None,
        in_national_squad: false,
    })
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
    /// Bit 0x10 of the header's type byte, seven bytes before the eid —
    /// FM's own gender flag. `None` when the frame ends too early to read it.
    pub female: Option<bool>,
}

/// The gender bit inside the identity-object header's type byte.
const FEMALE_BIT: u8 = 0x10;

/// Informational bits the type byte carries on top of the 0-2 type value:
/// [`FEMALE_BIT`], and 0x20, which appears on aged saves (newgens among
/// others). Masked off before the type test, so a woman's header is still a
/// header — requiring the raw byte to be 0-2 silently rejected every female
/// identity object.
const TYPE_INFO_BITS: u8 = 0x30;

/// Reads the gender flag from the type byte seven bytes before a triple.
fn read_female_flag(frame: &[u8], triple_at: usize) -> Option<bool> {
    let byte = triple_at.checked_sub(7).and_then(|i| frame.get(i))?;
    Some(byte & FEMALE_BIT != 0)
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
            person.female = id.female;
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
        person.female = cand.female;
        bound.push(*cand);
    }
    bound
}

/// Whether an `[eid][uid][uid]` triple is preceded by an entity object header:
/// a type byte of 0-2 — ignoring the informational bits it also carries, the
/// gender flag among them — and then `0x40`, seven bytes back.
fn has_object_header(frame: &[u8], at: usize) -> bool {
    let Some(head) = at.checked_sub(7) else {
        return false;
    };
    frame.get(head).is_some_and(|&b| b & !TYPE_INFO_BITS <= 0x02)
        && frame.get(head + 1) == Some(&0x40)
}

/// How far before the record prefix the contract block can sit. It is not the
/// nearest block: a player abroad or in a B team carries other eid-anchored
/// rows between the contract and the record prefix (Jae-Wan Choi's sits 337
/// bytes back, behind an international-duty row), and on a 2035 save the
/// measured tail runs to ~600 bytes with nothing beyond.
const CONTRACT_WINDOW: usize = 600;

/// How far before the wage anchor the expiry's 8xFF run can sit. Measured on
/// 141K contracts in an aged save: all but 13 sit within 400 bytes.
const EXPIRY_WINDOW: usize = 400;

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
///
/// The hunt walks backwards through *every* occurrence of the eid, not just
/// the last: the record prefix is also preceded by other rows anchored on the
/// same eid — international duty, a B-team place — whose tails do not read
/// `01 xx 00 FF FF FF FF`. Demanding the shape of the last occurrence threw
/// the contract away whenever one of those rows sat between it and the
/// record, which on a 2035 save was 63,000 contracted players shown with no
/// wage — and every one of them misfiled by a "free agent" filter that
/// reads no contract *and* no club as unemployment.
pub fn bind_contracts(frame: &[u8], people: &mut [Person]) {
    for person in people.iter_mut() {
        let Some(eid) = person.eid else { continue };
        let lo = person.offset.saturating_sub(CONTRACT_WINDOW);
        let Some(p) = rfind_contract_anchor(frame, eid, lo, person.offset) else {
            continue;
        };
        let wage = read_u32(frame, p + 12);

        // Expiry: the date pair after the last 8xFF run before the anchor.
        let expiry_lo = p.saturating_sub(EXPIRY_WINDOW);
        let until = frame
            .get(expiry_lo..p)
            .and_then(|w| {
                w.windows(8)
                    .enumerate()
                    .rev()
                    .find(|(_, run)| run == &[0xFF; 8])
                    .map(|(i, _)| expiry_lo + i)
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
        person.release_clause = find_release_clause(frame, p);
    }
}

/// How far before the wage anchor the contract's money list can start.
/// Measured shapes sit within ~215 bytes; the margin covers longer lists.
const MONEY_LIST_WINDOW: usize = 300;

/// Most rows seen in a money list is five; a count beyond this is not one.
const MAX_MONEY_ROWS: usize = 12;

/// The money-list row type carrying the minimum fee release clause.
const RELEASE_CLAUSE_TYPE: u16 = 0x26;

/// Reads the minimum fee release clause from the contract's money list.
///
/// The list sits before the wage anchor:
///
/// ```text
/// 00 00 00 [count u8] [u32 value] FF FF
///   then per further row: [u16 type] [u32 value] FF FF
/// ```
///
/// with the last row's two tail bytes belonging to whatever follows. The
/// clause is the type-0x26 row — always last where present — reading
/// `FF FF FF FF` when the contract has no clause. Values are the save's
/// display currency, exactly as the wage is: Pedri's €1,000,000,000 buyout
/// reads 864,206,784 through the game's own rate, the €60M La Liga default
/// reads 51,852,408 on Berenguer, Joan Jordán and the rest of the fleet,
/// and Haaland's clause-less contract still carries the row, unset. A block
/// with no recognisable list yields `None` — undecoded, not clause-free.
fn find_release_clause(frame: &[u8], anchor: usize) -> Option<u32> {
    let lo = anchor.saturating_sub(MONEY_LIST_WINDOW);
    let mut at = anchor;
    while at > lo + 4 {
        at -= 1;
        // The count marker: three zero bytes then a small count.
        if frame.get(at..at + 3) != Some(&[0u8; 3][..]) {
            continue;
        }
        let count = usize::from(*frame.get(at + 3)?);
        if !(1..=MAX_MONEY_ROWS).contains(&count) {
            continue;
        }
        // First row: bare value, FF FF tail.
        if frame.get(at + 8..at + 10) != Some(&[0xFF, 0xFF][..]) {
            continue;
        }
        // Typed rows follow; every tail but the last must read FF FF.
        let mut clause = None;
        let mut shape_holds = true;
        for i in 1..count {
            let row = at + 10 + (i - 1) * 8;
            let Some(ty) = read_u16(frame, row) else {
                shape_holds = false;
                break;
            };
            let last = i == count - 1;
            if !last && frame.get(row + 6..row + 8) != Some(&[0xFF, 0xFF][..]) {
                shape_holds = false;
                break;
            }
            if ty == RELEASE_CLAUSE_TYPE {
                clause = read_u32(frame, row + 2);
            }
        }
        if !shape_holds {
            continue;
        }
        return clause.filter(|&v| v != 0 && v != u32::MAX);
    }
    None
}

/// Names the employer of contracted people no squad list claimed.
///
/// The contract anchor's second u32 — `[eid][team u32][00 x4][wage]` — is
/// the employing team's id: the squad-table row ordinal plus one, verified
/// exactly for every one of 1,462 clubs with five or more anchored
/// first-team players on a day-one save. For a club outside the loaded
/// leagues that row sits *empty* until the game materialises it, which is
/// why its players bound to nothing — but the row exists, so the ordinal
/// still resolves ([`crate::squad::employer_ordinals`]). Depay → COR and
/// Jorginho → FLA on day one, matching FM's own search.
///
/// Runs last and only fills gaps: a person a squad list, backroom list or
/// team list already placed keeps that link — for a loanee the contract
/// names the *owning* club while the lists name where they play, and the
/// lists are what FM displays. Only people whose contract actually read
/// (wage or expiry) are touched, which keeps the free-agent sentinel rows
/// out. `squad_level` stays `None`: no squad list claims these people, and
/// saying which list bound them would be an invention.
#[expect(clippy::implicit_hasher, reason = "internal map, always the default hasher")]
pub fn link_employers(
    frame: &[u8],
    people: &mut [Person],
    ordinals: &std::collections::HashMap<u32, u32>,
) {
    for person in people.iter_mut() {
        if person.club_eid.is_some() || person.compact {
            continue;
        }
        if person.wage.is_none() && person.contract_until.is_none() {
            continue;
        }
        let Some(team) = contract_team_id(frame, person) else {
            continue;
        };
        let Some(&club) = team.checked_sub(1).and_then(|o| ordinals.get(&o)) else {
            continue;
        };
        person.club_eid = Some(club);
    }
}

/// The employing team's id from a person's contract block — the second u32
/// of the contract anchor, which is the employer's squad-table row ordinal
/// plus one. `None` when no contract-shaped row anchors on the person's eid.
#[must_use]
pub fn contract_team_id(frame: &[u8], person: &Person) -> Option<u32> {
    let eid = person.eid?;
    let lo = person.offset.saturating_sub(CONTRACT_WINDOW);
    let anchor = rfind_contract_anchor(frame, eid, lo, person.offset)?;
    read_u32(frame, anchor + 4)
}

/// Last occurrence of `eid` in `frame[lo..hi]` that anchors a contract-shaped
/// row — `[eid][u32][00 00 00 00][wage u32] 01 xx 00 [FF FF FF FF]`. Earlier
/// occurrences are tried when a later one fails the shape, because the same
/// eid also anchors non-contract rows nearer the record prefix.
fn rfind_contract_anchor(frame: &[u8], eid: u32, lo: usize, hi: usize) -> Option<usize> {
    let needle = eid.to_le_bytes();
    let window = frame.get(lo..hi)?;
    window
        .windows(4)
        .enumerate()
        .rev()
        .filter(|(_, w)| *w == needle)
        .map(|(i, _)| lo + i)
        .find(|&p| contract_shape_at(frame, p))
}

/// Whether the bytes at `p` carry the contract row's verified tail.
fn contract_shape_at(frame: &[u8], p: usize) -> bool {
    frame.get(p + 8..p + 12) == Some(&[0, 0, 0, 0][..])
        && frame.get(p + 16) == Some(&0x01)
        && frame.get(p + 18) == Some(&0x00)
        && frame.get(p + 19..p + 23) == Some(&[0xFF; 4][..])
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
        // The header tell misses identities written without one. The values
        // are their own tell there: a shadow's eid and uid both end in a zero
        // byte, and the very next offset reads them shifted back — Verberne,
        // really eid 2057 / uid 601116, was bound as 526592 / 153885696 and
        // took the staff sheet behind his record down with him.
        if eid.trailing_zeros() >= 8
            && uid.trailing_zeros() >= 8
            && read_triple(frame, at + 1) == Some((eid >> 8, uid >> 8))
        {
            at += 1;
            continue;
        }
        out.push(Identity {
            offset: at,
            eid,
            uid,
            female: read_female_flag(frame, at),
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

    /// The nine bytes between the date of birth and the nation identifier,
    /// then the identifier. Fixtures that append anything after a record need
    /// this: whatever follows is read as the nation, and a value past the end
    /// of FM's nation table now disqualifies the record.
    fn nation_field(id: u16) -> Vec<u8> {
        let mut v = vec![0u8; 9];
        v.extend_from_slice(&id.to_le_bytes());
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
        let dob = p.date_of_birth.unwrap();
        assert_eq!((dob.day, dob.month), (21, 7));
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
    fn resolves_the_common_name_and_displays_it() {
        let mut strings = table();
        strings.common_names.insert(555, "Vinícius Júnior".to_owned());
        let buf = record(100, 200, 555, Some("Vinícius José de Oliveira Júnior"), 194, 2000);
        let found = scan_people(&buf, &strings);
        let p = found.first().unwrap();
        assert_eq!(p.common_name.as_deref(), Some("Vinícius Júnior"));
        assert_eq!(p.display_name(), "Vinícius Júnior");
        // The legal name is untouched — the common name sits beside it.
        assert_eq!(p.full_name, "Vinícius José de Oliveira Júnior");
    }

    #[test]
    fn an_unresolved_common_name_id_displays_the_full_name() {
        // The pool not holding the id is undecoded, not licence to invent:
        // the id is kept, the name field stays honest.
        let buf = record(100, 200, 555, Some("Vinícius José de Oliveira Júnior"), 194, 2000);
        let found = scan_people(&buf, &table());
        let p = found.first().unwrap();
        assert_eq!(p.common_name_id, Some(555));
        assert_eq!(p.common_name, None);
        assert_eq!(p.display_name(), "Vinícius José de Oliveira Júnior");
    }

    #[test]
    fn reads_the_nation_identifier() {
        // 139 is England; the field sits 13 bytes past the date of birth.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Bukayo Ayoyinka Saka"), 248, 2001);
        buf.extend_from_slice(&[0u8; 9]);
        buf.extend_from_slice(&139u16.to_le_bytes());
        let found = scan_people(&buf, &table());
        assert_eq!(found.first().unwrap().nation_id, Some(139));
        assert_eq!(found.first().unwrap().nation(), Some("England"));
    }

    #[test]
    fn names_the_nations_the_club_records_settled() {
        // 98 and 116 were one entry between them before the club names were
        // read: Cayman's clubs are Bodden Town and Scholars International,
        // Saint Vincent's are Avenues United and Layou FC.
        assert_eq!(nation_name(98), Some("Cayman Islands"));
        assert_eq!(nation_name(116), Some("Saint Vincent and the Grenadines"));
        assert_eq!(nation_name(179), Some("Fiji"));
        assert_eq!(nation_name(249), Some("Saint Barthélemy"));
        // Four British Army regimental sides name no country, so 213 stays a
        // number rather than becoming a guess.
        assert_eq!(nation_name(213), None);
    }

    #[test]
    fn rejects_a_record_whose_nation_is_not_a_nation() {
        // The shape that produced the 9-year-olds and the 109-year-old: a
        // prefix and a date that pass, and a nation field of 1280.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Oldřich Dharmaraja Singgam"), 257, 1923);
        buf.extend_from_slice(&[0u8; 9]);
        buf.extend_from_slice(&1280u16.to_le_bytes());
        assert!(scan_people(&buf, &table()).is_empty());
    }

    #[test]
    fn keeps_a_record_whose_nation_field_never_arrives() {
        // Truncated at the end of the frame: no nation to judge, so the
        // record stands with `None` rather than being thrown away.
        let buf = record(100, 200, NO_COMMON_NAME, Some("Bukayo Ayoyinka Saka"), 248, 2001);
        let found = scan_people(&buf, &table());
        assert_eq!(found.len(), 1);
        assert_eq!(found.first().unwrap().nation_id, None);
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
        buf.extend(nation_field(170));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        let found = scan_people(&buf, &table());
        assert_eq!(found.len(), 2);
        assert_eq!(found.get(1).unwrap().full_name, "Virgil van Dijk");
    }

    /// A compact entry as it sits on disk: `10 00`, the two name ids, `01`,
    /// then the entity object header and the doubled-uid triple.
    fn compact_entry(first: u32, surname: u32, eid: u32, uid: u32, uid2: u32) -> Vec<u8> {
        compact_entry_typed(0x02, first, surname, eid, uid, uid2)
    }

    /// A compact entry with a chosen type byte — the byte that carries the
    /// gender flag on top of its 0-2 value.
    fn compact_entry_typed(
        type_byte: u8,
        first: u32,
        surname: u32,
        eid: u32,
        uid: u32,
        uid2: u32,
    ) -> Vec<u8> {
        let mut v = vec![0x10, 0x00];
        v.extend_from_slice(&first.to_le_bytes());
        v.extend_from_slice(&surname.to_le_bytes());
        v.extend_from_slice(&[0x01, type_byte, 0x40, 0x18, 0x04, 0x00, 0x00, 0x00]);
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid2.to_le_bytes());
        v
    }

    #[test]
    fn reads_a_compact_entry() {
        // Mbappé's real shape in a 2035 save: name ids and identity, no
        // record prefix, no date of birth, nothing else.
        let mut buf = vec![0u8; 4];
        buf.extend(compact_entry(100, 200, 22279, 85_139_014, 85_139_014));
        let found = scan_compact(&buf, &table(), 0);
        assert_eq!(found.len(), 1);
        let p = found.first().unwrap();
        assert_eq!(p.full_name, "Erling Haaland");
        assert_eq!(p.eid, Some(22279));
        assert_eq!(p.uid, Some(85_139_014));
        assert!(p.compact);
        assert_eq!(p.date_of_birth, None);
        assert_eq!(p.nation_id, None);
        assert_eq!(p.offset, 4);
    }

    #[test]
    fn a_compact_woman_parses_and_carries_her_gender() {
        // The type byte carries the gender flag: 0x12 is a type-2 object for
        // a woman. Requiring the raw byte to be 0-2 rejected every one of
        // these entries, which is why compact women were missing entirely.
        let mut buf = compact_entry_typed(0x12, 100, 200, 22279, 85_139_014, 85_139_014);
        buf.extend(compact_entry(101, 201, 22280, 85_139_015, 85_139_015));
        let found = scan_compact(&buf, &table(), 0);
        assert_eq!(found.len(), 2);
        assert_eq!(found.first().unwrap().female, Some(true));
        assert_eq!(found.get(1).unwrap().female, Some(false));
    }

    #[test]
    fn a_compact_type_byte_past_the_flag_bits_is_still_a_decoy() {
        let buf = compact_entry_typed(0x43, 100, 200, 22279, 85_139_014, 85_139_014);
        assert!(scan_compact(&buf, &table(), 0).is_empty());
    }

    #[test]
    fn rejects_compact_decoys() {
        let mut buf = Vec::new();
        // Name ids that resolve in no pool.
        buf.extend(compact_entry(999, 200, 22280, 7, 7));
        buf.extend(compact_entry(100, 999, 22281, 8, 8));
        // A uid that is not doubled, and the two reserved uids.
        buf.extend(compact_entry(100, 200, 22282, 9, 10));
        buf.extend(compact_entry(100, 200, 22283, 0, 0));
        buf.extend(compact_entry(100, 200, 22284, u32::MAX, u32::MAX));
        // An entity id out of range.
        buf.extend(compact_entry(100, 200, MAX_EID, 11, 11));
        assert!(scan_compact(&buf, &table(), 0).is_empty());
    }

    /// An identity as it sits on disk: the seven-byte entity object header —
    /// type byte, `0x40`, flags, then four zeros — and the triple after it.
    fn identity_block(eid: u32, uid: u32) -> Vec<u8> {
        identity_block_typed(0x00, eid, uid)
    }

    /// An identity block with a chosen type byte, for the gender flag it
    /// carries in bit 0x10.
    fn identity_block_typed(type_byte: u8, eid: u32, uid: u32) -> Vec<u8> {
        let mut v = vec![type_byte, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00];
        v.extend_from_slice(&eid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v.extend_from_slice(&uid.to_le_bytes());
        v
    }

    #[test]
    fn gender_reads_from_the_identity_header_type_byte() {
        // Sam Kerr's shape: type byte 0x10 — the female bit over type 0 —
        // against van Dijk's plain 0x00. The bit is FM's own record of
        // gender; squads never mix it on any save tested.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(identity_block(50, 9_000_001));
        buf.extend(nation_field(170));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(identity_block_typed(0x10, 51, 9_000_002));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);

        assert_eq!(people.first().unwrap().female, Some(false));
        assert_eq!(people.get(1).unwrap().female, Some(true));
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
    fn a_headerless_shadow_is_told_by_its_values() {
        // Verberne's shape: the identity block carries no object header, so
        // the header tell cannot flag the short read one byte before it.
        // Really eid 2057 / uid 601116; the shadow reads 526592 / 153885696.
        // Four zeros ahead of the triple, as on disk, so the short read one
        // byte early passes the zeros test too and the tie is real.
        let mut buf = record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000);
        buf.extend(nation_field(170));
        buf.extend_from_slice(&[0u8; 4]);
        buf.extend_from_slice(&2057u32.to_le_bytes());
        buf.extend_from_slice(&601_116u32.to_le_bytes());
        buf.extend_from_slice(&601_116u32.to_le_bytes());

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);

        let bound = people.first().unwrap();
        assert_eq!(bound.eid, Some(2057), "the value-shifted shadow must not win");
        assert_eq!(bound.uid, Some(601_116));
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
        buf.extend(nation_field(170));
        buf.extend(identity_block(40_000, 9_000_001));
        buf.extend(record(101, 201, NO_COMMON_NAME, None, 189, 1991));
        buf.extend(nation_field(170));
        buf.extend(identity_block(41_000, 9_000_002));
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(nation_field(170));
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
    /// No money list — the shape most lower-league contracts carry.
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

    /// The same block with the typed money list in front, as Berenguer's
    /// real bytes hold it: count 5, a bare first value, three typed bonus
    /// rows, then the type-0x26 clause row — `None` writes the unset
    /// `FF FF FF FF` sentinel Haaland's contract carries.
    fn contract_with_clause(
        eid: u32,
        wage: u32,
        expiry_doy: u16,
        expiry_year: u16,
        clause: Option<u32>,
    ) -> Vec<u8> {
        let mut v = vec![0u8; 8];
        v.extend_from_slice(&[0x00, 0x00, 0x00, 0x05]);
        v.extend_from_slice(&4304u32.to_le_bytes());
        v.extend_from_slice(&[0xFF, 0xFF]);
        for (ty, val) in [(0x20u16, 3228u32), (0x27, 3228), (0x22, 1076)] {
            v.extend_from_slice(&ty.to_le_bytes());
            v.extend_from_slice(&val.to_le_bytes());
            v.extend_from_slice(&[0xFF, 0xFF]);
        }
        v.extend_from_slice(&0x26u16.to_le_bytes());
        v.extend_from_slice(&clause.unwrap_or(u32::MAX).to_le_bytes());
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
    fn reads_the_release_clause_from_the_money_list() {
        // Berenguer's real shape: the typed list before the expiry run,
        // clause row last. The value is the game's own display-currency
        // conversion of the €60M La Liga default.
        let mut buf = contract_with_clause(50, 21_522, 181, 2027, Some(51_852_408));
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().release_clause, Some(51_852_408));
        assert_eq!(people.first().unwrap().wage, Some(21_522));
    }

    #[test]
    fn an_unset_clause_row_reads_none() {
        // Haaland's real shape: the row exists, the value is the FFFFFFFF
        // sentinel — a contract with no release clause, not a zero one.
        let mut buf = contract_with_clause(50, 450_000, 181, 2034, None);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().release_clause, None);
        assert_eq!(people.first().unwrap().wage, Some(450_000));
    }

    #[test]
    fn a_contract_without_a_money_list_reads_no_clause() {
        // The plain fixture has no list at all: undecoded, not clause-free,
        // and above all not a misread of neighbouring bytes.
        let mut buf = contract(50, 450_000, 181, 2034);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        assert_eq!(people.first().unwrap().release_clause, None);
    }

    #[test]
    fn a_decoy_row_between_the_contract_and_the_record_does_not_eat_it() {
        // Jae-Wan Choi's shape in a 2035 save: the contract sits 337 bytes
        // back, and a later eid-anchored row with a non-contract tail
        // (international duty) sits between it and the record prefix. The
        // old hunt took only the *last* occurrence, failed its shape test,
        // and reported a contracted Rangers player as a free agent.
        let mut buf = contract(50, 10_582, 181, 2034);
        buf.extend(vec![0u8; 200]);
        // The decoy: same eid, zeroes where the contract has them, but a
        // 01-00-00-01 tail instead of 01-xx-00-FFFFFFFF.
        buf.extend_from_slice(&50u32.to_le_bytes());
        buf.extend_from_slice(&2306u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 4]);
        buf.extend_from_slice(&3805u32.to_le_bytes());
        buf.extend_from_slice(&[0x01, 0x00, 0x00, 0x01]);
        buf.extend_from_slice(&[0u8; 8]);
        buf.extend(record(100, 200, NO_COMMON_NAME, Some("Erling Braut Haaland"), 203, 2000));
        buf.extend(identity_block(50, 9_000_001));

        let mut people = scan_people(&buf, &table());
        bind_identities(&buf, &mut people, 0);
        bind_contracts(&buf, &mut people);

        let p = people.first().unwrap();
        assert_eq!(p.wage, Some(10_582), "the earlier, true contract row must win");
        assert_eq!(p.contract_until.map(|d| (d.year, d.month, d.day)), Some((2034, 6, 30)));
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
