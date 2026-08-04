//! Settles which person owns a staff block: the person whose exact
//! (eid, uid) the block's object carries, or the person one eid up
//! (the binding the parser ships). Discriminates on role: pure staff carry
//! high non-player ability, players low, so blocks sandwiched between a
//! player and a member of staff vote for their real owner.
//!
//! ```text
//! cargo run --release --example staffowner -- <save.fm>
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: staffowner <save.fm>");
        std::process::exit(2);
    };
    let bytes = std::fs::read(&path)?;
    let frames = fm_save::container::read_frames(&bytes)?;
    let Some(main) = frames.iter().max_by_key(|f| f.data.len()) else {
        return Ok(());
    };
    let sheets = fm_save::staff::scan_staff(&main.data);
    let save = fm_save::Save::parse(&bytes)?;

    // person by eid, with their kind: true = player (has an ability block).
    let by_eid: std::collections::HashMap<u32, (bool, u32)> = save
        .people
        .iter()
        .filter_map(|p| Some((p.eid?, (p.ability.is_some(), p.uid?))))
        .collect();

    // Blocks where the two candidate owners have different roles.
    let mut same_player = Vec::new();
    let mut same_staff = Vec::new();
    let mut next_player = Vec::new();
    let mut next_staff = Vec::new();
    for s in &sheets {
        let Some(&(same_is_player, same_uid)) = by_eid.get(&s.eid) else {
            continue;
        };
        if same_uid != s.uid {
            continue; // only exact-pair blocks carry certain identity
        }
        let Some(&(next_is_player, _)) = by_eid.get(&s.eid.saturating_add(1)) else {
            continue;
        };
        if same_is_player == next_is_player {
            continue;
        }
        if same_is_player {
            same_player.push(s.current_ability);
        } else {
            same_staff.push(s.current_ability);
        }
        if next_is_player {
            next_player.push(s.current_ability);
        } else {
            next_staff.push(s.current_ability);
        }
    }
    let avg = |v: &[u16]| -> f64 {
        if v.is_empty() {
            return 0.0;
        }
        f64::from(v.iter().map(|&x| u32::from(x)).sum::<u32>())
            / f64::from(u32::try_from(v.len()).unwrap_or(u32::MAX))
    };
    println!("blocks where exact-pair person and eid+1 person differ in role:");
    println!(
        "  exact-pair owner is a player: {} blocks, mean block CA {:.1}",
        same_player.len(),
        avg(&same_player)
    );
    println!(
        "  exact-pair owner is staff:    {} blocks, mean block CA {:.1}",
        same_staff.len(),
        avg(&same_staff)
    );
    println!(
        "  eid+1 owner is a player:      {} blocks, mean block CA {:.1}",
        next_player.len(),
        avg(&next_player)
    );
    println!(
        "  eid+1 owner is staff:         {} blocks, mean block CA {:.1}",
        next_staff.len(),
        avg(&next_staff)
    );
    println!(
        "\nIf the exact-pair rule is right, 'owner is staff' reads high and 'owner is a player' low."
    );
    println!("If the eid+1 rule is right, the pattern follows the eid+1 rows instead.");
    Ok(())
}
