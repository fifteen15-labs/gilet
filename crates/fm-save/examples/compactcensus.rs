//! Census of compact entries after the type-byte fix: counts by gender,
//! sample names, and duplicate-eid safety.
#![allow(clippy::unwrap_used, clippy::expect_used)]
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let name = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().into_owned();
        let compact: Vec<_> = save.people.iter().filter(|p| p.compact).collect();
        let women = compact.iter().filter(|p| p.female == Some(true)).count();
        let sample: Vec<&str> = compact.iter().rev().take(8).map(|p| p.full_name.as_str()).collect();
        let mut eids: Vec<u32> = save.people.iter().filter_map(|p| p.eid).collect();
        let total = eids.len();
        eids.sort_unstable();
        eids.dedup();
        println!("{name}: compact {} (women {women})  dup eids {}  tail: {}", compact.len(), total - eids.len(), sample.join(", "));
    }
}
