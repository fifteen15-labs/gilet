//! Why a club shows no players.
//!
//! Takes a save and a name fragment, and reports, for every club matching it:
//! whether the club record carries the `(eid, uid)` pair the squad table is
//! validated against, whether a squad record references that eid, and how many
//! of the squad's person eids resolve to a decoded person. Also looks up a
//! person by name fragment and says which squad, if any, lists them.
//!
//! Usage: `cargo run --release --example clubgap -- <save.fm> <club> [person]`

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]

use std::collections::{HashMap, HashSet};

fn name_of_club(save: &fm_save::Save, eid: u32) -> &str {
    save.clubs
        .iter()
        .find(|c| c.eid == Some(eid))
        .map_or("?", |c| c.name.as_str())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: clubgap <save.fm> <club> [person]");
    let club_query = args.next().expect("usage: clubgap <save.fm> <club> [person]").to_lowercase();
    let person_query = args.next().map(|q| q.to_lowercase());

    let bytes = std::fs::read(&path).expect("read save");
    let save = fm_save::Save::parse(&bytes).expect("parse save");

    // Club offsets are into the single largest frame, which is where
    // `Save::parse` scans them from.
    let frames = fm_save::container::read_frames(&bytes).expect("frames");
    let main = frames.iter().max_by_key(|f| f.data.len()).expect("main frame");
    let frame = &main.data;

    let read_u32 = |at: usize| -> Option<u32> {
        frame.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };

    // The head invariants `club::parse_head` checks, printed so a failing
    // record says which one it broke rather than just "no eid".
    let head_report = |len_at: usize| {
        let at = |back: usize| len_at.checked_sub(back).and_then(read_u32);
        let nation1 = at(14);
        let nation2 = at(18);
        let nation3 = at(26);
        let ff = at(22);
        let zero = len_at.checked_sub(27).and_then(|p| frame.get(p)).copied();
        let uid = at(35);
        let uid2 = at(31);
        let eid = at(39);
        println!(
            "  head: nation1 {nation1:?} nation2 {nation2:?} nation3 {nation3:?} \
             ff {ff:08X?} zero {zero:?} eid {eid:?} uid {uid:?} uid2 {uid2:?}",
            ff = ff.unwrap_or(0)
        );
        let mut broke = Vec::new();
        if nation1 != nation2 || nation2 != nation3 {
            broke.push("nation triple");
        }
        if ff != Some(0xFFFF_FFFF) {
            broke.push("FFFFFFFF");
        }
        if zero != Some(0) {
            broke.push("zero byte");
        }
        if uid != uid2 {
            broke.push("uid repeat");
        }
        if uid == Some(0) || uid == Some(u32::MAX) {
            broke.push("uid range");
        }
        println!(
            "  broke: {}",
            if broke.is_empty() { "nothing".to_string() } else { broke.join(", ") }
        );
        let from = len_at.saturating_sub(48);
        let window: Vec<String> =
            frame.get(from..len_at + 8).unwrap_or_default().iter().map(|b| format!("{b:02X}")).collect();
        println!("  bytes[-48..+8]: {}", window.join(" "));
    };

    println!(
        "{}: {} people, {} clubs, {} squads, {} stubs",
        path,
        save.people.len(),
        save.clubs.len(),
        save.squads.len(),
        save.stubs.len()
    );

    // Person eids that actually resolved to a decoded person record.
    let people_by_eid: HashMap<u32, &fm_save::Person> =
        save.people.iter().filter_map(|p| p.eid.map(|e| (e, p))).collect();
    let stub_eids: HashSet<u32> = save.stubs.iter().map(|s| s.eid).collect();

    // Squads by the club eid they claim.
    let mut squads_by_club: HashMap<u32, Vec<&fm_save::Squad>> = HashMap::new();
    for squad in &save.squads {
        squads_by_club.entry(squad.club_eid).or_default().push(squad);
    }

    let matches: Vec<&fm_save::Club> = save
        .clubs
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&club_query)
                || c.short_name.to_lowercase().contains(&club_query)
        })
        .collect();

    // `*` surveys every headless club instead of naming one, tallying which
    // invariant each broke — the question is whether TNS is a one-off or a
    // class.
    if club_query == "*" {
        // Club eids a squad record actually references. A headless club whose
        // candidate eid appears here is a real club the reader is still
        // failing to link — the strongest evidence available without the game.
        let referenced: HashSet<u32> = save.squads.iter().map(|s| s.club_eid).collect();

        let mut with_eid = 0usize;
        let mut buckets: HashMap<String, usize> = HashMap::new();
        let mut orphans = Vec::new();
        let mut orphan_count = 0usize;

        for club in &save.clubs {
            if club.eid.is_some() {
                with_eid += 1;
                continue;
            }
            let len_at = club.offset;
            let at = |back: usize| len_at.checked_sub(back).and_then(read_u32);
            let (n1, location, n3) = (at(14), at(18), at(26));
            let ff = at(22);
            let zero = len_at.checked_sub(27).and_then(|p| frame.get(p)).copied();
            let (uid, uid2) = (at(35), at(31));
            let eid = at(39);

            let mut broke = Vec::new();
            if n1 != n3 {
                broke.push("nation pair");
            }
            if location == Some(0) || location.is_none_or(|l| l > 10_000) {
                broke.push("location range");
            }
            if ff != Some(0xFFFF_FFFF) {
                broke.push("FFFFFFFF");
            }
            if zero != Some(0) {
                broke.push("zero byte");
            }
            if uid != uid2 {
                broke.push("uid repeat");
            }
            if uid == Some(0) || uid == Some(u32::MAX) {
                broke.push("uid range");
            }
            *buckets.entry(broke.join(" + ")).or_default() += 1;

            // Is the candidate eid one a squad record points at?
            if let Some(candidate) = eid {
                if referenced.contains(&candidate) {
                    orphan_count += 1;
                    if orphans.len() < 20 {
                        let members = save
                            .squads
                            .iter()
                            .find(|s| s.club_eid == candidate)
                            .map_or(0, |s| s.player_eids.len());
                        orphans.push(format!(
                            "{:?} eid {candidate} squad of {members} — broke [{}]",
                            club.name,
                            broke.join(" + ")
                        ));
                    }
                }
            }
        }

        let linked = save.people.iter().filter(|p| p.club_eid.is_some()).count();
        let squad_clubs = referenced.len();
        println!("\n=== headless club survey ===");
        println!("  {with_eid} clubs have an eid");
        println!("  {linked} people are linked to a club");
        println!("  {squad_clubs} distinct clubs own a squad");

        let mut sorted: Vec<(&String, &usize)> = buckets.iter().collect();
        sorted.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("\n  headless records by what they broke:");
        for (what, n) in sorted {
            println!("    {n:>5}  {what}");
        }

        println!("\n  {orphan_count} headless records carry an eid a squad references:");
        for line in &orphans {
            println!("    {line}");
        }

        // The other direction, which is what the user actually sees: a squad
        // whose club eid matches no club record is a team with players and no
        // name. Those players show a blank club in the table.
        let named: HashSet<u32> = save.clubs.iter().filter_map(|c| c.eid).collect();
        let nameless: Vec<&fm_save::Squad> =
            save.squads.iter().filter(|s| !named.contains(&s.club_eid)).collect();
        let stranded: usize = nameless.iter().map(|s| s.player_eids.len()).sum();
        println!(
            "\n  {} squads reference a club eid no club record carries ({stranded} players stranded)",
            nameless.len()
        );
        for squad in nameless.iter().take(15) {
            let sample: Vec<&str> = squad
                .player_eids
                .iter()
                .filter_map(|e| people_by_eid.get(e).map(|p| p.full_name.as_str()))
                .take(3)
                .collect();
            println!(
                "    club eid {:>7}  {} members  e.g. {:?}",
                squad.club_eid,
                squad.player_eids.len(),
                sample
            );
        }

        // And clubs that parsed fine but own no squad — expected for leagues
        // the save never loaded, so the count is context, not a defect.
        let squadless = save
            .clubs
            .iter()
            .filter(|c| c.eid.is_some_and(|e| !referenced.contains(&e)))
            .count();
        println!("  {squadless} clubs have an eid but no squad record (unloaded leagues)");

        // A false-positive squad is a run of bytes that passed the shape
        // tests, so its "members" would not resolve to decoded people. Real
        // squads resolve almost entirely.
        let mut ragged = 0usize;
        let mut worst = Vec::new();
        let mut total_members = 0usize;
        let mut total_resolved = 0usize;
        for squad in &save.squads {
            let resolved =
                squad.player_eids.iter().filter(|e| people_by_eid.contains_key(e)).count();
            total_members += squad.player_eids.len();
            total_resolved += resolved;
            if resolved * 2 < squad.player_eids.len() {
                ragged += 1;
                if worst.len() < 15 {
                    worst.push(format!(
                        "club eid {:>7}  {resolved}/{} resolve  {:?}",
                        squad.club_eid,
                        squad.player_eids.len(),
                        name_of_club(&save, squad.club_eid)
                    ));
                }
            }
        }
        println!(
            "\n  squad members resolving to a decoded person: {total_resolved}/{total_members} ({:.2}%)",
            100.0 * total_resolved as f64 / total_members.max(1) as f64
        );
        println!("  {ragged} squads resolve under half their members (false-positive smell)");
        for line in &worst {
            println!("    {line}");
        }
        return;
    }

    // `nation:<id>` lists one nation's clubs and whether each owns a squad —
    // the question behind "this team has no players" once the link itself is
    // sound.
    if let Some(id) = club_query.strip_prefix("nation:").and_then(|s| s.parse::<u32>().ok()) {
        let referenced: HashSet<u32> = save.squads.iter().map(|s| s.club_eid).collect();
        let mut with = 0usize;
        let mut without = Vec::new();
        for club in save.clubs.iter().filter(|c| c.nation_id == id) {
            match club.eid {
                Some(eid) if referenced.contains(&eid) => with += 1,
                _ => without.push(club),
            }
        }
        println!("\n=== nation {id}: {} clubs ===", with + without.len());
        println!("  {with} own a squad");
        println!("  {} do not:", without.len());
        for club in without.iter().take(60) {
            println!("    {:?} / {:?}  eid {:?}", club.name, club.short_name, club.eid);
        }
        return;
    }

    println!("\n=== clubs matching {club_query:?}: {} ===", matches.len());
    for club in &matches {
        println!(
            "\n{:?} / {:?}  offset {}  club_id {}  nation {}  eid {:?}  uid {:?}",
            club.name, club.short_name, club.offset, club.club_id, club.nation_id, club.eid, club.uid
        );
        let Some(eid) = club.eid else {
            println!("  NO EID — the record head did not validate, so no squad can reference it");
            head_report(club.offset);
            continue;
        };
        let Some(squads) = squads_by_club.get(&eid) else {
            println!("  no squad record references club eid {eid}");
            continue;
        };
        for squad in squads {
            let total = squad.player_eids.len();
            let people = squad.player_eids.iter().filter(|e| people_by_eid.contains_key(e)).count();
            let stubs = squad.player_eids.iter().filter(|e| stub_eids.contains(e)).count();
            println!(
                "  squad at {}: {total} members — {people} people, {stubs} stubs, {} unresolved",
                squad.offset,
                total - people - stubs
            );
            for eid in squad.player_eids.iter().take(30) {
                match people_by_eid.get(eid) {
                    Some(p) => println!("    {eid:>9}  {}", p.full_name),
                    None if stub_eids.contains(eid) => println!("    {eid:>9}  (stub)"),
                    None => println!("    {eid:>9}  UNRESOLVED"),
                }
            }
        }
    }

    if let Some(query) = person_query {
        println!("\n=== people matching {query:?} ===");
        let mut found = 0;
        for person in &save.people {
            let name = person.full_name.clone();
            if !name.to_lowercase().contains(&query) {
                continue;
            }
            found += 1;
            // Which squads list this person, by eid.
            let listed: Vec<u32> = person
                .eid
                .map(|e| {
                    save.squads
                        .iter()
                        .filter(|s| s.player_eids.contains(&e))
                        .map(|s| s.club_eid)
                        .collect()
                })
                .unwrap_or_default();
            let clubs: Vec<&str> = listed
                .iter()
                .filter_map(|club_eid| {
                    save.clubs
                        .iter()
                        .find(|c| c.eid == Some(*club_eid))
                        .map(|c| c.short_name.as_str())
                })
                .collect();
            println!(
                "  {name:?}  eid {:?}  offset {}  listed by squads of {:?} {:?}",
                person.eid, person.offset, listed, clubs
            );
            if found >= 40 {
                println!("  …");
                break;
            }
        }
        if found == 0 {
            println!("  none — the person record itself is not being decoded");
        }
    }
}
