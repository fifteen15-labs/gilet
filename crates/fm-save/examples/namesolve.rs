//! Names attribute indices by intersecting several in-game player reports.
//!
//! Each report pins an index only when its displayed value is unique on that
//! screen; across five players with different profiles almost every index
//! resolves. Attributes the game hides for a given player (goalkeeping ones
//! for outfielders, most technical ones for keepers) are left blank.
//!
//! ```text
//! cargo run --release --example namesolve -- Ongoing.fm
//! ```

// Research spike, not shipped code.
#![allow(clippy::indexing_slicing, clippy::too_many_lines)]

/// One player's report: the name to search for, then `(attribute, value)` for
/// every attribute the game showed on screen.
struct Report {
    query: &'static str,
    values: &'static [(&'static str, u8)],
}

const REPORTS: &[Report] = &[
    Report {
        query: "Jamal Musiala",
        values: &[
            ("Crossing", 13), ("Dribbling", 18), ("Finishing", 16), ("First Touch", 17),
            ("Heading", 13), ("Long Shots", 14), ("Marking", 12), ("Passing", 17),
            ("Tackling", 11), ("Technique", 18), ("Corners", 10), ("Free Kick Taking", 11),
            ("Long Throws", 5), ("Penalty Taking", 14), ("Aggression", 12), ("Anticipation", 16),
            ("Bravery", 13), ("Composure", 18), ("Concentration", 13), ("Decisions", 16),
            ("Determination", 16), ("Flair", 19), ("Leadership", 9), ("Off the Ball", 16),
            ("Positioning", 11), ("Teamwork", 14), ("Vision", 17), ("Work Rate", 15),
            ("Acceleration", 14), ("Agility", 18), ("Balance", 16), ("Jumping Reach", 12),
            ("Natural Fitness", 14), ("Pace", 15), ("Stamina", 14), ("Strength", 12),
        ],
    },
    Report {
        query: "Soul",
        values: &[
            ("Crossing", 16), ("Dribbling", 16), ("Finishing", 16), ("First Touch", 16),
            ("Heading", 6), ("Long Shots", 15), ("Marking", 9), ("Passing", 16),
            ("Tackling", 12), ("Technique", 17), ("Corners", 15), ("Free Kick Taking", 16),
            ("Long Throws", 6), ("Penalty Taking", 13), ("Aggression", 13), ("Anticipation", 14),
            ("Bravery", 12), ("Composure", 14), ("Concentration", 13), ("Decisions", 13),
            ("Determination", 16), ("Flair", 17), ("Leadership", 11), ("Off the Ball", 13),
            ("Positioning", 10), ("Teamwork", 13), ("Vision", 16), ("Work Rate", 13),
            ("Acceleration", 15), ("Agility", 18), ("Balance", 16), ("Jumping Reach", 8),
            ("Natural Fitness", 15), ("Pace", 15), ("Stamina", 15), ("Strength", 11),
        ],
    },
    Report {
        query: "Sandro Tonali",
        values: &[
            ("Crossing", 12), ("Dribbling", 12), ("Finishing", 9), ("First Touch", 17),
            ("Heading", 8), ("Long Shots", 11), ("Marking", 13), ("Passing", 17),
            ("Tackling", 14), ("Technique", 16), ("Corners", 14), ("Free Kick Taking", 12),
            ("Long Throws", 8), ("Penalty Taking", 12), ("Aggression", 15), ("Anticipation", 16),
            ("Bravery", 16), ("Composure", 16), ("Concentration", 16), ("Decisions", 16),
            ("Determination", 16), ("Flair", 13), ("Leadership", 14), ("Off the Ball", 13),
            ("Positioning", 17), ("Teamwork", 18), ("Vision", 15), ("Work Rate", 19),
            ("Acceleration", 11), ("Agility", 12), ("Balance", 14), ("Jumping Reach", 9),
            ("Natural Fitness", 15), ("Pace", 12), ("Stamina", 13), ("Strength", 11),
        ],
    },
    Report {
        query: "Ayyoub Bouaddi",
        values: &[
            ("Crossing", 13), ("Dribbling", 14), ("Finishing", 9), ("First Touch", 16),
            ("Heading", 11), ("Long Shots", 10), ("Marking", 18), ("Passing", 17),
            ("Tackling", 15), ("Technique", 17), ("Corners", 6), ("Free Kick Taking", 4),
            ("Long Throws", 5), ("Penalty Taking", 13), ("Aggression", 11), ("Anticipation", 17),
            ("Bravery", 13), ("Composure", 16), ("Concentration", 18), ("Decisions", 16),
            ("Determination", 16), ("Flair", 14), ("Leadership", 12), ("Off the Ball", 13),
            ("Positioning", 18), ("Teamwork", 14), ("Vision", 17), ("Work Rate", 16),
            ("Acceleration", 15), ("Agility", 16), ("Balance", 14), ("Jumping Reach", 15),
            ("Natural Fitness", 16), ("Pace", 15), ("Stamina", 16), ("Strength", 16),
        ],
    },
    Report {
        // A goalkeeper: the game shows the goalkeeping set instead of most of
        // the technical one, which is the only way to name those eleven.
        query: "Lucas Chevalier",
        values: &[
            ("Aerial Reach", 15), ("Command of Area", 14), ("Communication", 15),
            ("Eccentricity", 14), ("First Touch", 14), ("Handling", 15), ("Kicking", 13),
            ("One on Ones", 18), ("Passing", 15), ("Punching Tendency", 5), ("Reflexes", 15),
            ("Rushing Out Tendency", 15), ("Throwing", 16),
            ("Aggression", 12), ("Anticipation", 16), ("Bravery", 14), ("Composure", 14),
            ("Concentration", 13), ("Decisions", 15), ("Determination", 16), ("Flair", 15),
            ("Leadership", 15), ("Off the Ball", 11), ("Positioning", 14), ("Teamwork", 12),
            ("Vision", 15), ("Work Rate", 14),
            ("Acceleration", 11), ("Agility", 15), ("Balance", 12), ("Jumping Reach", 15),
            ("Natural Fitness", 13), ("Pace", 12), ("Stamina", 13), ("Strength", 12),
            ("Free Kick Taking", 4), ("Penalty Taking", 5), ("Technique", 13),
        ],
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: namesolve <save.fm>")?;
    let bytes = std::fs::read(&path)?;
    let save = fm_save::Save::parse(&bytes)?;

    // Decoded block per report.
    let mut blocks: Vec<(&str, [u8; fm_save::ability::ATTRIBUTE_COUNT])> = Vec::new();
    for r in REPORTS {
        let needle = r.query.to_lowercase();
        let person = save
            .people
            .iter()
            .find(|p| p.full_name.to_lowercase().contains(&needle))
            .ok_or_else(|| format!("{} not found", r.query))?;
        let a = person.ability.as_ref().ok_or_else(|| format!("{} has no block", r.query))?;
        println!("{:<26} CA {} PA {}", person.full_name, a.current, a.potential);
        blocks.push((r.query, a.attributes));
    }

    // Every attribute name mentioned anywhere.
    let mut names: Vec<&str> = REPORTS
        .iter()
        .flat_map(|r| r.values.iter().map(|(n, _)| *n))
        .collect();
    names.sort_unstable();
    names.dedup();

    // An index matches a name when every report that shows the name agrees
    // with the decoded value at that index.
    let mut by_index: Vec<Vec<&str>> = vec![Vec::new(); fm_save::ability::ATTRIBUTE_COUNT];
    let mut by_name: std::collections::BTreeMap<&str, Vec<usize>> = std::collections::BTreeMap::new();
    for name in &names {
        for index in 0..fm_save::ability::ATTRIBUTE_COUNT {
            let agrees = REPORTS.iter().zip(&blocks).all(|(r, (_, attrs))| {
                r.values
                    .iter()
                    .find(|(n, _)| n == name)
                    .is_none_or(|(_, v)| attrs[index] == *v)
            });
            if agrees {
                by_index[index].push(name);
                by_name.entry(name).or_default().push(index);
            }
        }
    }

    println!("\n--- unique matches ---");
    for (name, idx) in &by_name {
        if idx.len() == 1 {
            println!("  {:>2}  {name}", idx[0]);
        }
    }
    println!("\n--- ambiguous names ---");
    for (name, idx) in &by_name {
        if idx.len() != 1 {
            println!("  {name}: {idx:?}");
        }
    }
    println!("\n--- per index ---");
    for (i, cands) in by_index.iter().enumerate() {
        let values: Vec<String> = blocks.iter().map(|(_, a)| a[i].to_string()).collect();
        println!("  {i:>2}: [{}]  {}", values.join(","), cands.join(" | "));
    }
    Ok(())
}
