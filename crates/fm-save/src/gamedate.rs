use crate::date::Date;

/// Finds the save's in-game date in the header frame.
///
/// This matters for ages: a save at 26 October 2025 read on 1 August 2026
/// reports everyone a year too old if the system clock is used instead.
///
/// The date is a `(u16 day_of_year, u16 year)` pair, the same encoding as a
/// date of birth. In the FM 26.0.0 reference save exactly one such pair occurs
/// in the header frame, at offset 50, which makes a validated scan reliable
/// rather than a guess at a fixed offset.
///
/// Returns `None` when no unambiguous date is present — FM 26.2.0 saves encode
/// it differently and are not yet understood, and a wrong date is worse than an
/// absent one because it silently shifts every age.
#[must_use]
pub fn find_game_date(header_frame: &[u8]) -> Option<Date> {
    let mut found: Option<Date> = None;

    let mut at = 0usize;
    while at + 4 <= header_frame.len() {
        let doy = read_u16(header_frame, at)?;
        let year = read_u16(header_frame, at + 2)?;
        // A save's own date sits within the era FM models, which is much
        // tighter than the range allowed for a date of birth.
        if (2000..=2100).contains(&year) {
            if let Some(date) = Date::from_day_of_year(doy, year) {
                match found {
                    // More than one candidate means the signature is not
                    // unique in this save, so decline rather than pick one.
                    Some(existing) if existing != date => return None,
                    Some(_) => {}
                    None => found = Some(date),
                }
            }
        }
        at += 2;
    }

    found
}

/// Offset of the week-stamp date pair in the main database frame's header.
const MAIN_FRAME_DATE_AT: usize = 0x2A;

/// Reads the date stamp at the head of the main database frame.
///
/// FM 26.2.0 moved the header-frame date, but the *database* frame carries a
/// date pair at offset `0x2A` on every version surveyed — nine saves across
/// 26.0.0 and 26.2.0, all reading within days of the save's true date (it
/// tracks the last weekly rollover, so it can lag by up to a week). A
/// days-stale date keeps every age right; falling back to the system clock
/// on an aged save shifts ages by years, which is how a player born 2012
/// showed as 14 in a 2035 save.
#[must_use]
pub fn find_main_frame_date(main_frame: &[u8]) -> Option<Date> {
    let doy = read_u16(main_frame, MAIN_FRAME_DATE_AT)?;
    let year = read_u16(main_frame, MAIN_FRAME_DATE_AT + 2)?;
    if !(2000..=2100).contains(&year) {
        return None;
    }
    Date::from_day_of_year(doy, year)
}

fn read_u16(b: &[u8], at: usize) -> Option<u16> {
    let s = b.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(<[u8; 2]>::try_from(s).ok()?))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn frame_with(doy: u16, year: u16) -> Vec<u8> {
        let mut v = vec![0u8; 50];
        v.extend_from_slice(&doy.to_le_bytes());
        v.extend_from_slice(&year.to_le_bytes());
        v.extend_from_slice(&[0u8; 40]);
        v
    }

    #[test]
    fn finds_the_reference_saves_date() {
        // Career.fm sits at day 299 of 2025 — 26 October 2025.
        let found = find_game_date(&frame_with(299, 2025)).unwrap();
        assert_eq!(found, Date { year: 2025, month: 10, day: 26 });
    }

    #[test]
    fn declines_when_no_date_is_present() {
        assert!(find_game_date(&[0u8; 256]).is_none());
    }

    #[test]
    fn declines_when_two_different_dates_are_candidates() {
        // Ambiguity means the signature is not unique; guessing would silently
        // shift every age in the save.
        let mut v = frame_with(299, 2025);
        v.extend_from_slice(&100u16.to_le_bytes());
        v.extend_from_slice(&2030u16.to_le_bytes());
        assert!(find_game_date(&v).is_none());
    }

    #[test]
    fn tolerates_a_short_frame() {
        for len in 0..8 {
            let _ = find_game_date(&vec![0u8; len]);
        }
    }
}
