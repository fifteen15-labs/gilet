//! Writes a copy of a save with one player added to or removed from an
//! in-game shortlist. Development aid for verifying the write path against
//! FM itself — the output is a new file, never the input:
//!
//! ```text
//! cargo run --release --example edit_shortlist -- \
//!     <save.fm> <out.fm> <shortlist name> add|remove "Player Name"
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output, list, op, player_name] = args.as_slice() else {
        eprintln!("usage: edit_shortlist <save.fm> <out.fm> <list> add|remove <player name>");
        std::process::exit(2);
    };

    let bytes = std::fs::read(input)?;
    let save = fm_save::Save::parse(&bytes)?;

    let person = save
        .people
        .iter()
        .find(|p| &p.full_name == player_name)
        .ok_or_else(|| format!("no person named {player_name}"))?;
    let eid = person.eid.ok_or("person has no entity id")?;
    let today = save.game_date.ok_or("save's own date is unknown; refusing to invent one")?;

    let scout = fm_save::archive::member_plaintext(&bytes, "scout_man.dat")?;
    let edited = match op.as_str() {
        "add" => {
            let stamp = fm_save::shortlist::date_added_bytes(today);
            fm_save::shortlist::add_entry(&scout, Some(list), eid, stamp)
        }
        "remove" => fm_save::shortlist::remove_entry(&scout, Some(list), eid),
        other => return Err(format!("unknown operation {other}").into()),
    }
    .ok_or_else(|| format!("no shortlist named {list}"))?;

    let written = fm_save::archive::replace_member(&bytes, "scout_man.dat", &edited)?;

    // Prove the edit before writing anything: the rebuilt save must reparse
    // and show the change.
    let reparsed = fm_save::Save::parse(&written)?;
    let members = reparsed
        .shortlists
        .iter()
        .find(|s| s.name.as_deref() == Some(list.as_str()))
        .map(|s| s.person_eids.clone())
        .unwrap_or_default();
    println!(
        "{list}: {} members after {op} ({} contains {player_name})",
        members.len(),
        if members.contains(&eid) { "now" } else { "no longer" },
    );

    std::fs::write(output, written)?;
    println!("wrote {output}");
    Ok(())
}
