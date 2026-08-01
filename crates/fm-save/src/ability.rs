//! Player attributes, Current Ability and Potential Ability.
//!
//! These are not in the person record. FM stores a separate attribute block —
//! 54 bytes, each an attribute on the 1-20 scale multiplied by 5 — and CA/PA
//! sit just before it. The block precedes its person's name by roughly
//! 930-1100 bytes, so the block is the anchor and the person is matched to it,
//! not the other way round.
//!
//! Only players have a block; staff do not. That absence is what separates the
//! two, which nothing in the person record does.

/// Attributes per block. FM's full set across technical, mental, physical and
/// goalkeeping.
pub const ATTRIBUTE_COUNT: usize = 54;

/// Indices of the goalkeeping attributes.
///
/// Found empirically rather than assumed: at each of these, ~90% of players sit
/// at 3 or below while a minority score highly — the signature of an attribute
/// only keepers have. There are exactly 11, matching FM's goalkeeping set.
/// The remaining 43 are outfield attributes whose individual names are not yet
/// mapped, so they are presented by index rather than guessed at.
pub const GOALKEEPING_INDICES: [usize; 11] = [11, 12, 13, 14, 15, 16, 19, 21, 31, 32, 33];

/// Whether an attribute index belongs to the goalkeeping set.
#[must_use]
pub fn is_goalkeeping(index: usize) -> bool {
    GOALKEEPING_INDICES.contains(&index)
}

/// Attributes are stored on the 1-20 scale times five, so every byte is a
/// multiple of 5 in 5..=100.
const SCALE: u8 = 5;
const MIN_ATTR: u8 = 5;
const MAX_ATTR: u8 = 100;

/// Bytes back from the start of the block.
const CA_BACK: usize = 39;
const PA_BACK: usize = 37;

/// The furthest a block sits before the person it belongs to.
///
/// A block always precedes its owner, so the owner is simply the next person
/// record — the bound only rejects an orphan block with no person after it.
/// The distance is far more variable than it first appears: the median is
/// about 1,200 bytes but the 99th percentile is near 29,000, so a tight bound
/// silently drops thousands of players.
const MAX_BLOCK_TO_NAME: usize = 50_000;

/// Ability ratings are on a 1-200 scale, with 200 the hard ceiling for
/// Potential Ability.
const MAX_ABILITY: u8 = 200;

/// One player's ability data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ability {
    /// Offset of the attribute block within the frame.
    pub block_offset: usize,
    /// Current Ability, 1-200.
    pub current: u8,
    /// Potential Ability, 1-200. Never below `current`.
    pub potential: u8,
    /// The 54 attributes, converted back to the 1-20 scale FM displays.
    pub attributes: [u8; ATTRIBUTE_COUNT],
}

fn is_attribute_byte(b: u8) -> bool {
    (MIN_ATTR..=MAX_ATTR).contains(&b) && b.is_multiple_of(SCALE)
}

/// Finds every attribute block in a frame and reads the ability values in
/// front of it.
///
/// A run must be exactly [`ATTRIBUTE_COUNT`] bytes long. Shorter runs occur —
/// staff carry a smaller set — and accepting them would attach ability values
/// to the wrong field offsets.
#[must_use]
pub fn scan_abilities(frame: &[u8]) -> Vec<Ability> {
    let mut out = Vec::new();
    let mut run_start: Option<usize> = None;

    for (i, &b) in frame.iter().enumerate() {
        if is_attribute_byte(b) {
            run_start.get_or_insert(i);
            continue;
        }
        if let Some(start) = run_start.take() {
            if i - start == ATTRIBUTE_COUNT {
                if let Some(ability) = read_block(frame, start) {
                    out.push(ability);
                }
            }
        }
    }

    out
}

fn read_block(frame: &[u8], start: usize) -> Option<Ability> {
    let current = *frame.get(start.checked_sub(CA_BACK)?)?;
    let potential = *frame.get(start.checked_sub(PA_BACK)?)?;

    // Both are 1-200 and a player can never exceed their own ceiling. A block
    // failing this is a false positive, not a player with odd data.
    if current == 0 || current > MAX_ABILITY || potential > MAX_ABILITY || potential < current {
        return None;
    }

    let raw = frame.get(start..start + ATTRIBUTE_COUNT)?;
    let mut attributes = [0u8; ATTRIBUTE_COUNT];
    for (slot, &b) in attributes.iter_mut().zip(raw) {
        *slot = b / SCALE;
    }

    Some(Ability {
        block_offset: start,
        current,
        potential,
        attributes,
    })
}

/// Matches each attribute block to the person it belongs to.
///
/// `people_offsets` must be the sorted record offsets of the people in the same
/// frame. Returns, for each ability, the index into that slice — the first
/// person starting after the block.
///
/// More than one block can resolve to the same person, because the person scan
/// does not find every record. Where that happens the nearest block wins:
/// letting a distant one overwrite a close one reassigns ability between
/// players, which is silently wrong rather than obviously broken.
#[must_use]
pub fn match_to_people(abilities: &[Ability], people_offsets: &[usize]) -> Vec<Option<usize>> {
    // Best (smallest) distance seen so far for each person index.
    let mut claimed: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut proposals: Vec<Option<(usize, usize)>> = Vec::with_capacity(abilities.len());

    for a in abilities {
        let idx = people_offsets.partition_point(|&o| o <= a.block_offset);
        let proposal = people_offsets.get(idx).and_then(|offset| {
            let distance = offset.saturating_sub(a.block_offset);
            (distance <= MAX_BLOCK_TO_NAME).then_some((idx, distance))
        });
        if let Some((person, distance)) = proposal {
            claimed
                .entry(person)
                .and_modify(|best| *best = (*best).min(distance))
                .or_insert(distance);
        }
        proposals.push(proposal);
    }

    proposals
        .into_iter()
        .map(|p| {
            let (person, distance) = p?;
            (claimed.get(&person) == Some(&distance)).then_some(person)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Builds a block with the ability bytes in front of it, matching the
    /// on-disk layout.
    fn block(current: u8, potential: u8, attrs: [u8; ATTRIBUTE_COUNT]) -> Vec<u8> {
        let mut v = vec![0u8; 64];
        let start = v.len();
        *v.get_mut(start - CA_BACK).unwrap() = current;
        *v.get_mut(start - PA_BACK).unwrap() = potential;
        v.extend(attrs.iter().map(|a| a * SCALE));
        // Terminate the run so the scanner sees its end.
        v.push(0xFF);
        v
    }

    fn flat(value: u8) -> [u8; ATTRIBUTE_COUNT] {
        [value; ATTRIBUTE_COUNT]
    }

    #[test]
    fn reads_current_and_potential_ability() {
        // Haaland's real values in the reference save.
        let buf = block(184, 195, flat(14));
        let found = scan_abilities(&buf);
        assert_eq!(found.len(), 1);
        let a = found.first().unwrap();
        assert_eq!((a.current, a.potential), (184, 195));
    }

    #[test]
    fn converts_attributes_back_to_the_twenty_point_scale() {
        let mut attrs = flat(10);
        attrs[0] = 20;
        attrs[1] = 1;
        let found = scan_abilities(&block(150, 170, attrs));
        let a = found.first().unwrap();
        assert_eq!(a.attributes[0], 20, "100 on disk is 20 displayed");
        assert_eq!(a.attributes[1], 1, "5 on disk is 1 displayed");
        assert_eq!(a.attributes.len(), ATTRIBUTE_COUNT);
    }

    #[test]
    fn rejects_a_block_whose_potential_is_below_its_current() {
        // Impossible in FM, so the run is not a real attribute block.
        assert!(scan_abilities(&block(180, 120, flat(12))).is_empty());
    }

    #[test]
    fn rejects_ability_above_the_two_hundred_ceiling() {
        assert!(scan_abilities(&block(210, 220, flat(12))).is_empty());
    }

    #[test]
    fn ignores_runs_that_are_not_exactly_the_attribute_count() {
        // Staff carry shorter runs; treating them as blocks would read ability
        // from the wrong offsets.
        let mut v = vec![0u8; 64];
        v.extend(std::iter::repeat_n(50u8, ATTRIBUTE_COUNT - 10));
        v.push(0xFF);
        assert!(scan_abilities(&v).is_empty());
    }

    #[test]
    fn matches_a_block_to_the_person_that_follows_it() {
        let abilities = scan_abilities(&block(184, 195, flat(14)));
        let a = abilities.first().unwrap();
        // One person just after the block, one far away.
        let offsets = vec![a.block_offset + 950, a.block_offset + 900_000];
        assert_eq!(match_to_people(&abilities, &offsets), vec![Some(0)]);
    }

    #[test]
    fn leaves_a_block_unmatched_when_no_person_follows_it() {
        let abilities = scan_abilities(&block(184, 195, flat(14)));
        let a = abilities.first().unwrap();
        // Far beyond any plausible record, and nothing after it.
        let offsets = vec![a.block_offset + MAX_BLOCK_TO_NAME + 1];
        assert_eq!(match_to_people(&abilities, &offsets), vec![None]);
    }

    #[test]
    fn the_nearest_block_wins_when_two_resolve_to_one_person() {
        // Two blocks, one person after both. Letting the far block overwrite
        // the near one would attribute the wrong player's ability.
        let mut buf = block(120, 140, flat(9));
        let far_offset = scan_abilities(&buf).first().unwrap().block_offset;
        buf.extend(std::iter::repeat_n(0u8, 400));
        buf.extend(block(184, 195, flat(14)));

        let abilities = scan_abilities(&buf);
        assert_eq!(abilities.len(), 2, "expected two blocks");
        let near = abilities.get(1).unwrap().block_offset;
        assert!(near > far_offset);

        let person = near + 900;
        let matched = match_to_people(&abilities, &[person]);
        assert_eq!(matched, vec![None, Some(0)], "the closer block should own the person");
    }

    #[test]
    fn tolerates_a_truncated_buffer() {
        let full = block(184, 195, flat(14));
        for cut in 0..full.len() {
            let _ = scan_abilities(full.get(..cut).unwrap());
        }
    }
}
