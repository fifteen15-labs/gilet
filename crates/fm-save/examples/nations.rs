//! Lists the best-known players of every nation identifier, so the ones that
//! are still unnamed can be identified by their squads.
//!
//! ```text
//! cargo run --release --example nations -- save.fm
//! ```

// Research spike, not shipped code.
#![allow(clippy::indexing_slicing)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: nations <save.fm>")?;
    let only_unnamed = std::env::args().nth(2).as_deref() != Some("all");
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    let mut by_nation: std::collections::HashMap<u16, Vec<(&str, u8)>> = std::collections::HashMap::new();
    for p in &save.people {
        if let Some(a) = &p.ability {
            by_nation
                .entry(p.nation_id)
                .or_default()
                .push((p.full_name.as_str(), a.current));
        }
    }

    let mut nations: Vec<(u16, Vec<(&str, u8)>)> = by_nation.into_iter().collect();
    nations.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    for (id, mut players) in nations {
        if only_unnamed && fm_save::person::nation_name(id).is_some() {
            continue;
        }
        if players.len() < 3 {
            continue;
        }
        players.sort_by_key(|(_, ca)| std::cmp::Reverse(*ca));
        let top: Vec<String> = players
            .iter()
            .take(10)
            .map(|(n, ca)| format!("{n} ({ca})"))
            .collect();
        let name = fm_save::person::nation_name(id).unwrap_or("?");
        println!("{id:>4} {name:<24} [{:>5} players]  {}", players.len(), top.join(", "));
    }
    Ok(())
}
