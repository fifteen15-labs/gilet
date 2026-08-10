//! Samples the clubless-and-contractless players — the population the free
//! agent filter shows — grouped by age band and CA, with names.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().into_owned();
        let year = save.game_date.map_or(2025, |d| d.year);
        let sample: Vec<_> = save
            .people
            .iter()
            .filter(|p| p.is_player() && p.club_eid.is_none() && p.wage.is_none() && p.contract_until.is_none())
            .collect();
        let mut by_band = [0usize; 5];
        let mut top: Vec<_> = sample.clone();
        for p in &sample {
            let age = p.date_of_birth.map(|d| i64::from(year) - i64::from(d.year));
            let band = match age { Some(a) if a < 18 => 0, Some(a) if a < 24 => 1, Some(a) if a < 30 => 2, Some(a) if a < 38 => 3, _ => 4 };
            by_band[band] += 1;
        }
        top.sort_by_key(|p| std::cmp::Reverse(p.ability.as_ref().map_or(0, |a| a.current)));
        println!("== {name}: {} clubless no-contract players", sample.len());
        println!("   age bands: <18 {}  18-23 {}  24-29 {}  30-37 {}  38+/unknown {}", by_band[0], by_band[1], by_band[2], by_band[3], by_band[4]);
        for p in top.iter().take(10) {
            println!(
                "   CA {:?} {:<34} b.{:?} nation {:?}",
                p.ability.as_ref().map(|a| a.current),
                p.full_name,
                p.date_of_birth.map(|d| d.year),
                p.nation_id
            );
        }
    }
}
