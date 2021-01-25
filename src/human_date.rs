use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::mem;

use todo_lib::{terr, tfilter};

const NO_CHANGE: &str = "no change";
const DAYS_PER_WEEK: u32 = 7;
const FAR_PAST: i64 = -100 * 365; // far in the past

type HumanResult = Result<NaiveDate, String>;

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if y % 4 == 0 {
                if y % 100 == 0 && y % 400 != 0 {
                    28
                } else {
                    29
                }
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn abs_time_diff(base: NaiveDate, human: &str, back: bool) -> HumanResult {
    let mut num = 0u32;
    let mut dt = base;

    for c in human.chars() {
        match c.to_digit(10) {
            None => {
                if num != 0 {
                    match c {
                        'd' => {
                            let dur = if back { Duration::days(-(num as i64)) } else { Duration::days(num as i64) };
                            dt += dur;
                        }
                        'w' => {
                            let dur = if back { Duration::weeks(-(num as i64)) } else { Duration::weeks(num as i64) };
                            dt += dur;
                        }
                        'm' => {
                            let mut y = dt.year();
                            let mut m = dt.month();
                            let mut d = dt.day();
                            let mxd = days_in_month(y, m);
                            if back {
                                let full_years = num / 12;
                                let num = num % 12;
                                y -= full_years as i32;
                                m = if m > num {
                                    m - num
                                } else {
                                    y -= 1;
                                    m + 12 - num
                                };
                            } else {
                                m += num;
                                if m > 12 {
                                    m -= 1;
                                    y += (m / 12) as i32;
                                    m = (m % 12) + 1;
                                }
                            }
                            let new_mxd = days_in_month(y, m);
                            if mxd > d || d == mxd {
                                if d == mxd || new_mxd < d {
                                    d = new_mxd
                                }
                                dt = NaiveDate::from_ymd(y as i32, m as u32, d as u32);
                            } else {
                                dt = NaiveDate::from_ymd(y as i32, m as u32, new_mxd as u32);
                            }
                        }
                        'y' => {
                            let mut y = dt.year();
                            let m = dt.month();
                            let mut d = dt.day();
                            let mxd = days_in_month(y, m);
                            if back {
                                y -= num as i32;
                            } else {
                                y += num as i32;
                            };
                            let new_mxd = days_in_month(y, m);
                            if mxd > d || d == mxd {
                                if new_mxd < d || d == mxd {
                                    d = new_mxd;
                                }
                                dt = NaiveDate::from_ymd(y as i32, m as u32, d as u32);
                            } else {
                                dt = NaiveDate::from_ymd(y as i32, m as u32, new_mxd as u32);
                            }
                        }
                        _ => {}
                    }
                    num = 0;
                }
            }
            Some(i) => num = num * 10 + i,
        }
    }
    if base == dt {
        // bad due date
        return Err(format!("invalid date '{}'", human));
    }
    Ok(dt)
}

fn next_weekday(base: NaiveDate, wd: Weekday) -> HumanResult {
    let base_wd = base.weekday();
    let (bn, wn) = (base_wd.number_from_monday(), wd.number_from_monday());
    if bn < wn {
        // this week
        Ok(base + Duration::days((wn - bn) as i64))
    } else {
        // next week
        Ok(base + Duration::days((DAYS_PER_WEEK + wn - bn) as i64))
    }
}

fn prev_weekday(base: NaiveDate, wd: Weekday) -> HumanResult {
    let base_wd = base.weekday();
    let (bn, wn) = (base_wd.number_from_monday(), wd.number_from_monday());
    if bn > wn {
        // this week
        Ok(base - Duration::days(bn as i64 - wn as i64))
    } else {
        // week before
        Ok(base + Duration::days(wn as i64 - bn as i64 - DAYS_PER_WEEK as i64))
    }
}

fn day_of_first_month(base: NaiveDate, human: &str) -> HumanResult {
    match human.parse::<u32>() {
        Err(e) => Err(format!("invalid day of month: {:?}", e)),
        Ok(n) => {
            if n == 0 || n > 31 {
                Err(format!("Day number too big: {}", n))
            } else {
                let mut m = base.month();
                let mut y = base.year();
                let mut d = base.day();
                let bdays = days_in_month(y, m);
                if d >= n {
                    if m == 12 {
                        m = 1;
                        y += 1;
                    } else {
                        m += 1;
                    }
                }
                d = if n >= days_in_month(y, m) || n >= bdays { days_in_month(y, m) } else { n };
                Ok(NaiveDate::from_ymd(y, m, d))
            }
        }
    }
}

fn no_year_date(base: NaiveDate, human: &str) -> HumanResult {
    let parts: Vec<_> = human.split('-').collect();
    if parts.len() != 2 {
        return Err("expected date in format MONTH-DAY".to_string());
    }
    let y = base.year();
    let m = match parts[0].parse::<u32>() {
        Err(_) => return Err(format!("invalid month number: {}", parts[0])),
        Ok(n) => {
            if !(1..=12).contains(&n) {
                return Err(format!("month number must be between 1 and 12 ({})", n));
            }
            n
        }
    };
    let d = match parts[1].parse::<u32>() {
        Err(_) => return Err(format!("invalid day number: {}", parts[1])),
        Ok(n) => {
            if !(1..=31).contains(&n) {
                return Err(format!("day number must be between 1 and 31 ({})", n));
            }
            let mx = days_in_month(y, m);
            if n > mx {
                mx
            } else {
                n
            }
        }
    };
    let dt = NaiveDate::from_ymd(y, m, d);
    if dt < base {
        let y = y + 1;
        let mx = days_in_month(y, m);
        let d = if mx < d { mx } else { d };
        Ok(NaiveDate::from_ymd(y, m, d))
    } else {
        Ok(dt)
    }
}

// Returns if a special day is always either in the future or in the past. E.g., `today` cannot be in
// the past and `yesterday` cannot be in the future, so the function returns `true` for both.
fn is_absolute(name: &str) -> bool {
    matches!(name, "today" | "tomorrow" | "tmr" | "tm" | "yesterday" | "overdue")
}

fn special_time_point(base: NaiveDate, human: &str, back: bool, soon_days: u8) -> HumanResult {
    let s = human.replace(&['-', '_'][..], "").to_lowercase();
    if back && is_absolute(human) {
        return Err(format!("'{}' cannot be back", human));
    }
    match s.as_str() {
        "today" => Ok(base),
        "tomorrow" | "tmr" | "tm" => Ok(base.succ()),
        "yesterday" => Ok(base.pred()),
        "overdue" => Ok(base + Duration::days(FAR_PAST)),
        "soon" => {
            let dur = Duration::days(soon_days as i64);
            Ok(if back { base - dur } else { base + dur })
        }
        "first" => {
            let mut y = base.year();
            let mut m = base.month();
            let d = base.day();
            if !back {
                if m < 12 {
                    m += 1;
                } else {
                    y += 1;
                    m = 1;
                }
            } else if d == 1 {
                if m == 1 {
                    m = 12;
                    y -= 1;
                } else {
                    m -= 1;
                }
            }
            Ok(NaiveDate::from_ymd(y, m, 1))
        }
        "last" => {
            let mut y = base.year();
            let mut m = base.month();
            let mut d = base.day();
            let last_day = days_in_month(y, m);
            if back {
                if m == 1 {
                    m = 12;
                    y -= 1;
                } else {
                    m -= 1;
                }
            } else if d == last_day {
                if m < 12 {
                    m += 1;
                } else {
                    m = 1;
                    y += 1;
                }
            }
            d = days_in_month(y, m);
            Ok(NaiveDate::from_ymd(y, m, d))
        }
        "monday" | "mon" | "mo" => {
            if back {
                prev_weekday(base, Weekday::Mon)
            } else {
                next_weekday(base, Weekday::Mon)
            }
        }
        "tuesday" | "tue" | "tu" => {
            if back {
                prev_weekday(base, Weekday::Tue)
            } else {
                next_weekday(base, Weekday::Tue)
            }
        }
        "wednesday" | "wed" | "we" => {
            if back {
                prev_weekday(base, Weekday::Wed)
            } else {
                next_weekday(base, Weekday::Wed)
            }
        }
        "thursday" | "thu" | "th" => {
            if back {
                prev_weekday(base, Weekday::Thu)
            } else {
                next_weekday(base, Weekday::Thu)
            }
        }
        "friday" | "fri" | "fr" => {
            if back {
                prev_weekday(base, Weekday::Fri)
            } else {
                next_weekday(base, Weekday::Fri)
            }
        }
        "saturday" | "sat" | "sa" => {
            if back {
                prev_weekday(base, Weekday::Sat)
            } else {
                next_weekday(base, Weekday::Sat)
            }
        }
        "sunday" | "sun" | "su" => {
            if back {
                prev_weekday(base, Weekday::Sun)
            } else {
                next_weekday(base, Weekday::Sun)
            }
        }
        _ => Err(format!("invalid date '{}'", human)),
    }
}

// Converts human-readable date to an absolute date in todo-txt format. If the date is already an
// absolute value, the function returns None. In case of any error None is returned as well.
pub fn human_to_date(base: NaiveDate, human: &str, soon_days: u8) -> HumanResult {
    if human.is_empty() {
        return Err("empty date".to_string());
    }
    let back = human.starts_with('-');
    let human = if back { &human[1..] } else { human };

    if human.find(|c: char| !('0'..='9').contains(&c)).is_none() {
        if back {
            return Err("negative day of month".to_string());
        }
        return day_of_first_month(base, human);
    }
    if human.find(|c: char| !('0'..='9').contains(&c) && c != '-').is_none() {
        if back {
            return Err("negative absolute date".to_string());
        }
        if human.matches('-').count() == 1 {
            // month-day case
            return no_year_date(base, human);
        }
        // normal date, nothing to fix
        return Err(NO_CHANGE.to_string());
    }
    if human.find(|c: char| c < '0' || (c > '9' && c != 'd' && c != 'm' && c != 'w' && c != 'y')).is_none() {
        return abs_time_diff(base, human, back);
    }

    // some "special" word like "tomorrow", "tue"
    special_time_point(base, human, back, soon_days)
}

pub fn fix_date(base: NaiveDate, orig: &str, look_for: &str, soon_days: u8) -> Option<String> {
    if orig.is_empty() || look_for.is_empty() {
        return None;
    }
    let spaced = " ".to_string() + look_for;
    let start = if orig.starts_with(look_for) {
        0
    } else if let Some(p) = orig.find(&spaced) {
        p + " ".len()
    } else {
        return None;
    };
    let substr = &orig[start + look_for.len()..];
    let human = if let Some(p) = substr.find(' ') { &substr[..p] } else { &substr };
    match human_to_date(base, human, soon_days) {
        Err(e) => {
            if e != NO_CHANGE {
                eprintln!("invalid due date: {}", human);
            }
            None
        }
        Ok(new_date) => {
            let what = look_for.to_string() + human;
            let with = look_for.to_string() + new_date.format("%Y-%m-%d").to_string().as_str();
            Some(orig.replace(what.as_str(), with.as_str()))
        }
    }
}

pub(crate) fn is_range_with_none(human: &str) -> bool {
    if !is_range(human) {
        return false;
    }
    human.starts_with("none..") || human.ends_with("..none") || human.starts_with("none:") || human.ends_with(":none")
}

pub(crate) fn human_to_range_with_none(
    base: NaiveDate,
    human: &str,
    soon_days: u8,
) -> Result<tfilter::DateRange, terr::TodoError> {
    let parts: Vec<&str> = if human.find(':') == None {
        human.split("..").filter(|s| !s.is_empty()).collect()
    } else {
        human.split(':').filter(|s| !s.is_empty()).collect()
    };
    if parts.len() > 2 {
        return Err(range_error(human));
    }
    if parts[1] == "none" {
        match human_to_date(base, parts[0], soon_days) {
            Err(e) => Err(range_error(&e)),
            Ok(d) => Ok(tfilter::DateRange {
                days: tfilter::ValueRange { low: tfilter::INCLUDE_NONE, high: (d - base).num_days() },
                span: tfilter::ValueSpan::Higher,
            }),
        }
    } else if parts[0] == "none" {
        match human_to_date(base, parts[1], soon_days) {
            Err(e) => Err(range_error(&e)),
            Ok(d) => Ok(tfilter::DateRange {
                days: tfilter::ValueRange { high: tfilter::INCLUDE_NONE, low: (d - base).num_days() },
                span: tfilter::ValueSpan::Lower,
            }),
        }
    } else {
        Err(range_error(human))
    }
}

pub(crate) fn is_range(human: &str) -> bool {
    human.find("..") != None || human.find(':') != None
}

fn range_error(msg: &str) -> terr::TodoError {
    terr::TodoError::from(terr::TodoErrorKind::InvalidValue { value: msg.to_string(), name: "date range".to_string() })
}

pub(crate) fn human_to_range(
    base: NaiveDate,
    human: &str,
    soon_days: u8,
) -> Result<tfilter::DateRange, terr::TodoError> {
    let parts: Vec<&str> = if human.find(':') == None {
        human.split("..").filter(|s| !s.is_empty()).collect()
    } else {
        human.split(':').filter(|s| !s.is_empty()).collect()
    };
    if parts.len() > 2 {
        return Err(range_error(human));
    }
    let left_open = human.starts_with(':') || human.starts_with("..");
    if parts.len() == 2 {
        let mut begin = match human_to_date(base, parts[0], soon_days) {
            Ok(d) => d,
            Err(e) => return Err(range_error(&e)),
        };
        let mut end = match human_to_date(base, parts[1], soon_days) {
            Ok(d) => d,
            Err(e) => return Err(range_error(&e)),
        };
        if begin > end {
            mem::swap(&mut begin, &mut end);
        }
        return Ok(tfilter::DateRange {
            days: tfilter::ValueRange { low: (begin - base).num_days(), high: (end - base).num_days() },
            span: tfilter::ValueSpan::Range,
        });
    }
    if left_open {
        let end = match human_to_date(base, parts[0], soon_days) {
            Ok(d) => d,
            Err(e) => return Err(range_error(&e)),
        };
        let diff = (end - base).num_days() + 1;
        return Ok(tfilter::DateRange {
            days: tfilter::ValueRange { low: diff, high: 0 },
            span: tfilter::ValueSpan::Lower,
        });
    }
    match human_to_date(base, parts[0], soon_days) {
        Ok(begin) => {
            let diff = (begin - base).num_days() - 1;
            Ok(tfilter::DateRange {
                days: tfilter::ValueRange { low: 0, high: diff },
                span: tfilter::ValueSpan::Higher,
            })
        }
        Err(e) => Err(range_error(&e)),
    }
}

#[cfg(test)]
mod humandate_test {
    use super::*;
    use chrono::Local;

    struct Test {
        txt: &'static str,
        val: NaiveDate,
    }
    struct TestRange {
        txt: &'static str,
        val: tfilter::DateRange,
    }

    #[test]
    fn no_change() {
        let dt = Local::now().date().naive_local();
        let res = human_to_date(dt, "2010-10-10", 0);
        let must = Err(NO_CHANGE.to_string());
        assert_eq!(res, must)
    }

    #[test]
    fn month_day() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let tests: Vec<Test> = vec![
            Test { txt: "7", val: NaiveDate::from_ymd(2020, 8, 7) },
            Test { txt: "11", val: NaiveDate::from_ymd(2020, 7, 11) },
            Test { txt: "31", val: NaiveDate::from_ymd(2020, 7, 31) },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }

        let dt = NaiveDate::from_ymd(2020, 6, 9);
        let nm = human_to_date(dt, "31", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 6, 30)));
        let dt = NaiveDate::from_ymd(2020, 2, 4);
        let nm = human_to_date(dt, "31", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 2, 29)));
        let dt = NaiveDate::from_ymd(2020, 2, 29);
        let nm = human_to_date(dt, "29", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 3, 31)));

        let nm = human_to_date(dt, "32", 0);
        assert!(nm.is_err());
        let nm = human_to_date(dt, "0", 0);
        assert!(nm.is_err());
    }

    #[test]
    fn month_and_day() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let nm = human_to_date(dt, "07-08", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2021, 7, 8)));
        let nm = human_to_date(dt, "07-11", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 7, 11)));
        let nm = human_to_date(dt, "02-31", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2021, 2, 28)));
    }

    #[test]
    fn absolute() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let tests: Vec<Test> = vec![
            Test { txt: "1w", val: NaiveDate::from_ymd(2020, 7, 16) },
            Test { txt: "3d4d", val: NaiveDate::from_ymd(2020, 7, 16) },
            Test { txt: "1y", val: NaiveDate::from_ymd(2021, 7, 9) },
            Test { txt: "2w2d1m", val: NaiveDate::from_ymd(2020, 8, 25) },
            Test { txt: "-1w", val: NaiveDate::from_ymd(2020, 7, 2) },
            Test { txt: "-3d4d", val: NaiveDate::from_ymd(2020, 7, 2) },
            Test { txt: "-1y", val: NaiveDate::from_ymd(2019, 7, 9) },
            Test { txt: "-2w2d1m", val: NaiveDate::from_ymd(2020, 5, 23) },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }

        let dt = NaiveDate::from_ymd(2021, 2, 28);
        let tests: Vec<Test> = vec![
            Test { txt: "1m", val: NaiveDate::from_ymd(2021, 3, 31) },
            Test { txt: "1y", val: NaiveDate::from_ymd(2022, 2, 28) },
            Test { txt: "3y", val: NaiveDate::from_ymd(2024, 2, 29) },
            Test { txt: "-1m", val: NaiveDate::from_ymd(2021, 1, 31) },
            Test { txt: "-1y", val: NaiveDate::from_ymd(2020, 2, 29) },
            Test { txt: "-3y", val: NaiveDate::from_ymd(2018, 2, 28) },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }
    }

    #[test]
    fn special() {
        let dt = NaiveDate::from_ymd(2020, 2, 29);
        let nm = human_to_date(dt, "last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 3, 31)));
        let nm = human_to_date(dt, "-last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 1, 31)));

        let dt = NaiveDate::from_ymd(2020, 2, 10);
        let nm = human_to_date(dt, "last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 2, 29)));
        let nm = human_to_date(dt, "-last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 1, 31)));

        let dt = NaiveDate::from_ymd(2020, 2, 1);
        let nm = human_to_date(dt, "first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 3, 1)));
        let nm = human_to_date(dt, "-first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 1, 1)));

        let dt = NaiveDate::from_ymd(2020, 2, 10);
        let nm = human_to_date(dt, "first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 3, 1)));
        let nm = human_to_date(dt, "-first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 2, 1)));

        let dt = NaiveDate::from_ymd(2020, 7, 9); // thursday
        let tests: Vec<Test> = vec![
            Test { txt: "tmr", val: NaiveDate::from_ymd(2020, 7, 10) },
            Test { txt: "tm", val: NaiveDate::from_ymd(2020, 7, 10) },
            Test { txt: "tomorrow", val: NaiveDate::from_ymd(2020, 7, 10) },
            Test { txt: "today", val: NaiveDate::from_ymd(2020, 7, 9) },
            Test { txt: "first", val: NaiveDate::from_ymd(2020, 8, 1) },
            Test { txt: "last", val: NaiveDate::from_ymd(2020, 7, 31) },
            Test { txt: "mon", val: NaiveDate::from_ymd(2020, 7, 13) },
            Test { txt: "tu", val: NaiveDate::from_ymd(2020, 7, 14) },
            Test { txt: "wed", val: NaiveDate::from_ymd(2020, 7, 15) },
            Test { txt: "thursday", val: NaiveDate::from_ymd(2020, 7, 16) },
            Test { txt: "fri", val: NaiveDate::from_ymd(2020, 7, 10) },
            Test { txt: "sa", val: NaiveDate::from_ymd(2020, 7, 11) },
            Test { txt: "sunday", val: NaiveDate::from_ymd(2020, 7, 12) },
            Test { txt: "yesterday", val: NaiveDate::from_ymd(2020, 7, 8) },
            Test { txt: "-mon", val: NaiveDate::from_ymd(2020, 7, 6) },
            Test { txt: "-tu", val: NaiveDate::from_ymd(2020, 7, 7) },
            Test { txt: "-wed", val: NaiveDate::from_ymd(2020, 7, 8) },
            Test { txt: "-thursday", val: NaiveDate::from_ymd(2020, 7, 2) },
            Test { txt: "-fri", val: NaiveDate::from_ymd(2020, 7, 3) },
            Test { txt: "-sa", val: NaiveDate::from_ymd(2020, 7, 4) },
            Test { txt: "-sunday", val: NaiveDate::from_ymd(2020, 7, 5) },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }
    }

    #[test]
    fn range_test() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let tests: Vec<TestRange> = vec![
            TestRange {
                txt: "..tue",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: 6, high: 0 },
                    span: tfilter::ValueSpan::Lower,
                },
            },
            TestRange {
                txt: ":2d",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: 3, high: 0 },
                    span: tfilter::ValueSpan::Lower,
                },
            },
            TestRange {
                txt: "tue..",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: 0, high: 4 },
                    span: tfilter::ValueSpan::Higher,
                },
            },
            TestRange {
                txt: "3d:",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: 0, high: 2 },
                    span: tfilter::ValueSpan::Higher,
                },
            },
            TestRange {
                txt: "-tue..we",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: -2, high: 6 },
                    span: tfilter::ValueSpan::Range,
                },
            },
            TestRange {
                txt: "we..-tue",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: -2, high: 6 },
                    span: tfilter::ValueSpan::Range,
                },
            },
            TestRange {
                txt: "-tue..-wed",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: -2, high: -1 },
                    span: tfilter::ValueSpan::Range,
                },
            },
            TestRange {
                txt: "-1w:today",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: -7, high: 0 },
                    span: tfilter::ValueSpan::Range,
                },
            },
            TestRange {
                txt: "..soon",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: 7, high: 0 },
                    span: tfilter::ValueSpan::Lower,
                },
            },
            TestRange {
                txt: "soon..",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: 0, high: 5 },
                    span: tfilter::ValueSpan::Higher,
                },
            },
            TestRange {
                txt: "-soon..soon",
                val: tfilter::DateRange {
                    days: tfilter::ValueRange { low: -6, high: 6 },
                    span: tfilter::ValueSpan::Range,
                },
            },
        ];
        for test in tests.iter() {
            let rng = human_to_range(dt, test.txt, 6).unwrap();
            assert_eq!(rng, test.val, "{}", test.txt);
        }
    }

    #[test]
    fn date_replace() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let s = fix_date(dt, "error due:xxxx next week", "due:", 0);
        assert_eq!(s, None);
        let s = fix_date(dt, "due: next week", "due:", 0);
        assert_eq!(s, None);

        let s = fix_date(dt, "due:1w next week", "due:", 0);
        assert_eq!(s, Some("due:2020-07-16 next week".to_string()));
        let s = fix_date(dt, "next day due:1d", "due:", 0);
        assert_eq!(s, Some("next day due:2020-07-10".to_string()));
        let s = fix_date(dt, "special due:sat in the middle", "due:", 0);
        assert_eq!(s, Some("special due:2020-07-11 in the middle".to_string()));
    }
}
