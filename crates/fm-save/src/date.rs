/// A calendar date.
///
/// FM stores dates of birth as a day-of-year plus a year rather than a
/// day/month/year triple, so converting is part of reading the format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// Bounds a plausible person in a football database. Anything outside this is
/// treated as a false positive rather than a very old player.
const MIN_YEAR: u16 = 1900;
const MAX_YEAR: u16 = 2100;

const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

impl Date {
    /// Converts a 1-based day-of-year and year into a calendar date, returning
    /// `None` when either is out of range — which is how the record scanner
    /// tells a real record from coincidental bytes.
    #[must_use]
    pub fn from_day_of_year(day_of_year: u16, year: u16) -> Option<Self> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return None;
        }
        let leap = is_leap_year(year);
        let total = if leap { 366 } else { 365 };
        if day_of_year < 1 || day_of_year > total {
            return None;
        }

        let mut remaining = day_of_year;
        for (index, base) in DAYS_IN_MONTH.iter().enumerate() {
            let len = u16::from(*base) + u16::from(index == 1 && leap);
            if remaining <= len {
                return Some(Self {
                    year,
                    month: (index + 1) as u8,
                    day: remaining as u8,
                });
            }
            remaining -= len;
        }
        None
    }

    /// Age in whole years on the given date.
    #[must_use]
    pub fn age_on(&self, on: Self) -> u16 {
        let had_birthday = (on.month, on.day) >= (self.month, self.day);
        on.year.saturating_sub(self.year).saturating_sub(u16::from(!had_birthday))
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The five players used to confirm the encoding against real DOBs.
    #[test]
    fn matches_known_players() {
        let cases = [
            (203, 2000, 21, 7, "Haaland"),
            (180, 2003, 29, 6, "Bellingham"),
            (123, 2003, 3, 5, "Wirtz"),
            (248, 2001, 5, 9, "Saka"),
            (354, 1998, 20, 12, "Mbappé"),
        ];
        for (doy, year, day, month, who) in cases {
            let d = Date::from_day_of_year(doy, year).unwrap();
            assert_eq!((d.day, d.month, d.year), (day, month, year), "{who}");
        }
    }

    #[test]
    fn handles_leap_years() {
        // 2000 is a leap year, so day 60 is 29 February.
        assert_eq!(Date::from_day_of_year(60, 2000).unwrap(), Date { year: 2000, month: 2, day: 29 });
        // 1900 is not, despite being divisible by 4.
        assert_eq!(Date::from_day_of_year(60, 1900).unwrap(), Date { year: 1900, month: 3, day: 1 });
    }

    #[test]
    fn accepts_the_first_and_last_day() {
        assert_eq!(Date::from_day_of_year(1, 2003).unwrap(), Date { year: 2003, month: 1, day: 1 });
        assert_eq!(Date::from_day_of_year(365, 2003).unwrap(), Date { year: 2003, month: 12, day: 31 });
        assert_eq!(Date::from_day_of_year(366, 2000).unwrap(), Date { year: 2000, month: 12, day: 31 });
    }

    #[test]
    fn rejects_out_of_range_values() {
        assert!(Date::from_day_of_year(0, 2003).is_none());
        assert!(Date::from_day_of_year(366, 2003).is_none(), "2003 is not a leap year");
        assert!(Date::from_day_of_year(367, 2000).is_none());
        assert!(Date::from_day_of_year(100, 1800).is_none());
    }

    #[test]
    fn computes_age_either_side_of_a_birthday() {
        let dob = Date { year: 2000, month: 7, day: 21 };
        assert_eq!(dob.age_on(Date { year: 2026, month: 7, day: 22 }), 26);
        assert_eq!(dob.age_on(Date { year: 2026, month: 7, day: 21 }), 26);
        assert_eq!(dob.age_on(Date { year: 2026, month: 7, day: 20 }), 25);
    }
}
