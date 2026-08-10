//! Samples the clubless-but-contracted players: who they are, what they
//! earn, where their nation sits — the pattern hunt for the next binding.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().into_owned();
        println!("== {name}");
        let mut sample: Vec<_> = save
            .people
            .iter()
            .filter(|p| p.is_player() && p.club_eid.is_none() && (p.contract_until.is_some() || p.wage.is_some_and(|w| w > 0)))
            .collect();
        sample.sort_by_key(|p| std::cmp::Reverse(p.wage.unwrap_or(0)));
        println!("  total {}", sample.len());
        for p in sample.iter().take(12) {
            println!(
                "  {:<34} wage {:>9?} until {:?} nation {:?} eid {:?}",
                p.full_name,
                p.wage,
                p.contract_until.map(|d| (d.day, d.month, d.year)),
                p.nation_id,
                p.eid,
            );
        }
        // nation histogram of the gap
        let mut by_nation: std::collections::HashMap<Option<u16>, usize> = std::collections::HashMap::new();
        for p in &sample {
            *by_nation.entry(p.nation_id).or_default() += 1;
        }
        let mut top: Vec<_> = by_nation.into_iter().collect();
        top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let show: Vec<String> = top
            .iter()
            .take(10)
            .map(|(id, n)| {
                let label = id
                    .and_then(fm_save::person::nation_name)
                    .map_or_else(|| id.map_or_else(|| "?".to_owned(), |i| i.to_string()), str::to_owned);
                format!("{label}:{n}")
            })
            .collect();
        println!("  nations: {}", show.join("  "));
    }
}
