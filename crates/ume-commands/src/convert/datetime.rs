//! The date and the time that `F_INS_DATE` and `F_INS_TIME` insert, and the clock they read.
//!
//! Sakura formats with the Windows API `GetDateFormat` / `GetTimeFormat`
//! (`CFormatManager::MyGetDateFormat`, `MyGetTimeFormat` in `sakura_core/env/CFormatManager.cpp`)
//! and a *picture* string such as `yyyy'年'M'月'd'日('dddd')'`. This module implements the
//! pictures with the names of the Japanese (ja-JP) locale, which is what Sakura's default
//! pictures are written for. The names of the weekdays and the half days of other locales are
//! not implemented.
//!
//! Supported letters. Date: `d dd ddd dddd` (day, weekday), `M MM MMM MMMM` (month), `y yy yyyy`
//! (year). Time: `h hh` (12-hour clock), `H HH`, `m mm`, `s ss`, `t tt` (午前 / 午後). Text between
//! single quotes is copied (`''` is a quote); any other char is copied. The letters of the
//! other kind, and the era `g`, are copied like any other char.

use chrono::{Datelike, Timelike};

/// Sakura's default date picture (`m_szDateFormat`, `CShareData::InitShareData`, and
/// `STR_DATA_FORMAT` in `sakura_core/sakura_rc.rc`).
pub const DEFAULT_DATE_PICTURE: &str = "yyyy'年'M'月'd'日('dddd')'";

/// Sakura's default time picture (`m_szTimeFormat`, `STR_TIME_FORMAT`).
pub const DEFAULT_TIME_PICTURE: &str = "tthh'時'mm'分'ss'秒'";

/// A date and a time of day on a wall clock, without a time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    /// 1-12.
    pub month: u32,
    /// 1-31.
    pub day: u32,
    /// 0 is Sunday.
    pub weekday: u32,
    /// 0-23.
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Where the commands get the time from. The commands use [`SystemClock`]; tests give their own.
pub trait Clock {
    /// The current local time (Sakura: `GetLocalTime`).
    fn now(&self) -> LocalTime;
}

/// The clock of the computer, in the time zone of the computer.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> LocalTime {
        let t = chrono::Local::now();
        LocalTime {
            year: t.year(),
            month: t.month(),
            day: t.day(),
            weekday: t.weekday().num_days_from_sunday(),
            hour: t.hour(),
            minute: t.minute(),
            second: t.second(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Date,
    Time,
}

/// The date by a picture like `GetDateFormat`'s (see the [module documentation](self)).
pub fn format_date(t: &LocalTime, picture: &str) -> String {
    format_picture(t, picture, Kind::Date)
}

/// The time of day by a picture like `GetTimeFormat`'s.
pub fn format_time(t: &LocalTime, picture: &str) -> String {
    format_picture(t, picture, Kind::Time)
}

const WEEKDAYS: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];

fn format_picture(t: &LocalTime, picture: &str, kind: Kind) -> String {
    let chars: Vec<char> = picture.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' && chars.get(i + 1) == Some(&'\'') {
            // two quotes outside quoted text are one quote
            out.push('\'');
            i += 2;
            continue;
        }
        if c == '\'' {
            // quoted text up to the closing quote; two quotes in it are one quote
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if chars.get(i + 1) == Some(&'\'') {
                        out.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        let run = chars[i..].iter().take_while(|&&x| x == c).count();
        i += run;
        match field(t, kind, c, run) {
            Some(text) => out.push_str(&text),
            None => out.extend(std::iter::repeat_n(c, run)),
        }
    }
    out
}

/// The text of the field made of `run` times the letter `c`, or `None` if it is not a field.
fn field(t: &LocalTime, kind: Kind, c: char, run: usize) -> Option<String> {
    let weekday = WEEKDAYS[(t.weekday % 7) as usize];
    let hour12 = if t.hour.is_multiple_of(12) {
        12
    } else {
        t.hour % 12
    };
    // two digits when the letter is doubled
    let n = |value: u32| {
        if run >= 2 {
            format!("{value:02}")
        } else {
            value.to_string()
        }
    };
    Some(match (kind, c) {
        (Kind::Date, 'd') => match run {
            1 | 2 => n(t.day),
            3 => weekday.to_string(),
            _ => format!("{weekday}曜日"),
        },
        (Kind::Date, 'M') => match run {
            1 | 2 => n(t.month),
            _ => format!("{}月", t.month),
        },
        (Kind::Date, 'y') => match run {
            // the last two digits, with a leading zero for `yy`
            1 | 2 => n(t.year.rem_euclid(100) as u32),
            _ => format!("{:04}", t.year),
        },
        (Kind::Time, 'h') => n(hour12),
        (Kind::Time, 'H') => n(t.hour),
        (Kind::Time, 'm') => n(t.minute),
        (Kind::Time, 's') => n(t.second),
        (Kind::Time, 't') if run == 1 => "午".to_string(),
        (Kind::Time, 't') if t.hour < 12 => "午前".to_string(),
        (Kind::Time, 't') => "午後".to_string(),
        _ => return None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A clock that stands still.
    pub struct FixedClock(pub LocalTime);

    impl Clock for FixedClock {
        fn now(&self) -> LocalTime {
            self.0
        }
    }

    /// Thursday, 2026-10-15 09:05:07.
    pub fn thursday_morning() -> LocalTime {
        LocalTime {
            year: 2026,
            month: 10,
            day: 15,
            weekday: 4,
            hour: 9,
            minute: 5,
            second: 7,
        }
    }

    fn at(hour: u32) -> LocalTime {
        LocalTime {
            hour,
            ..thursday_morning()
        }
    }

    #[test]
    fn the_default_pictures() {
        // Sakura's default pictures (CShareData.cpp), with ja-JP names
        assert_eq!(
            format_date(&thursday_morning(), DEFAULT_DATE_PICTURE),
            "2026年10月15日(木曜日)"
        );
        assert_eq!(
            format_time(&thursday_morning(), DEFAULT_TIME_PICTURE),
            "午前09時05分07秒"
        );
        assert_eq!(
            format_time(&at(15), DEFAULT_TIME_PICTURE),
            "午後03時05分07秒"
        );
    }

    #[test]
    fn date_letters() {
        let t = thursday_morning();
        assert_eq!(format_date(&t, "d dd ddd dddd"), "15 15 木 木曜日");
        assert_eq!(format_date(&t, "M MM MMM MMMM"), "10 10 10月 10月");
        assert_eq!(format_date(&t, "y yy yyyy"), "26 26 2026");
        let early = LocalTime {
            year: 2005,
            month: 3,
            day: 4,
            weekday: 0,
            ..t
        };
        assert_eq!(
            format_date(&early, "d dd M MM y yy yyyy"),
            "4 04 3 03 5 05 2005"
        );
        assert_eq!(format_date(&early, "dddd"), "日曜日");
        assert_eq!(format_date(&early, "yyyy/MM/dd"), "2005/03/04");
    }

    #[test]
    fn time_letters() {
        let t = thursday_morning();
        assert_eq!(
            format_time(&t, "h hh H HH m mm s ss"),
            "9 09 9 09 5 05 7 07"
        );
        assert_eq!(format_time(&at(0), "h hh H HH"), "12 12 0 00");
        assert_eq!(format_time(&at(12), "h t tt"), "12 午 午後");
        assert_eq!(format_time(&at(23), "h HH tt"), "11 23 午後");
        assert_eq!(format_time(&at(11), "tt"), "午前");
    }

    #[test]
    fn quotes_and_other_chars() {
        let t = thursday_morning();
        assert_eq!(format_date(&t, "yyyy'-'MM'-'dd"), "2026-10-15");
        // letters in quotes are text, and two quotes are one quote
        assert_eq!(format_date(&t, "'yyyy it''s' d"), "yyyy it's 15");
        // two quotes outside quoted text are a quote too
        assert_eq!(format_time(&t, "HH''mm"), "09'05");
        // the letters of the other kind and unknown letters are copied
        assert_eq!(format_date(&t, "h:m g"), "h:m g");
        assert_eq!(format_time(&t, "d/M H:mm"), "d/M 9:05");
        // an open quote runs to the end
        assert_eq!(format_date(&t, "d 'abc"), "15 abc");
        assert_eq!(format_date(&t, ""), "");
    }

    #[test]
    fn the_system_clock_gives_a_valid_time() {
        let t = SystemClock.now();
        assert!((1..=12).contains(&t.month));
        assert!((1..=31).contains(&t.day));
        assert!(t.weekday < 7);
        assert!(t.hour < 24 && t.minute < 60 && t.second < 62);
        assert!(t.year >= 2026);
    }
}
