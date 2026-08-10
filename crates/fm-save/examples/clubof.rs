//! Prints the club and squad level of named people — the quick check that a
//! binding change did what FM's own search shows.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("save");
    let bytes = std::fs::read(&path).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    for needle in args {
        match save.people.iter().find(|p| p.full_name.contains(&needle)) {
            None => println!("{needle}: NOT FOUND"),
            Some(p) => {
                let club = p
                    .club_eid
                    .and_then(|e| save.clubs.iter().find(|c| c.eid == Some(e)))
                    .map_or("-", |c| c.short_name.as_str());
                println!(
                    "{:<36} club={club:<20} level={:?}",
                    p.full_name,
                    p.squad_level.map(fm_save::squad::SquadKind::name)
                );
            }
        }
    }
}
