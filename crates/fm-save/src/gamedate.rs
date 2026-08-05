use crate::date::Date;

/// Offset of the wall-clock stamp in the header frame (`game_info.dat`).
const HEADER_WALL_CLOCK_AT: usize = 50;

/// Reads the header frame's date stamp — the **real-world** date the save was
/// written, not the in-game one.
///
/// Same `(u16 stamp, u16 year)` shape as the database frame's week stamp, low
/// nine bits carrying the day of year. It is exposed because it says when a
/// file on disk was last played, and kept separate because it was for a while
/// mistaken for the in-game date: on the 26.0.0 reference save the two happened
/// to sit weeks apart and the header's value looked plausible. Seven saves
/// settle it — the four 26.2.0 careers here stamp 1, 2 and 3 August 2026, their
/// files' own modification dates, while sitting at in-game 2026, 2030, 2032 and
/// 2035. Ages come from [`find_main_frame_date`]; nothing about a person's age
/// may be derived from this.
#[must_use]
pub fn find_wall_clock_date(header_frame: &[u8]) -> Option<Date> {
    let stamp = read_u16(header_frame, HEADER_WALL_CLOCK_AT)?;
    let year = read_u16(header_frame, HEADER_WALL_CLOCK_AT + 2)?;
    if !(2000..=2100).contains(&year) {
        return None;
    }
    Date::from_day_of_year(stamp & DOY_MASK, year)
}

/// Offset of the week-stamp date pair in the main database frame's header.
const MAIN_FRAME_DATE_AT: usize = 0x2A;

/// The day of year occupies the low nine bits of the week stamp; the high
/// seven carry something else (unknown — 0 on one save, 13 and 41 on others).
const DOY_MASK: u16 = 0x01FF;

/// Reads the in-game date from the head of the database frame.
///
/// `game_db.dat` carries a date stamp at offset `0x2A`: a u16 whose **low nine
/// bits are the day of year**, then the u16 year. The high seven bits vary per
/// save and are not understood, so they are masked off. It tracks the last
/// weekly rollover, so it can lag the true date by up to a week. Verified two
/// ways: the 2035 save's masked stamp lands four days before its known true
/// date, and an FM 26.2.0 career's masked stamp (day 159 of 2026) matches the
/// current-date stamps repeated through that save's competition frames exactly.
///
/// A days-stale date keeps every age right; falling back to the system clock
/// on an aged save shifts ages by years, which is how a player born 2012
/// showed as 14 in a 2035 save.
///
/// **Both format versions.** The stamp was once thought to be 26.2.0-only,
/// because on 26.0.0 it read a valid-looking wrong date — but that reading came
/// from the largest frame rather than `game_db.dat`, which on a long career is
/// the match-history member. Named through the manifest, the stamp is right on
/// 26.0.0 too: Adam Clouston's 26.0.0 save reads 1 July 2033, and its header
/// wall-clock stamp reads October 2025, the real-world month a 26.0.0 file was
/// last written.
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
    fn reads_the_headers_wall_clock_stamp() {
        // Career (v02).fm was written on day 299 of 2025 — 26 October 2025,
        // real-world, with its high bits clear.
        let found = find_wall_clock_date(&frame_with(299, 2025)).unwrap();
        assert_eq!(found, Date { year: 2025, month: 10, day: 26 });
    }

    #[test]
    fn masks_the_wall_clock_stamps_high_bits() {
        // Adam Clouston.fm: 0x5129 = 40 << 9 | 297, year 2025 — 24 October,
        // the real-world date, while that save sits at in-game July 2033.
        let found = find_wall_clock_date(&frame_with(0x5129, 2025)).unwrap();
        assert_eq!(found, Date { year: 2025, month: 10, day: 24 });
    }

    #[test]
    fn declines_a_wall_clock_stamp_with_no_plausible_year() {
        assert!(find_wall_clock_date(&[0u8; 256]).is_none());
    }

    #[test]
    fn tolerates_a_short_frame() {
        for len in 0..8 {
            let _ = find_wall_clock_date(&vec![0u8; len]);
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
