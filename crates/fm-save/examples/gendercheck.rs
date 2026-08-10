//! Validates the candidate explicit-gender bits found by `genderbit`:
//! identity-13 bit 6 (0x40) and identity-7 bit 4 (0x10). For each save,
//! counts people flagged by each bit, checks curated names, and prints the
//! verdict for the men the forename boundary is known to misfile.
//!
//! ```text
//! cargo run --release --example gendercheck -- <save.fm>...
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::indexing_slicing
)]

const KNOWN_WOMEN: &[&str] = &["Sam Kerr", "Millie Bright", "Marta Vieira da Silva", "Lucy Bronze", "Aitana Bonmat"];
const KNOWN_MEN: &[&str] = &["Erling Braut Haaland", "Virgil van Dijk", "Paolo Cesare Maldini", "Kylian Mbapp"];
// Men the boundary+squad verdict filed as women on Day One.
const MISFILED_MEN: &[&str] = &[
    "Rifet Kapi",
    "Stepanenko Taras",
    "González Copete",
    "Zéguéï",
    "Rayviën Rosario",
];

fn main() {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("read save");
        let save = fm_save::Save::parse(&bytes).expect("parse save");
        let frames = fm_save::container::read_frames(&bytes).expect("frames");
        let members =
            fm_save::manifest::read_manifest(&frames.last().unwrap().data).expect("manifest");
        let index =
            fm_save::manifest::frame_index_of(&members, "game_db.dat").expect("game_db.dat");
        let frame = &frames[index].data;

        let name = std::path::Path::new(&path)
            .file_stem()
            .map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
        println!("== {name}");

        let identity = |p: &fm_save::person::Person| -> Option<usize> {
            let (eid, uid) = (p.eid?, p.uid?);
            let mut needle = Vec::with_capacity(12);
            needle.extend_from_slice(&eid.to_le_bytes());
            needle.extend_from_slice(&uid.to_le_bytes());
            needle.extend_from_slice(&uid.to_le_bytes());
            let window = frame.get(p.offset..p.offset + 2048)?;
            window.windows(12).position(|w| w == needle).map(|pos| p.offset + pos)
        };

        let mut m13 = 0usize; // identity-13 bit 6 set
        let mut m7 = 0usize; // identity-7 bit 4 set
        let mut both = 0usize;
        let mut either = 0usize;
        let mut with_identity = 0usize;
        let mut disagree = 0usize; // against the squad/boundary verdict
        for p in &save.people {
            if p.compact {
                continue;
            }
            let Some(at) = identity(p) else { continue };
            with_identity += 1;
            let b13 = frame.get(at.wrapping_sub(13)).is_some_and(|b| b & 0x40 != 0);
            let b7 = frame.get(at.wrapping_sub(7)).is_some_and(|b| b & 0x10 != 0);
            m13 += usize::from(b13);
            m7 += usize::from(b7);
            both += usize::from(b13 && b7);
            either += usize::from(b13 || b7);
            if p.female == Some(!b7) {
                disagree += 1;
            }
        }
        println!(
            "  people with identity {with_identity}: bit13 {m13}  bit7 {m7}  both {both}  either {either}"
        );
        println!(
            "  disagreements with shipped verdict (where verdict exists): {disagree}"
        );

        let check = |label: &str, names: &[&str]| {
            for n in names {
                let Some(p) = save
                    .people
                    .iter()
                    .find(|p| !p.compact && p.full_name.contains(n))
                else {
                    continue;
                };
                let Some(at) = identity(p) else {
                    println!("  {label} {n}: no identity block");
                    continue;
                };
                let b13 = frame.get(at - 13).is_some_and(|b| b & 0x40 != 0);
                let b7 = frame.get(at - 7).is_some_and(|b| b & 0x10 != 0);
                println!(
                    "  {label} {:<30} bit13 {} bit7 {}  (shipped female={:?})",
                    p.full_name, b13, b7, p.female
                );
            }
        };
        check("WOMAN   ", KNOWN_WOMEN);
        check("MAN     ", KNOWN_MEN);
        check("MISFILED", MISFILED_MEN);
    }
}
