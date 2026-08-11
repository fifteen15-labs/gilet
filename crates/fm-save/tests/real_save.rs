//! Squad-table checks that need the raw parse rather than the command layer:
//! captains and entity ids are not part of what the UI consumes yet.
//!
//! Skips rather than fails when no save is present, like the journey tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

fn read_named(name: &str) -> Option<Vec<u8>> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home)
        .join("Library/Application Support/Sports Interactive/Football Manager 26/games")
        .join(name);
    std::fs::read(path).ok()
}

fn load_named(name: &str) -> Option<fm_save::Save> {
    Some(fm_save::Save::parse(&read_named(name)?).unwrap())
}

fn load() -> Option<fm_save::Save> {
    load_named("Career.fm")
}

/// The decoded value of a named attribute.
fn value_of(ability: &fm_save::Ability, name: &str) -> u8 {
    let index = (0..fm_save::ability::ATTRIBUTE_COUNT)
        .find(|&i| fm_save::ability::attribute_name(i) == Some(name))
        .unwrap_or_else(|| panic!("{name} is not a named attribute"));
    ability.attributes[index]
}

/// Asserts a whole in-game report screen against a decoded block.
fn assert_attributes(ability: &fm_save::Ability, shown: &[(&str, u8)]) {
    for (name, expected) in shown {
        assert_eq!(value_of(ability, name), *expected, "{name}");
    }
}

macro_rules! save_or_skip {
    () => {
        match load() {
            Some(s) => s,
            None => {
                eprintln!("skipped: no FM26 save on this machine");
                return;
            }
        }
    };
}

#[test]
fn squad_records_name_the_captains() {
    let save = save_or_skip!();

    let club_eid = |name: &str| -> u32 {
        save.clubs
            .iter()
            .find(|c| c.name == name)
            .and_then(|c| c.eid)
            .unwrap_or_else(|| panic!("{name} should have an entity id"))
    };
    let squad_of = |name: &str| -> &fm_save::Squad {
        let eid = club_eid(name);
        save.squads
            .iter()
            .find(|s| s.club_eid == eid)
            .unwrap_or_else(|| panic!("{name} should have a squad record"))
    };
    let person_name = |eid: Option<u32>| -> String {
        save.people
            .iter()
            .find(|p| p.eid == eid)
            .map(|p| p.full_name.clone())
            .unwrap_or_default()
    };

    // The reference save's real captaincies at its in-game date, October 2025.
    let city = squad_of("Manchester City");
    assert_eq!(person_name(city.captain_eid), "Bernardo Mota Veiga de Carvalho e Silva");
    assert_eq!(person_name(city.vice_captain_eid), "Rúben Santos Gato Alves Dias");

    let liverpool = squad_of("Liverpool");
    assert_eq!(person_name(liverpool.captain_eid), "Virgil van Dijk");

    let arsenal = squad_of("Arsenal");
    assert_eq!(person_name(arsenal.captain_eid), "Martin Ødegaard");

    // Squad sizes across the table are squad-shaped.
    assert!(save.squads.len() > 1_000, "got {} squads", save.squads.len());
    assert!(save.squads.iter().all(|s| s.player_eids.len() <= 60));
}

#[test]
fn known_entity_ids_read_back_exactly() {
    let save = save_or_skip!();

    // Values confirmed by hand against the reference save's bytes.
    let by_name = |name: &str| save.clubs.iter().find(|c| c.name == name).unwrap();
    assert_eq!(by_name("Manchester City").eid, Some(369));
    assert_eq!(by_name("Manchester City").uid, Some(678));
    assert_eq!(by_name("Arsenal").eid, Some(293));
    assert_eq!(by_name("Liverpool").eid, Some(366));

    let haaland = save
        .people
        .iter()
        .find(|p| p.full_name == "Erling Braut Haaland")
        .unwrap();
    assert_eq!(haaland.eid, Some(10_241));
    assert_eq!(haaland.uid, Some(29_179_241));
}

#[test]
fn contracts_match_public_figures() {
    let save = save_or_skip!();
    let person = |name: &str| save.people.iter().find(|p| p.full_name == name).unwrap();

    // FM Scout lists Haaland at £450K a week to 30 June 2034 in FM26.
    let haaland = person("Erling Braut Haaland");
    assert_eq!(haaland.wage, Some(450_000));
    let until = haaland.contract_until.unwrap();
    assert_eq!((until.year, until.month, until.day), (2034, 6, 30));

    // Salah and van Dijk both signed to 2027.
    assert_eq!(person("Mohamed Salah Ghaly").contract_until.map(|d| d.year), Some(2027));
    assert_eq!(person("Virgil van Dijk").contract_until.map(|d| d.year), Some(2027));

    // Foreign contracts convert to the display currency, so Madrid wages are
    // not round numbers — the signature of a real read, not a guess.
    let mbappe = person("Kylian Mbappé Lottin");
    let wage = mbappe.wage.unwrap();
    assert!((300_000..700_000).contains(&wage), "Mbappé wage {wage}");
    assert!(wage % 1000 != 0, "a converted wage should not be round");

    // Wages across the database form a pyramid: semi-pro at the median,
    // thousands in the professional tail.
    let mut wages: Vec<u32> = save.people.iter().filter_map(|p| p.wage).collect();
    wages.sort_unstable();
    assert!(wages.len() > 15_000, "expected contracts, got {}", wages.len());
    let p50 = wages[wages.len() / 2];
    let p90 = wages[wages.len() * 9 / 10];
    assert!(p50 < 5_000, "median weekly wage should be modest, got {p50}");
    assert!(p90 > 5_000, "the top decile should be professional, got {p90}");
}

/// An aged save is the hard case: a decade of training moves attribute
/// internals off the multiples of five, and transfers shuffle every squad
/// list out of entity-id order. The values asserted here are read straight
/// from the in-game player report for Jamal Musiala on 28 May 2035.
#[test]
fn an_aged_save_decodes_against_the_in_game_report() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };

    let musiala = save
        .people
        .iter()
        .find(|p| p.full_name == "Jamal Musiala")
        .expect("Musiala should be in the database");
    let ability = musiala.ability.as_ref().expect("Musiala is a player");

    assert_eq!((ability.current, ability.potential), (180, 185));

    // The club link survives the shuffled squad lists.
    let liverpool = save
        .clubs
        .iter()
        .find(|c| c.name == "Liverpool")
        .and_then(|c| c.eid)
        .expect("Liverpool should have an entity id");
    assert_eq!(musiala.club_eid, Some(liverpool));

    // His whole report screen, attribute by attribute.
    let shown = [
        ("Crossing", 13), ("Dribbling", 18), ("Finishing", 16), ("First Touch", 17),
        ("Heading", 13), ("Long Shots", 14), ("Marking", 12), ("Passing", 17),
        ("Tackling", 11), ("Technique", 18), ("Corners", 10), ("Free Kick Taking", 11),
        ("Long Throws", 5), ("Penalty Taking", 14), ("Aggression", 12), ("Anticipation", 16),
        ("Bravery", 13), ("Composure", 18), ("Concentration", 13), ("Decisions", 16),
        ("Determination", 16), ("Flair", 19), ("Leadership", 9), ("Off the Ball", 16),
        ("Positioning", 11), ("Teamwork", 14), ("Vision", 17), ("Work Rate", 15),
        ("Acceleration", 14), ("Agility", 18), ("Balance", 16), ("Jumping Reach", 12),
        ("Natural Fitness", 14), ("Pace", 15), ("Stamina", 14), ("Strength", 12),
    ];
    assert_attributes(ability, &shown);

    // Right-footed, and the game shows his left as "Fairly Strong".
    assert_eq!(value_of(ability, "Right Foot"), 20);
    assert_eq!(value_of(ability, "Left Foot"), 12);

    // His position ratings: natural AM(C), accomplished across the AM strip.
    assert_eq!(ability.natural_positions().first().copied(), Some("AMC"));

    // His header strip: a wage inside the scouted £350K-£425K band, contract
    // to 30 June 2037.
    assert_eq!(musiala.wage, Some(392_499));
    let until = musiala.contract_until.expect("contract expiry");
    assert_eq!((until.year, until.month, until.day), (2037, 6, 30));
}

/// People who leave the loaded game world are folded down to compact entries
/// — a name reference and an identity, no record prefix — and an aged save
/// accumulates hundreds of them. Kylian Mbappé is stored that way in the 2035
/// save: before the compact scan he was simply missing from the parse, while
/// day-one saves hold his full record (uid 85139014 in both, FM's public
/// database id for him).
#[test]
fn an_aged_save_keeps_its_compacted_people() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };

    let mbappe = save
        .people
        .iter()
        .find(|p| p.uid == Some(85_139_014))
        .expect("Kylian Mbappé should parse from his compact entry");
    assert_eq!(mbappe.full_name, "Kylian Mbappé");
    assert!(mbappe.compact);
    // A compact entry carries nothing beyond name and identity, and the
    // fields must say so rather than carry a neighbour's values.
    assert_eq!(mbappe.date_of_birth, None);
    assert_eq!(mbappe.nation_id, None);
    assert_eq!(mbappe.wage, None);
    assert!(mbappe.ability.is_none());

    // The save holds hundreds of compacted people, and none of them may
    // shadow an eid a full record already claims.
    let compact_count = save.people.iter().filter(|p| p.compact).count();
    assert!(
        compact_count > 900,
        "expected the 2035 save's compact population, found {compact_count}"
    );
    let mut eids: Vec<u32> = save.people.iter().filter_map(|p| p.eid).collect();
    let total = eids.len();
    eids.sort_unstable();
    eids.dedup();
    assert_eq!(eids.len(), total, "an eid is bound to two people");
}

/// The goalkeeping attributes are invisible on an outfielder's report, so a
/// keeper is the only way to check them. Values from Lucas Chevalier's
/// in-game report in the same 2035 save.
#[test]
fn a_goalkeepers_report_confirms_the_goalkeeping_set() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };

    let keeper = save
        .people
        .iter()
        .find(|p| p.full_name == "Lucas Chevalier")
        .expect("Chevalier should be in the database");
    let ability = keeper.ability.as_ref().expect("Chevalier is a player");
    assert_eq!(ability.natural_positions(), vec!["GK"]);

    assert_attributes(
        ability,
        &[
            // The goalkeeping set his screen shows in place of the technical one.
            ("Command of Area", 14),
            ("Kicking", 13),
            ("One on Ones", 18),
            ("Throwing", 16),
            ("Eccentricity", 14),
            ("Rushing Out Tendency", 15),
            ("Punching Tendency", 5),
            // Shared attributes, which must read the same for a keeper.
            ("First Touch", 14),
            ("Passing", 15),
            ("Free Kick Taking", 4),
            ("Penalty Taking", 5),
            ("Technique", 13),
            ("Anticipation", 16),
            ("Concentration", 13),
            ("Determination", 16),
            ("Leadership", 15),
            ("Positioning", 14),
            ("Jumping Reach", 15),
            ("Acceleration", 11),
            ("Pace", 12),
            ("Agility", 15),
            ("Strength", 12),
        ],
    );

    // His screen shows Aerial Reach, Communication, Handling and Reflexes
    // all at 15 — which is why this report alone could not separate them.
    // The split came from Donnarumma's published block instead (see
    // `published_keeper_attributes_split_the_last_four`); this report still
    // pins all four values.
    for name in ["Handling", "Aerial Reach", "Communication", "Reflexes"] {
        assert_eq!(value_of(ability, name), 15, "{name}");
    }

    // Outfielders score near nothing on the goalkeeping set — the property
    // the whole grouping was found by.
    let musiala = save
        .people
        .iter()
        .find(|p| p.full_name == "Jamal Musiala")
        .and_then(|p| p.ability.as_ref())
        .expect("Musiala");
    for index in fm_save::ability::GOALKEEPING_INDICES {
        assert!(musiala.attributes[index] <= 4, "outfielder at goalkeeping index {index}");
        assert!(ability.attributes[index] >= 5, "keeper at goalkeeping index {index}");
    }
}

/// FM 26.2.0 moved the header date; the main frame's week stamp stands in.
/// Without it, ages on an aged save compute against the real-world clock and
/// come out years wrong.
#[test]
fn an_aged_save_reads_its_own_date() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };
    let date = save.game_date.expect("26.2.0 save should read a date");
    // The stamp tracks the weekly rollover, so it sits within days of the
    // true 28 May 2035.
    assert_eq!(date.year, 2035);
    assert_eq!(date.month, 5);
}

/// The Afan Lido career regressed two ways at once: its week stamp carries
/// junk in the high bits (0x1A9F, not a day of year until masked), and the
/// men's Tottenham club record reads flags 0x12, which dropped the club and
/// left its whole squad showing no club.
#[test]
fn a_262_career_reads_its_date_and_flagged_clubs() {
    let Some(save) = load_named("Paul Dolden - Afan Lido.fm") else {
        eprintln!("skipped: no Afan Lido save on this machine");
        return;
    };

    // Day 159 of 2026, confirmed by the current-date stamps repeated through
    // the save's competition frames.
    let date = save.game_date.expect("masked week stamp should read");
    assert_eq!(date, fm_save::Date { year: 2026, month: 6, day: 8 });

    // The men's Tottenham record (eid 418) must exist alongside the women's
    // (eid 15924), and its squad must resolve — Pape Matar Sarr was one of
    // the contracted players showing no club.
    let spurs: Vec<_> = save
        .clubs
        .iter()
        .filter(|c| c.name == "Tottenham Hotspur")
        .collect();
    assert!(spurs.iter().any(|c| c.eid == Some(418)), "men's Spurs record missing");
    let squad = save
        .squads
        .iter()
        .find(|s| s.club_eid == 418)
        .expect("men's Spurs squad missing");
    let sarr = save
        .people
        .iter()
        .find(|p| p.full_name == "Pape Matar Sarr")
        .expect("Sarr missing");
    assert!(squad.player_eids.contains(&sarr.eid.unwrap()));
    assert_eq!(sarr.club_eid, Some(418));

    // Chelsea also reads flags 0x12 in this save; it must be present.
    assert!(save.clubs.iter().any(|c| c.name == "Chelsea" && c.eid.is_some()));
}

/// The two bytes read as a fixed `FF FF` signature before the club name are
/// per-club, exactly as the flags byte in front of them turned out to be.
/// Heybridge Swifts — the club this save's career is played with — reads
/// `10 FF 00`, so the club was dropped along with its whole squad, and the
/// user's own team could not be found at all. 592 head-validated clubs in this
/// save sit behind a pair that is not `FF FF`; Newport County, Birmingham
/// City, Blackburn Rovers and Bolton Wanderers are among them.
#[test]
fn a_club_whose_tail_pair_is_not_ffff_keeps_its_squad() {
    let Some(save) = load_named("Heybridge Swifts.fm") else {
        eprintln!("skipped: no Heybridge Swifts.fm on this machine");
        return;
    };

    let swifts = save
        .clubs
        .iter()
        .find(|c| c.name == "Heybridge Swifts")
        .expect("Heybridge Swifts missing from the club table");
    assert_eq!(swifts.eid, Some(5404));
    assert_eq!(swifts.uid, Some(5_100_159));
    assert_eq!(swifts.nation_id, 139);

    let squad = save
        .squads
        .iter()
        .find(|s| s.club_eid == 5404)
        .expect("Heybridge Swifts field a squad");
    assert_eq!(squad.player_eids.len(), 25);

    let crook = save
        .people
        .iter()
        .find(|p| p.full_name == "Billy Crook")
        .expect("Billy Crook missing");
    assert!(squad.player_eids.contains(&crook.eid.unwrap()));
    assert_eq!(crook.club_eid, Some(5404));

    // The clubs the same anchor dropped in the divisions above them.
    for name in ["Newport County", "Birmingham City", "Blackburn Rovers", "Bolton Wanderers"] {
        assert!(
            save.clubs.iter().any(|c| c.name == name && c.eid.is_some()),
            "{name} should carry an entity id"
        );
    }
}

/// A club playing in one nation's pyramid from a ground in another kept no
/// entity id, because the club head's third u32 was read as a repeat of the
/// nation and required to match. It is not a repeat — it is where the club
/// sits, and the two differ for every cross-border club in the database.
///
/// The New Saints play in Wales (175) from Oswestry, England (139). Without
/// the entity id no squad record could reference them, so their entire first
/// team showed no club. The same head shape covers Cardiff, Swansea, Wrexham
/// and Newport County in the English pyramid, Derry City in the Irish one and
/// Berwick Rangers in the Scottish one.
#[test]
fn a_cross_border_club_resolves_its_squad() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let tns: Vec<_> = save.clubs.iter().filter(|c| c.name == "The New Saints").collect();
    assert!(!tns.is_empty(), "The New Saints missing entirely");
    assert!(
        tns.iter().all(|c| c.eid.is_some()),
        "a cross-border club must still carry an entity id"
    );
    // The club's own nation stays the pyramid it plays in, not where it sits.
    assert!(tns.iter().all(|c| c.nation_id == 175), "TNS should read as Welsh");

    // A day-one squad member rather than one signed later in a career, so the
    // anchor holds against the static save.
    let keeper = save
        .people
        .iter()
        .find(|p| p.full_name == "Nathan Shepperd")
        .expect("Nathan Shepperd missing from the people table");
    let club_eid = keeper.club_eid.expect("Shepperd should be linked to a club");
    assert!(
        tns.iter().any(|c| c.eid == Some(club_eid)),
        "Shepperd should be linked to The New Saints, got club eid {club_eid}"
    );

    // The men's squad resolves whole — the failure being pinned here was every
    // member showing no club, not a few going missing.
    let squad = save
        .squads
        .iter()
        .find(|s| s.club_eid == 1220)
        .expect("men's TNS squad missing");
    assert!(squad.player_eids.len() >= 20, "squad list unexpectedly short");
    let resolved = squad
        .player_eids
        .iter()
        .filter(|eid| save.people.iter().any(|p| p.eid == Some(**eid)))
        .count();
    assert_eq!(resolved, squad.player_eids.len(), "every member should resolve to a person");
}

/// The common-name pool holds FM's display names — "Raúl", not "Raúl
/// González Blanco" — and a person referencing one displays it. The pool
/// also carries the surname-first orderings FM uses for East Asian players.
#[test]
fn common_names_resolve_to_display_names() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let raul = save
        .people
        .iter()
        .find(|p| p.full_name == "Raúl González Blanco")
        .expect("Raúl missing from the people table");
    assert_eq!(raul.common_name.as_deref(), Some("Raúl"));
    assert_eq!(raul.display_name(), "Raúl");

    // Population sanity: the reference is common enough that resolution
    // failing broadly would say the pool sectioning broke.
    let with_id = save.people.iter().filter(|p| p.common_name_id.is_some()).count();
    let resolved = save.people.iter().filter(|p| p.common_name.is_some()).count();
    assert!(with_id >= 1_000, "expected a real population, got {with_id}");
    assert!(
        resolved * 10 >= with_id * 9,
        "common names should mostly resolve: {resolved} of {with_id}"
    );
}

/// The representative rows the club walk refuses are the international
/// signal: a person a national side's list names is in that setup. Day One
/// anchors: Saka, Haaland and van Dijk are all in their nations' selections;
/// Ethan Mbappé — a real player, uncapped — is in none.
#[test]
fn national_squad_lists_mark_their_members() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let marked = |name: &str| {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .unwrap_or_else(|| panic!("{name} missing from the people table"))
            .in_national_squad
    };
    assert!(marked("Bukayo Ayoyinka Saka"), "Saka is in England's squad");
    assert!(marked("Erling Braut Haaland"), "Haaland is in Norway's");
    assert!(marked("Virgil van Dijk"), "van Dijk is in the Netherlands'");
    assert!(!marked("Ethan Mbappé Lottin"), "Ethan Mbappé is uncapped");

    let total = save.people.iter().filter(|p| p.in_national_squad).count();
    assert!(
        (5_000..30_000).contains(&total),
        "the mark should cover the international population, got {total}"
    );
}

/// The boardroom run after the club's short name binds the director of
/// football seat and the board (`SAVE_FORMAT.md` §4). Day One anchors are
/// the real-world people: Richard Hughes as Liverpool's sporting director
/// with the FSG owners on the board, Hugo Viana at Manchester City with
/// Sheikh Mansour among his board.
#[test]
fn the_boardroom_binds_its_real_people() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let seat_of = |name: &str| {
        let p = save
            .people
            .iter()
            .find(|p| p.full_name == name)
            .unwrap_or_else(|| panic!("{name} missing from the people table"));
        (
            p.staff_role.unwrap_or_else(|| panic!("{name} carries no role")),
            p.club_eid.unwrap_or_else(|| panic!("{name} carries no club")),
        )
    };

    let (role, club) = seat_of("Richard Hughes");
    assert_eq!(role, fm_save::backroom::Role::DirectorOfFootball);
    assert!(
        save.clubs.iter().any(|c| c.eid == Some(club) && c.name == "Liverpool"),
        "Hughes should sit at Liverpool, got club eid {club}"
    );

    let (role, club) = seat_of("Hugo Miguel Ferreira Gomes Viana");
    assert_eq!(role, fm_save::backroom::Role::DirectorOfFootball);
    let city = save
        .clubs
        .iter()
        .find(|c| c.eid == Some(club))
        .expect("Viana's club should parse");
    assert_eq!(city.name, "Manchester City");

    let (role, club) = seat_of("Mansour bin Zayed Al Nahyan");
    assert_eq!(role, fm_save::backroom::Role::Board);
    assert_eq!(Some(club), city.eid, "Mansour sits on City's board");

    // Population sanity: the exact shape covers a fleet of clubs, not a
    // couple of lucky hits.
    let dofs = save
        .people
        .iter()
        .filter(|p| p.staff_role == Some(fm_save::backroom::Role::DirectorOfFootball))
        .count();
    let board = save
        .people
        .iter()
        .filter(|p| p.staff_role == Some(fm_save::backroom::Role::Board))
        .count();
    assert!(dofs >= 50, "expected a fleet of DoFs, got {dofs}");
    assert!(board >= 300, "expected hundreds of board members, got {board}");
}

/// The squad table's 0x13/0x15-typed rows are a club's B and youth squads,
/// keyed by the club's eid with the team entity's own uid — the rows the
/// uid-validated first-team walk can never claim. They bind the players
/// outside the loaded leagues, who otherwise show no club at all.
///
/// Day-one anchor: Jay Spearing, in Liverpool's youth setup in the FM26
/// database, appears in no first-team list — only a team squad links him.
#[test]
fn team_squads_bind_players_outside_first_team_lists() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    assert!(
        save.team_squads.len() >= 400,
        "expected hundreds of B/youth squads, got {}",
        save.team_squads.len()
    );

    // Every team squad keys a real club, outside the nation-entity range.
    let club_eids: std::collections::HashSet<u32> =
        save.clubs.iter().filter_map(|c| c.eid).collect();
    assert!(save.team_squads.iter().all(|s| club_eids.contains(&s.club_eid)));
    assert!(save.team_squads.iter().all(|s| s.club_eid > 260));

    let spearing = save
        .people
        .iter()
        .find(|p| p.full_name == "Jay Francis Spearing")
        .expect("Spearing missing from the people table");
    let club = spearing.club_eid.expect("Spearing should be linked via a team squad");
    let liverpool = save
        .clubs
        .iter()
        .find(|c| c.eid == Some(club))
        .map(|c| c.short_name.as_str());
    assert_eq!(liverpool, Some("Liverpool"));
    let in_first_team = save
        .squads
        .iter()
        .any(|s| s.player_eids.contains(&spearing.eid.unwrap()));
    assert!(!in_first_team, "the anchor only means something off the first-team lists");
}

/// The contract block names the employer: its second u32 is the employing
/// team's squad-table row ordinal plus one, and the row exists — empty —
/// even for a club the loaded leagues never materialise. Depay therefore
/// resolves to COR (eid 128, the unlicensed Corinthians) on a day-one save
/// where his squad row holds nobody, and Jorginho to FLA — the case the
/// out-of-league work left open (`OPEN_PROBLEMS.md` §1).
///
/// The same team id is the cross-check that unmasks a national side wearing
/// a club's entity pair. Argentina's roster carried A.E.C. Manlleu's eid
/// *and* uid, so the uid-validated first-team walk accepted it and put
/// Messi at a Catalan sixth-tier club; every member's contract pointing
/// elsewhere is what refuses the row, after which Messi's real Inter Miami
/// row is the only claim left.
#[test]
fn contracts_name_the_employer_and_unmask_represented_squads() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let person = |needle: &str| -> &fm_save::Person {
        save.people
            .iter()
            .find(|p| p.full_name.contains(needle))
            .unwrap_or_else(|| panic!("{needle} is not in this save"))
    };
    let club_of = |p: &fm_save::Person| -> Option<&str> {
        p.club_eid
            .and_then(|e| save.clubs.iter().find(|c| c.eid == Some(e)))
            .map(|c| c.short_name.as_str())
    };

    // Employer through the contract alone — the squad rows are empty.
    assert_eq!(club_of(person("Memphis Depay")), Some("COR"));
    assert_eq!(club_of(person("Jorge Luiz Frello")), Some("FLA"));
    // Atlético Mineiro's row hides behind an 0xFF-typed separator and a
    // one-byte shadow hit; both must stay defeated for Hulk to resolve.
    assert_eq!(club_of(person("Givanildo Vieira de Sousa")), Some("ATM"));

    // Whole clubs that used to vanish: a name starting with a digit failed
    // the capital-letter test and took all of Mainz with it, and Vancouver's
    // head reads location == own nation under a foreign pyramid, which the
    // two-copies rule rejected along with every Canadian MLS club.
    assert_eq!(club_of(person("Nadiem Amiri")), Some("1. FSV Mainz 05"));
    assert_eq!(club_of(person("Thomas Müller")), Some("Vancouver"));

    // The represented-squad veto: Messi at his club, not at Manlleu, and
    // his international team-mates likewise.
    assert_eq!(club_of(person("Lionel Andrés Messi")), Some("Inter Miami"));
    assert_eq!(
        club_of(person("Emiliano Damián Martínez")),
        Some("Aston Villa")
    );
    let manlleu = save
        .clubs
        .iter()
        .find(|c| c.short_name == "Manlleu")
        .and_then(|c| c.eid)
        .expect("Manlleu parses");
    assert!(
        !save.squads.iter().any(|s| s.club_eid == manlleu),
        "the Argentina roster wearing Manlleu's entity pair must be refused"
    );
}

/// The index case for the team-squad work: an aged career whose newly
/// promoted B-team players FM's own search shows at their club while every
/// list Gilet read left them clubless — and, through the free-agent filter's
/// "no contract and no club" test, misfiled as free agents. TJ.fm is a live
/// career, so the assertions stay structural: the player reads as *someone*,
/// with a club and a wage.
#[test]
fn a_b_team_player_carries_his_club_and_wage() {
    let Some(save) = load_named("TJ.fm") else {
        eprintln!("skipped: no TJ.fm on this machine");
        return;
    };

    let choi = save
        .people
        .iter()
        .find(|p| p.full_name == "Jae-Wan Choi")
        .expect("Choi missing from the people table");
    assert!(choi.club_eid.is_some(), "a B-team player must carry his club");
    assert!(choi.wage.is_some(), "his contract row sits behind a duty row and must still bind");

    // The senior out-of-league rows only materialise on aged saves, so this
    // career is also where their acceptance is pinned.
    assert!(
        save.team_squads
            .iter()
            .any(|s| s.kind == fm_save::squad::SquadKind::OutOfLeague),
        "an aged save should carry out-of-league senior squads"
    );
}

/// The last four goalkeeping indices were separated by published FM 26 data
/// rather than an in-game screen: fminside.net serves the 26.2 database with
/// display×5 values (the same source class as the FM Scout wage check), and
/// Donnarumma's page splits all four — Handling 80, Aerial Reach 75,
/// Communication 70, Reflexes 90 — where the one in-game keeper report read
/// 15 across the board. His decoded block matches that split (16/15/14/18),
/// and Alisson's is consistent with his page (85/70/70/85 → 17/14/14/17).
#[test]
fn published_keeper_attributes_split_the_last_four() {
    let save = save_or_skip!();

    let block = |name: &str| -> &fm_save::Ability {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .and_then(|p| p.ability.as_ref())
            .unwrap_or_else(|| panic!("{name} should be a player"))
    };

    let donnarumma = block("Gianluigi Donnarumma");
    assert_eq!(value_of(donnarumma, "Handling"), 16);
    assert_eq!(value_of(donnarumma, "Aerial Reach"), 15);
    assert_eq!(value_of(donnarumma, "Communication"), 14);
    assert_eq!(value_of(donnarumma, "Reflexes"), 18);

    let alisson = block("Alisson Ramsés Becker");
    assert_eq!(value_of(alisson, "Handling"), 17);
    assert_eq!(value_of(alisson, "Aerial Reach"), 14);
    assert_eq!(value_of(alisson, "Communication"), 14);
    assert_eq!(value_of(alisson, "Reflexes"), 17);
}

/// The eight-byte run after the nation marker is the hidden personality set.
/// Slot names verified where a screen shows the value or the personality
/// label pins it: Adaptability is visible on staff reports, and the
/// Model Professional / Model Citizen labels constrain Professionalism.
#[test]
fn hidden_personality_matches_the_in_game_labels() {
    let Some(save) = load_named("Ongoing.fm") else {
        eprintln!("skipped: no Ongoing.fm on this machine");
        return;
    };
    let person = |name: &str| {
        save.people
            .iter()
            .find(|p| p.full_name.contains(name))
            .unwrap_or_else(|| panic!("{name} missing"))
    };

    // Adaptability is visible on staff screens: Leckie Elite, Ottley
    // Outstanding, Emery Good.
    assert_eq!(person("Isaac Leckie").adaptability(), Some(20));
    assert_eq!(person("Reece Thomas Ottley").adaptability(), Some(19));
    assert_eq!(person("Unai Emery").adaptability(), Some(13));

    // Emery is a Model Professional; Musiala a Model Citizen.
    assert_eq!(person("Unai Emery").professionalism(), Some(20));
    assert_eq!(person("Jamal Musiala").professionalism(), Some(16));
    assert_eq!(person("Jamal Musiala").personality, Some([18, 19, 15, 14, 16, 16, 18, 4]));

    // The run parses for nearly every adult. The misses are the simulated
    // children an aged save accumulates (born in-game, a different record
    // layout, junk nation ids) plus human-manager avatars.
    let adults: Vec<_> = save
        .people
        .iter()
        .filter(|p| {
            p.nation_id.is_some_and(|n| n <= 250)
                && p.date_of_birth.is_some_and(|d| d.year < 2020)
        })
        .collect();
    let with = adults.iter().filter(|p| p.personality.is_some()).count();
    assert!(
        with * 10 > adults.len() * 9,
        "only {with} of {} adults have personality",
        adults.len()
    );
}

/// The probe save was made after creating shortlists whose contents are
/// known exactly — the ground truth that unlocked `scout_man.dat`.
#[test]
fn probe_save_reads_the_in_game_shortlists() {
    let Some(save) = load_named("Probe.fm") else {
        eprintln!("skipped: no Probe.fm on this machine");
        return;
    };

    let names_in = |list: &fm_save::GameShortlist| -> Vec<String> {
        list.person_eids
            .iter()
            .map(|&eid| {
                save.people
                    .iter()
                    .find(|p| p.eid == Some(eid))
                    .unwrap_or_else(|| panic!("shortlist eid {eid} resolves to nobody"))
                    .full_name
                    .clone()
            })
            .collect()
    };

    let list = |name: &str| -> &fm_save::GameShortlist {
        save.shortlists
            .iter()
            .find(|s| s.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no shortlist named {name}"))
    };

    // Created in FM as ZZPROBE with exactly these three, in this order.
    assert_eq!(
        names_in(list("ZZPROBE")),
        vec!["Virgil van Dijk", "Florian Richard Wirtz", "Mohamed Salah Ghaly"]
    );
    // WirtzNew was imported back into the career from its .fmf export.
    assert_eq!(
        names_in(list("WirtzNew")),
        vec!["Florian Richard Wirtz", "Roberto Firmino Barbosa de Oliveira", "Mohamed Salah Ghaly"]
    );
    // The career's unnamed default list exists and resolves too.
    let unnamed = save.shortlists.iter().find(|s| s.name.is_none());
    assert!(unnamed.is_some(), "the unnamed default shortlist should parse");
}

/// The writer's safety proof, against the real probe save: taking the file
/// apart and reassembling it unchanged must reproduce it byte for byte, and
/// an in-memory shortlist edit must survive a full reparse. No file on disk
/// is touched.
#[test]
fn probe_save_survives_reassembly_and_a_shortlist_edit() {
    let Some(bytes) = ({
        std::env::var_os("HOME").and_then(|home| {
            std::fs::read(
                std::path::PathBuf::from(home)
                    .join("Library/Application Support/Sports Interactive/Football Manager 26/games")
                    .join("Probe.fm"),
            )
            .ok()
        })
    }) else {
        eprintln!("skipped: no Probe.fm on this machine");
        return;
    };

    // Identity: decompose + assemble with nothing changed is the same file.
    let d = fm_save::archive::decompose(&bytes).unwrap();
    let rebuilt =
        fm_save::archive::assemble(d.header, d.body, d.inner_header, d.manifest_frame).unwrap();
    assert_eq!(rebuilt, bytes, "identity reassembly must be byte-identical");

    // Edit: put Haaland on ZZPROBE, rebuild, reparse the whole save.
    let save = fm_save::Save::parse(&bytes).unwrap();
    let haaland = save
        .people
        .iter()
        .find(|p| p.full_name == "Erling Braut Haaland")
        .and_then(|p| p.eid)
        .expect("Haaland should have an entity id");

    let scout = fm_save::archive::member_plaintext(&bytes, "scout_man.dat").unwrap();
    // Date-added bytes observed in this save's own entries — the low half of
    // the field is undecoded, so nothing is invented here.
    let date = [0x9F, 0x1A, 0xEA, 0x07];
    let edited = fm_save::shortlist::add_entry(&scout, Some("ZZPROBE"), haaland, date).unwrap();
    let written = fm_save::archive::replace_member(&bytes, "scout_man.dat", &edited).unwrap();

    let reparsed = fm_save::Save::parse(&written).unwrap();
    let zzprobe = reparsed
        .shortlists
        .iter()
        .find(|s| s.name.as_deref() == Some("ZZPROBE"))
        .expect("ZZPROBE should still parse after the rewrite");
    assert_eq!(zzprobe.person_eids.len(), 4);
    assert_eq!(zzprobe.person_eids.last().copied(), Some(haaland));

    // The rest of the save is untouched by the edit: same people, same
    // captain, same other lists.
    assert_eq!(reparsed.people.len(), save.people.len());
    let wirtznew = |s: &fm_save::Save| {
        s.shortlists
            .iter()
            .find(|l| l.name.as_deref() == Some("WirtzNew"))
            .map(|l| l.person_eids.clone())
    };
    assert_eq!(wirtznew(&reparsed), wirtznew(&save));
}

/// The roster table's manager slot names each club's manager: on day one
/// Slot manages Liverpool, Arteta Arsenal, Guardiola Manchester City — and
/// the coverage is the loaded world's managed clubs (1,646 filled slots on
/// this save), not a handful of big names.
#[test]
fn managers_bind_to_their_clubs() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipping: no Day One.fm");
        return;
    };

    let club_eid = |name: &str| -> u32 {
        save.clubs
            .iter()
            .find(|c| c.name == name)
            .and_then(|c| c.eid)
            .unwrap_or_else(|| panic!("{name} should have an entity id"))
    };
    let employer = |name: &str| -> Option<u32> {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .unwrap_or_else(|| panic!("{name} is not in the save"))
            .club_eid
    };
    assert_eq!(employer("Arend Martijn Slot"), Some(club_eid("Liverpool")));
    assert_eq!(employer("Mikel Arteta Amatriain"), Some(club_eid("Arsenal")));
    assert_eq!(employer("Josep Guardiola Sala"), Some(club_eid("Manchester City")));

    // The rest of the backroom comes from the staff lists inside the club
    // record body: Slot's assistant is in Liverpool's coaching list, and the
    // roster seat marks Slot himself as the manager.
    assert_eq!(employer("Sipke Hulshoff"), Some(club_eid("Liverpool")));
    let role = |name: &str| {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .and_then(|p| p.staff_role)
    };
    assert_eq!(role("Arend Martijn Slot"), Some(fm_save::backroom::Role::Manager));
    assert_eq!(role("Sipke Hulshoff"), Some(fm_save::backroom::Role::Coaching));

    // Managers plus backroom staff, across the loaded world — 6,653 on this
    // save — not a handful of big names.
    let employed_staff = save
        .people
        .iter()
        .filter(|p| p.staff.is_some() && p.ability.is_none() && p.club_eid.is_some())
        .count();
    assert!(
        employed_staff > 6_000,
        "only {employed_staff} staff carry an employer"
    );
}

/// The non-player sheet reads back exactly what the pre-game editor shows.
///
/// Both people are checked against their editor "All Attributes" page, and
/// both matter for a different reason. Nikolić is a manager with a full
/// database row whose object carries a preamble, so a fixed stride to the five
/// u16s reads garbage. Fradley is a player-analyst: his own record holds no
/// sheet at all, and reading the block off the object sharing his eid gives
/// the next person's numbers — the mistake that defeated every earlier attempt
/// (`docs/OPEN_PROBLEMS.md` §3b).
#[test]
fn staff_sheets_match_the_editor() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipping: no Day One.fm");
        return;
    };

    let sheet = |name: &str| {
        save.people
            .iter()
            .find(|p| p.full_name == name)
            .unwrap_or_else(|| panic!("{name} is not in the save"))
            .staff
            .clone()
            .unwrap_or_else(|| panic!("{name} has no non-player sheet"))
    };

    // Editor: Current Ability 130, Potential 145, Current Reputation 130,
    // World 90, Home 125.
    let nikolic = sheet("Marko Nikolić");
    assert_eq!(nikolic.eid, 5155, "the sheet is one entity id below the person");
    assert_eq!((nikolic.current_ability, nikolic.potential_ability), (130, 145));
    assert_eq!(
        (
            nikolic.home_reputation,
            nikolic.current_reputation,
            nikolic.world_reputation
        ),
        (125, 130, 90),
        "the triple is home, current, world — not the editor's print order"
    );
    for (name, expected) in [
        ("Attacking", 9),
        ("Directness", 14),
        ("Authority", 17),
        ("Trigger Press", 16),
        ("Working With Youngsters", 11),
        ("Buying Players", 10),
        ("Mind Games", 12),
        ("Squad Rotation", 8),
        ("Judging Player Ability", 12),
        ("Judging Player Potential", 10),
        ("People Management", 10),
        ("Motivating", 16),
        ("Tactical Knowledge", 13),
        ("Coaching Attacking", 9),
        ("Coaching Defending", 16),
        ("Coaching Fitness", 11),
        ("Coaching Possession", 13),
        ("Coaching Technical", 13),
        ("Coaching Tactical", 14),
        ("Coaching Set Pieces", 12),
    ] {
        assert_eq!(nikolic.get(name), Some(expected), "Nikolić {name}");
    }

    // Arne Slot's sheet sits behind Maurice Verberne's identity block, which
    // carries no object header — the header-first scan missed it entirely
    // (Slot showed no sheet), and the value-shifted shadow had bound Verberne
    // himself to eid 526592 (2057 << 8). Both fixed 4 August 2026.
    let verberne = save
        .people
        .iter()
        .find(|p| p.full_name == "Maurice Verberne")
        .expect("Verberne is in the save");
    assert_eq!(
        (verberne.eid, verberne.uid),
        (Some(2057), Some(601_116)),
        "the headerless shadow must not win"
    );
    let slot = sheet("Arend Martijn Slot");
    assert_eq!(slot.eid, 2057, "Slot's sheet sits behind Verberne's identity");
    assert_eq!((slot.current_ability, slot.potential_ability), (165, 175));
    assert_eq!(
        (slot.home_reputation, slot.current_reputation, slot.world_reputation),
        (165, 165, 135)
    );

    // Editor: CA 140, PA 150, Current Reputation 140, World 110, Home 127.
    let fradley = sheet("Daniel Fradley");
    assert_eq!(fradley.eid, 20129);
    assert_eq!((fradley.current_ability, fradley.potential_ability), (140, 150));
    assert_eq!(
        (
            fradley.home_reputation,
            fradley.current_reputation,
            fradley.world_reputation
        ),
        (127, 140, 110)
    );
    for (name, expected) in [
        ("Working With Youngsters", 12),
        ("Judging Player Ability", 12),
        ("Judging Player Potential", 14),
        ("Judging Player Data", 16),
        ("People Management", 12),
        ("Motivating", 10),
        ("Tactical Knowledge", 16),
        ("Coaching Attacking", 5),
        ("Coaching Defending", 5),
        ("Coaching Fitness", 1),
        ("Coaching Possession", 6),
        ("Coaching Technical", 16),
        ("Coaching Tactical", 16),
        ("Coaching Set Pieces", 12),
    ] {
        assert_eq!(fradley.get(name), Some(expected), "Fradley {name}");
    }
}

/// The 26.0.0 regression, read straight from the frames — no person scan, so a
/// 456 MB save costs a decompression rather than a parse.
///
/// The header's stamp is the real-world time the file was written, October
/// 2025, when 26.0.0 was the shipping format. Reading it as the in-game date
/// put every person in this save eight years short of their age: sixteen-
/// year-old newgens showed as eight, and 49,161 of its people came out under
/// fourteen. The in-game date is `game_db.dat`'s week stamp, on both format
/// versions — and `game_db.dat` must come from the manifest, because on a
/// career this long the match-history member is the larger frame.
#[test]
fn an_aged_2600_save_reads_its_date_from_the_database_not_the_wall_clock() {
    let Some(bytes) = read_named("Adam Clouston.fm") else {
        eprintln!("skipped: no Adam Clouston.fm on this machine");
        return;
    };
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let header = frames.first().expect("header frame");
    assert_eq!(fm_save::gamedate::format_version(&header.data), Some((26, 0)));

    let members = fm_save::manifest::read_manifest(&frames.last().expect("manifest").data)
        .expect("a manifest");
    let index = fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
    let db = frames.get(index).expect("the database frame");
    let biggest = frames.iter().map(|f| f.data.len()).max().unwrap_or(0);
    assert!(
        db.data.len() < biggest,
        "the database is no longer the largest frame here — that is the whole point"
    );

    let date = fm_save::gamedate::find_main_frame_date(&db.data).expect("the week stamp");
    assert_eq!((date.year, date.month), (2033, 7), "the save's own date");

    let written = fm_save::gamedate::find_wall_clock_date(&header.data).expect("the header stamp");
    assert_eq!((written.year, written.month), (2025, 10), "when the file was written");
}

/// Nobody in a football database is nine years old or a hundred and nine.
///
/// Both used to show, from records that were never people: three string ids
/// that happen to resolve and four bytes that happen to decode as a date are
/// weak enough that a frame this size supplies about a thousand of them. The
/// nation field is the tell — FM's highest identifier is 249, and these read
/// 1280, 8704, 45209.
#[test]
fn nobody_is_born_outside_a_footballing_lifetime() {
    let Some(save) = load() else {
        eprintln!("skipped: no Career.fm on this machine");
        return;
    };
    let today = save.game_date.expect("the save's own date");

    let odd: Vec<&fm_save::person::Person> = save
        .people
        .iter()
        .filter(|p| !p.compact)
        .filter(|p| p.date_of_birth.is_some_and(|d| !(14..=100).contains(&d.age_on(today))))
        .collect();
    assert!(
        odd.len() < 10,
        "{} people outside a footballing lifetime, e.g. {:?}",
        odd.len(),
        odd.first().map(|p| &p.full_name)
    );

    let highest = save.people.iter().filter_map(|p| p.nation_id).max();
    assert!(
        highest.is_some_and(|n| n <= 249),
        "nation identifier past the end of FM's table: {highest:?}"
    );
}

/// Player reputation binds from the tag-02 line behind the one-eid-below
/// object, gated on the line repeating the player's own CA/PA. Haaland's
/// day-one line is his editor page exactly — Game Reputations 9350 current,
/// 9300 home, 9300 world, stored home/current/world ×50 — and coverage is
/// near-total, not a lucky handful. The historical "unrecoverable" verdict
/// (`OPEN_PROBLEMS.md` §3b) read the line after the person's *own* identity
/// copy, which belongs to the next person over.
#[test]
fn player_reputation_reads_haalands_editor_page() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let haaland = save
        .people
        .iter()
        .find(|p| p.full_name == "Erling Braut Haaland")
        .expect("Haaland missing from the people table");
    let rep = haaland.reputation.expect("Haaland should carry a reputation");
    assert_eq!((rep.home, rep.current, rep.world), (186, 187, 186));

    let players = save.people.iter().filter(|p| p.is_player()).count();
    let with_rep = save
        .people
        .iter()
        .filter(|p| p.is_player() && p.reputation.is_some())
        .count();
    assert!(
        with_rep * 100 >= players * 95,
        "reputation coverage collapsed: {with_rep} of {players} players"
    );
}

/// The human manager reads from `humans.dat` and their club falls out of the
/// manager-seat binding — no club id is stored in the member itself. The
/// Day One career is a Bala Town save.
#[test]
fn the_human_manager_resolves_to_their_club() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let eid = save.human_eid.expect("human eid should read from humans.dat");
    let human = save
        .people
        .iter()
        .find(|p| p.eid == Some(eid))
        .expect("human eid should resolve to a person");
    let club_eid = human.club_eid.expect("the Day One human manages a club");
    let club = save
        .clubs
        .iter()
        .find(|c| c.eid == Some(club_eid))
        .expect("the human's club should resolve");
    assert_eq!(club.name, "Bala Town");
}

/// The active tactic reads from `tactics_man.dat`: name and style in the
/// clear, eleven slots from the position masks, the stored XI in slot order.
/// The Port Talbot career carries a real 4-2-3-1; its keeper slot must
/// resolve to a person, and a fresh career must read no tactic at all
/// rather than a template.
#[test]
fn the_active_tactic_reads_name_shape_and_eleven() {
    let Some(save) = load_named("Port Talbot.fm") else {
        eprintln!("skipped: no Port Talbot.fm on this machine");
        return;
    };

    let tactic = save.tactic.as_ref().expect("Port Talbot has an active tactic");
    assert_eq!(tactic.name, "GYR - BLACK PANTHER 4231 FM26");
    assert_eq!(tactic.style.as_deref(), Some("Custom Gegenpress"));
    assert_eq!(
        tactic.positions,
        ["GK", "DR", "DC", "DC", "DL", "AMC", "DM", "DM", "AMR", "AML", "ST"]
    );
    assert_eq!(tactic.starters.len(), 11);
    let keeper = save.people.iter().find(|p| p.eid == Some(tactic.starters[0]));
    assert!(keeper.is_some(), "the XI's keeper slot should resolve to a person");

    if let Some(fresh) = load_named("Heybridge Swifts.fm") {
        assert!(fresh.tactic.is_none(), "a career with no tactic set must read none");
    }
}

/// Club reputation reads from the roster table on the editor's 0-10000 scale,
/// matching published FM26 values: Manchester City tops the English ladder,
/// well clear of a mid-table side, which is clear of a lower-league one. The
/// exact figure is asserted for City (9150 on this day-one save, round(v/100)
/// against fminside), the ordering for the rest — a scale that inverts or
/// collapses is what a wrong field would show.
#[test]
fn club_reputation_orders_the_ladder() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let rep = |name: &str| -> Option<u16> {
        save.clubs
            .iter()
            .filter(|c| c.name == name)
            .filter_map(|c| c.reputation)
            .max()
    };

    let city = rep("Manchester City").expect("Man City should carry a reputation");
    assert_eq!(city, 9150, "Man City reputation off published FM26 value");

    let liverpool = rep("Liverpool").expect("Liverpool should carry a reputation");
    assert!(liverpool > 8000 && liverpool < city, "Liverpool below City, well above mid-table");

    // Coverage is partial by nature — only clubs in a loaded competition carry
    // a roster row — but the elite must all decode.
    let with_rep = save.clubs.iter().filter(|c| c.reputation.is_some()).count();
    assert!(with_rep > 3000, "club reputation coverage collapsed: {with_rep}");
}

/// Gender must not be asserted wrongly, which is worse than not asserting it.
///
/// Two inference schemes died before the save's own field was found. The
/// widest-median-gap boundary filed Haaland and most of Liverpool's first
/// team as women on a 2037 career; its fewest-straddled-squads successor
/// fixed them but still misfiled ~700 men on a fresh save — whole foreign
/// squads (US Monastir, Kolos, Urawa) whose forenames sit past the "female"
/// block, because the pool's tail is not purely female — and returned
/// nothing at all on a 2035 career once newgen names blurred the split.
/// Gender now reads from the identity-object header's type byte, bit 0x10 —
/// FM's own record — verified by squad purity on three saves.
#[test]
fn men_are_not_filed_as_women() {
    let Some(save) = load_named("Day One.fm") else {
        eprintln!("skipped: no Day One.fm on this machine");
        return;
    };

    let person = |needle: &str| -> &fm_save::Person {
        save.people
            .iter()
            .find(|p| p.full_name.contains(needle))
            .unwrap_or_else(|| panic!("{needle} is not in this save"))
    };

    // Men whose forename ids sit high enough that a boundary in the middle of
    // the male range calls them women — Haaland was the index case — plus the
    // men the squad-purity inference itself misfiled: members of foreign
    // squads whose forename ids sit past the female block's start.
    for name in [
        "Erling Braut Haaland",
        "Mohamed Salah",
        "Virgil van Dijk",
        "Rifet Kapić",
        "Stepanenko Taras Mykolayovych",
    ] {
        assert_eq!(person(name).female, Some(false), "{name} should be a man");
    }

    // And the women's game reads as itself.
    for name in ["Sam Kerr", "Millie Bright", "Aitana Bonmatí"] {
        assert_eq!(person(name).female, Some(true), "{name} should be a woman");
    }

    // The women's game must still be recognised, or the split is not a split.
    let women = save.people.iter().filter(|p| p.female == Some(true)).count();
    let men = save.people.iter().filter(|p| p.female == Some(false)).count();
    assert!(men > women, "a save should hold more men than women: {men} vs {women}");
    assert!(women > 1_000, "women's football should be recognised, not erased: {women}");

    // Every member of a squad shares its gender — the claim the boundary rests
    // on, asserted where it is easiest to see.
    let by_eid: std::collections::HashMap<u32, bool> = save
        .people
        .iter()
        .filter_map(|p| Some((p.eid?, p.female?)))
        .collect();
    let mixed = save
        .squads
        .iter()
        .filter(|s| {
            let g: Vec<bool> = s.player_eids.iter().filter_map(|e| by_eid.get(e).copied()).collect();
            g.len() >= 5 && g.iter().any(|&x| x) && g.iter().any(|&x| !x)
        })
        .count();
    assert_eq!(mixed, 0, "squads are single-gender, so none may be mixed after binding");
}
