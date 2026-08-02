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

/// The day of year occupies the low nine bits of the week stamp; the high
/// seven carry something else (unknown — 0 on one save, 13 and 41 on others).
const DOY_MASK: u16 = 0x01FF;

/// Reads the date stamp at the head of the main database frame.
///
/// FM 26.2.0 moved the header-frame date, but the *database* frame carries a
/// date stamp at offset `0x2A`: a u16 whose **low nine bits are the day of
/// year**, then the u16 year. The high seven bits vary per save and are not
/// understood, so they are masked off. Verified two ways: the 2035 save's
/// masked stamp lands four days before its known true date (it tracks the
/// last weekly rollover, so it can lag by up to a week), and an FM 26.2.0
/// career's masked stamp (day 159 of 2026) matches the current-date stamps
/// repeated through that save's competition frames exactly.
///
/// A days-stale date keeps every age right; falling back to the system clock
/// on an aged save shifts ages by years, which is how a player born 2012
/// showed as 14 in a 2035 save.
///
/// **Only meaningful on 26.2.0-format saves.** On 26.0.0 the same offset
/// holds a different quantity whose masked value is a valid-looking but wrong
/// date (Career.fm: 18 July vs the true 26 October) — gate on
/// [`format_version`] before trusting this.
#[must_use]
pub fn find_main_frame_date(main_frame: &[u8]) -> Option<Date> {
    let stamp = read_u16(main_frame, MAIN_FRAME_DATE_AT)?;
    let year = read_u16(main_frame, MAIN_FRAME_DATE_AT + 2)?;
    if !(2000..=2100).contains(&year) {
        return None;
    }
    Date::from_day_of_year(stamp & DOY_MASK, year)
}

/// Reads the save-format version string from the header frame.
///
/// The header opens `03 01 "tad." u16`, then a length-prefixed string such as
/// `"26.0.0+0"` or `"26.2.0+0"` at offset 8. Returns `(major, minor)`; the
/// patch and build parts are ignored. Used to decide whether the main-frame
/// week stamp is trustworthy — see [`find_main_frame_date`].
#[must_use]
pub fn format_version(header_frame: &[u8]) -> Option<(u32, u32)> {
    let len = read_u32(header_frame, 8)? as usize;
    if !(5..=16).contains(&len) {
        return None;
    }
    let raw = header_frame.get(12..12usize.checked_add(len)?)?;
    let text = std::str::from_utf8(raw).ok()?;
    let mut parts = text.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    Some((major, minor))
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(s).ok()?))
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

    fn main_frame_with(stamp: u16, year: u16) -> Vec<u8> {
        let mut v = vec![0u8; MAIN_FRAME_DATE_AT];
        v.extend_from_slice(&stamp.to_le_bytes());
        v.extend_from_slice(&year.to_le_bytes());
        v.extend_from_slice(&[0u8; 16]);
        v
    }

    #[test]
    fn masks_the_week_stamps_high_bits() {
        // The Afan Lido save's real stamp: 0x1A9F = 13 << 9 | 159, year 2026.
        // Day 159 of 2026 is 8 June, confirmed by the current-date stamps
        // repeated through that save's competition frames.
        let date = find_main_frame_date(&main_frame_with(0x1A9F, 2026)).unwrap();
        assert_eq!(date, Date { year: 2026, month: 6, day: 8 });
    }

    #[test]
    fn reads_an_unmasked_stamp_as_is() {
        // The 2035 save reads 144/2035 — 24 May, four days behind its known
        // true date, which is the weekly-rollover lag.
        let date = find_main_frame_date(&main_frame_with(144, 2035)).unwrap();
        assert_eq!(date, Date { year: 2035, month: 5, day: 24 });
    }

    #[test]
    fn declines_a_stamp_with_an_out_of_range_year() {
        assert!(find_main_frame_date(&main_frame_with(144, 1899)).is_none());
        assert!(find_main_frame_date(&main_frame_with(144, 3000)).is_none());
    }

    fn header_with_version(version: &str) -> Vec<u8> {
        let mut v = vec![0x03, 0x01, b't', b'a', b'd', b'.', 0x2E, 0x00];
        v.extend_from_slice(&(u32::try_from(version.len()).unwrap()).to_le_bytes());
        v.extend_from_slice(version.as_bytes());
        v
    }

    #[test]
    fn reads_the_format_version() {
        assert_eq!(format_version(&header_with_version("26.0.0+0")), Some((26, 0)));
        assert_eq!(format_version(&header_with_version("26.2.0+0")), Some((26, 2)));
    }

    #[test]
    fn declines_a_malformed_version() {
        assert_eq!(format_version(&header_with_version("garbage!")), None);
        assert_eq!(format_version(&[0u8; 10]), None);
    }
}
