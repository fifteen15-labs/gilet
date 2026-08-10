//! Prints clubs matching a name needle, or the club at a given eid.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::cast_precision_loss, clippy::missing_docs_in_private_items)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");
    for a in &args[2..] {
        if let Ok(eid) = a.parse::<u32>() {
            match save.clubs.iter().find(|c| c.eid == Some(eid)) {
                Some(c) => println!("eid {eid}: {} / {} uid {:?} nation {:?}", c.short_name, c.name, c.uid, c.nation_id),
                None => println!("eid {eid}: no club"),
            }
        } else {
            for c in save.clubs.iter().filter(|c| c.name.contains(a.as_str()) || c.short_name.contains(a.as_str())) {
                println!("{a}: {} / {} eid {:?} uid {:?}", c.short_name, c.name, c.eid, c.uid);
            }
        }
    }
}
