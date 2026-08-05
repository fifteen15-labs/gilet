//! Lists every nation identifier with the clubs that carry it and the
//! best-known people who do, so the unnamed ones can be identified.
//!
//! Club names settle it where a squad of players only suggests: the club table
//! stores its names in the clear and uses the same nation numbering as people
//! do, so "Ba FC, Labasa FC, Lautoka FC" names 179 as Fiji outright.
//!
//! ```text
//! cargo run --release --example nations -- save.fm [all]
//! ```

// Research spike, not shipped code.
#![allow(clippy::indexing_slicing)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: nations <save.fm> [all]")?;
    let only_unnamed = std::env::args().nth(2).as_deref() != Some("all");
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let mut people: std::collections::HashMap<u16, Vec<(&str, u8)>> = std::collections::HashMap::new();
    for p in &save.people {
        if let Some(nation) = p.nation_id {
            people
                .entry(nation)
                .or_default()
                .push((p.full_name.as_str(), p.ability.as_ref().map_or(0, |a| a.current)));
        }
    }
    let mut clubs: std::collections::HashMap<u16, Vec<&str>> = std::collections::HashMap::new();
    for c in &save.clubs {
        if let Ok(nation) = u16::try_from(c.nation_id) {
            clubs.entry(nation).or_default().push(c.name.as_str());
        }
    }

    let mut ids: Vec<u16> = people.keys().chain(clubs.keys()).copied().collect();
    ids.sort_unstable();
    ids.dedup();
    ids.sort_by_key(|id| std::cmp::Reverse(people.get(id).map_or(0, Vec::len)));

    for id in ids {
        let named = fm_save::person::nation_name(id);
        if only_unnamed && named.is_some() {
            continue;
        }
        let mut folk = people.remove(&id).unwrap_or_default();
        folk.sort_by_key(|(_, ca)| std::cmp::Reverse(*ca));
        let top: Vec<String> = folk.iter().take(6).map(|(n, ca)| format!("{n} ({ca})")).collect();
        let sides = clubs.remove(&id).unwrap_or_default();
        println!(
            "{id:>5} {:<20} {:>6} people, {:>5} clubs",
            named.unwrap_or("?"),
            folk.len(),
            sides.len()
        );
        if !sides.is_empty() {
            println!("      clubs:  {}", sides.iter().take(8).copied().collect::<Vec<_>>().join(" | "));
        }
        if !top.is_empty() {
            println!("      people: {}", top.join(", "));
        }
    }
    Ok(())
}
