//! Prints clubs whose record carries no eid, filtered by name needle, plus
//! counts — the population the employer lookup cannot reach.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    let total = save.clubs.iter().filter(|c| c.eid.is_none()).count();
    println!("clubs without eid: {total} of {}", save.clubs.len());
    for a in &args[2..] {
        for c in save.clubs.iter().filter(|c| c.name.contains(a.as_str()) || c.short_name.contains(a.as_str())) {
            println!("{a}: '{}' / '{}' eid {:?} uid {:?} nation {} offset 0x{:x}", c.short_name, c.name, c.eid, c.uid, c.nation_id, c.offset);
        }
    }
}
