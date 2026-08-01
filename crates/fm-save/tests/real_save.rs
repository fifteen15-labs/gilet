//! Squad-table checks that need the raw parse rather than the command layer:
//! captains and entity ids are not part of what the UI consumes yet.
//!
//! Skips rather than fails when no save is present, like the journey tests.

#![allow(clippy::unwrap_used, clippy::expect_used)]

fn load() -> Option<fm_save::Save> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home)
        .join("Library/Application Support/Sports Interactive/Football Manager 26/games/Career.fm");
    let bytes = std::fs::read(path).ok()?;
    Some(fm_save::Save::parse(&bytes).unwrap())
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
