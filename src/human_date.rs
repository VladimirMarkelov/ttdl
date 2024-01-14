use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::mem;

use todo_lib::{terr, tfilter};

pub const NO_CHANGE: &str = "no change";
const DAYS_PER_WEEK: u32 = 7;
const FAR_PAST: i64 = -100 * 365; // far in the past

type HumanResult = Result<NaiveDate, String>;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum CalendarRangeType {
    Days(i8),
    Weeks(i8),
    Months(i8),
    Years(i8),
    DayRange(i8, i8),
    WeekRange(i8, i8),
    MonthRange(i8, i8),
    YearRange(i8, i8),
}
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct CalendarRange {
    pub(crate) strict: bool,
    pub(crate) rng: CalendarRangeType,
}

impl Default for CalendarRange {
    fn default() -> CalendarRange {
        CalendarRange { strict: false, rng: CalendarRangeType::Days(1) }
    }
}

fn parse_int(s: &str) -> (&str, String) {
    let mut res = String::new();
    for c in s.chars() {
        if !c.is_ascii_digit() {
            break;
        }
        res.push(c);
    }
    (&s[res.len()..], res)
}

impl CalendarRange {
    pub(crate) fn parse(s: &str) -> Result<CalendarRange, terr::TodoError> {
        if s.contains("..") || s.contains(':') {
            CalendarRange::parse_range(s)
        } else {
            CalendarRange::parse_single(s)
        }
    }

    fn parse_single_num(s_in: &str) -> Result<(&str, i8, bool), terr::TodoError> {
        let (s, strict) = if s_in.starts_with('+') { (&s_in["+".len()..], true) } else { (s_in, false) };
        let (s, sgn) = if s.starts_with('-') { (&s["-".len()..], -1i8) } else { (s, 1i8) };
        let (s, num_str) = parse_int(s);
        let num = if num_str.is_empty() {
            1
        } else {
            match num_str.parse::<i8>() {
                Ok(n) => n,
                Err(_) => {
                    return Err(terr::TodoError::InvalidValue(s_in.to_string(), "calendar range value".to_string()))
                }
            }
        };
        let num = num * sgn;
        match s {
            "" | "d" | "D" => {
                if num.abs() > 100 {
                    return Err(terr::TodoError::InvalidValue(
                        s_in.to_string(),
                        "number of days(range -100..100)".to_string(),
                    ));
                }
            }
            "w" | "W" => {
                if num.abs() > 16 {
                    return Err(terr::TodoError::InvalidValue(
                        s_in.to_string(),
                        "number of weeks(range -16..16)".to_string(),
                    ));
                }
            }
            "m" | "M" => {
                if num.abs() > 24 {
                    return Err(terr::TodoError::InvalidValue(
                        s_in.to_string(),
                        "number of months(range -24..24)".to_string(),
                    ));
                }
            }
            "y" | "Y" => {
                if num.abs() > 2 {
                    return Err(terr::TodoError::InvalidValue(
                        s_in.to_string(),
                        "number of years(range -2..2)".to_string(),
                    ));
                }
            }
            _ => return Err(terr::TodoError::InvalidValue(s_in.to_string(), "calendar range type".to_string())),
        }
        Ok((s, num, strict))
    }

    fn parse_range(s: &str) -> Result<CalendarRange, terr::TodoError> {
        let ends: Vec<&str> = if s.contains("..") { s.split("..").collect() } else { s.split(':').collect() };
        if ends.len() > 2 {
            return Err(terr::TodoError::InvalidValue(
                s.to_string(),
                "calendar range cannot contain more than 2 values".to_string(),
            ));
        }
        let (ltp, lnum, lstrict) = CalendarRange::parse_single_num(ends[0])?;
        let (rtp, rnum, rstrict) = CalendarRange::parse_single_num(ends[1])?;
        if ltp != rtp {
            return Err(terr::TodoError::InvalidValue(
                s.to_string(),
                "both range ends must use the same dimensions".to_string(),
            ));
        }
        let (lnum, rnum) = if lnum > rnum { (rnum, lnum) } else { (lnum, rnum) };
        let rng = CalendarRange {
            strict: lstrict || rstrict,
            rng: match ltp {
                "" | "d" | "D" => CalendarRangeType::DayRange(lnum, rnum),
                "w" | "W" => CalendarRangeType::WeekRange(lnum, rnum),
                "m" | "M" => CalendarRangeType::MonthRange(lnum, rnum),
                "y" | "Y" => CalendarRangeType::YearRange(lnum, rnum),
                _ => {
                    return Err(terr::TodoError::InvalidValue(
                        ltp.to_string(),
                        "date range type".to_string(),
                    ));
                },
            },
        };
        Ok(rng)
    }

    fn parse_single(s: &str) -> Result<CalendarRange, terr::TodoError> {
        let (tp, num, strict) = CalendarRange::parse_single_num(s)?;
        let rng = CalendarRange {
            strict,
            rng: match tp {
                "" | "d" | "D" => CalendarRangeType::Days(num),
                "w" | "W" => CalendarRangeType::Weeks(num),
                "m" | "M" => CalendarRangeType::Months(num),
                "y" | "Y" => CalendarRangeType::Years(num),
                _ => {
                    return Err(terr::TodoError::InvalidValue(
                        tp.to_string(),
                        "date range type".to_string(),
                    ));
                },
            },
        };
        Ok(rng)
    }
}

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

pub fn add_months(dt: NaiveDate, num: u32, back: bool) -> NaiveDate {
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
        NaiveDate::from_ymd_opt(y, m, d).unwrap_or(dt)
    } else {
        NaiveDate::from_ymd_opt(y, m, new_mxd).unwrap_or(dt)
    }
}

pub fn add_years(dt: NaiveDate, num: u32, back: bool) -> NaiveDate {
    let mut y = dt.year();
    let m = dt.month();
    let mut d = dt.day();
    if back {
        y -= num as i32;
    } else {
        y += num as i32;
    }
    if d > days_in_month(y, m) {
        d = days_in_month(y, m);
    }
    NaiveDate::from_ymd_opt(y, m, d).unwrap_or(dt)
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
                            dt = add_months(dt, num, back);
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
                                dt = NaiveDate::from_ymd_opt(y, m, d).unwrap_or(base);
                            } else {
                                dt = NaiveDate::from_ymd_opt(y, m, new_mxd).unwrap_or(base);
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
        return Err(format!("invalid date '{human}'"));
    }
    Ok(dt)
}

pub(crate) fn next_weekday(base: NaiveDate, wd: Weekday) -> NaiveDate {
    let base_wd = base.weekday();
    let (bn, wn) = (base_wd.number_from_monday(), wd.number_from_monday());
    if bn < wn {
        // this week
        base + Duration::days((wn - bn) as i64)
    } else {
        // next week
        base + Duration::days((DAYS_PER_WEEK + wn - bn) as i64)
    }
}

pub(crate) fn prev_weekday(base: NaiveDate, wd: Weekday) -> NaiveDate {
    let base_wd = base.weekday();
    let (bn, wn) = (base_wd.number_from_monday(), wd.number_from_monday());
    if bn > wn {
        // this week
        base - Duration::days(bn as i64 - wn as i64)
    } else {
        // week before
        base + Duration::days(wn as i64 - bn as i64 - DAYS_PER_WEEK as i64)
    }
}

// Converts "human" which is a string contains a number to a date.
// "human" is a day of a date. If today's day is less than "human", the function returns the
// "human" date of the next month, otherwise of this month.
// E.g: today=2022-06-20, human="24" --> 2022-06-24
//      today=2022-06-20, human="19" --> 2022-07-19
fn day_of_first_month(base: NaiveDate, human: &str) -> HumanResult {
    match human.parse::<u32>() {
        Err(e) => Err(format!("invalid day of month: {e:?}")),
        Ok(n) => {
            if n == 0 || n > 31 {
                Err(format!("Day number too big: {n}"))
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
                Ok(NaiveDate::from_ymd_opt(y, m, d).unwrap_or(base))
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
                return Err(format!("month number must be between 1 and 12 ({n})"));
            }
            n
        }
    };
    let d = match parts[1].parse::<u32>() {
        Err(_) => return Err(format!("invalid day number: {}", parts[1])),
        Ok(n) => {
            if !(1..=31).contains(&n) {
                return Err(format!("day number must be between 1 and 31 ({n})"));
            }
            let mx = days_in_month(y, m);
            if n > mx {
                mx
            } else {
                n
            }
        }
    };
    let dt = NaiveDate::from_ymd_opt(y, m, d).unwrap_or(base);
    if dt < base {
        let y = y + 1;
        let mx = days_in_month(y, m);
        let d = if mx < d { mx } else { d };
        Ok(NaiveDate::from_ymd_opt(y, m, d).unwrap_or(base))
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
        return Err(format!("'{human}' cannot be back"));
    }
    match s.as_str() {
        "today" => Ok(base),
        "tomorrow" | "tmr" | "tm" => Ok(base.succ_opt().unwrap_or(base)),
        "yesterday" => Ok(base.pred_opt().unwrap_or(base)),
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
            Ok(NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(base))
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
            Ok(NaiveDate::from_ymd_opt(y, m, d).unwrap_or(base))
        }
        "monday" | "mon" | "mo" => {
            if back {
                Ok(prev_weekday(base, Weekday::Mon))
            } else {
                Ok(next_weekday(base, Weekday::Mon))
            }
        }
        "tuesday" | "tue" | "tu" => {
            if back {
                Ok(prev_weekday(base, Weekday::Tue))
            } else {
                Ok(next_weekday(base, Weekday::Tue))
            }
        }
        "wednesday" | "wed" | "we" => {
            if back {
                Ok(prev_weekday(base, Weekday::Wed))
            } else {
                Ok(next_weekday(base, Weekday::Wed))
            }
        }
        "thursday" | "thu" | "th" => {
            if back {
                Ok(prev_weekday(base, Weekday::Thu))
            } else {
                Ok(next_weekday(base, Weekday::Thu))
            }
        }
        "friday" | "fri" | "fr" => {
            if back {
                Ok(prev_weekday(base, Weekday::Fri))
            } else {
                Ok(next_weekday(base, Weekday::Fri))
            }
        }
        "saturday" | "sat" | "sa" => {
            if back {
                Ok(prev_weekday(base, Weekday::Sat))
            } else {
                Ok(next_weekday(base, Weekday::Sat))
            }
        }
        "sunday" | "sun" | "su" => {
            if back {
                Ok(prev_weekday(base, Weekday::Sun))
            } else {
                Ok(next_weekday(base, Weekday::Sun))
            }
        }
        _ => Err(format!("invalid date '{human}'")),
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

    if human.find(|c: char| !c.is_ascii_digit()).is_none() {
        if back {
            return Err("negative day of month".to_string());
        }
        return day_of_first_month(base, human);
    }
    if human.find(|c: char| !c.is_ascii_digit() && c != '-').is_none() {
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

// Replace a special word in due date with a real date.
// E.g, "due:sat" ==> "due:2022-07-09" for today between 2022-07-03 and 2022-07-09
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
    let human = if let Some(p) = substr.find(' ') { &substr[..p] } else { substr };
    match human_to_date(base, human, soon_days) {
        Err(e) => {
            if e != NO_CHANGE {
                eprintln!("invalid due date: {human}");
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
    let parts: Vec<&str> = if human.find(':').is_none() {
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
                days: tfilter::ValueRange { high: tfilter::INCLUDE_NONE, low: (d - base).num_days() },
                span: tfilter::ValueSpan::Range,
            }),
        }
    } else if parts[0] == "none" {
        match human_to_date(base, parts[1], soon_days) {
            Err(e) => Err(range_error(&e)),
            Ok(d) => Ok(tfilter::DateRange {
                days: tfilter::ValueRange { low: tfilter::INCLUDE_NONE, high: (d - base).num_days() },
                span: tfilter::ValueSpan::Range,
            }),
        }
    } else {
        Err(range_error(human))
    }
}

pub(crate) fn is_range(human: &str) -> bool {
    human.contains("..") || human.contains(':')
}

fn range_error(msg: &str) -> terr::TodoError {
    terr::TodoError::InvalidValue(msg.to_string(), "date range".to_string())
}

pub(crate) fn human_to_range(
    base: NaiveDate,
    human: &str,
    soon_days: u8,
) -> Result<tfilter::DateRange, terr::TodoError> {
    let parts: Vec<&str> = if human.find(':').is_none() {
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

pub(crate) fn calendar_first_day(today: NaiveDate, rng: &CalendarRange, first_sunday: bool) -> NaiveDate {
    match rng.rng {
        CalendarRangeType::Days(n) => {
            if n >= 0 {
                today
            } else {
                let diff = n + 1;
                today.checked_add_signed(Duration::days(diff.into())).unwrap_or(today)
            }
        }
        CalendarRangeType::DayRange(n, _) => today.checked_add_signed(Duration::days(n.into())).unwrap_or(today),
        CalendarRangeType::Weeks(n) => {
            let is_first =
                (today.weekday() == Weekday::Sun && first_sunday) || (today.weekday() == Weekday::Mon && !first_sunday);
            let today = if rng.strict || is_first {
                today
            } else {
                match first_sunday {
                    true => prev_weekday(today, Weekday::Sun),
                    false => prev_weekday(today, Weekday::Mon),
                }
            };
            if rng.strict || n >= -1 {
                return today;
            }
            let diff = if rng.strict {
                n
            } else if n > 0 {
                n - 1
            } else {
                n + 1
            };
            today.checked_add_signed(Duration::weeks(diff.into())).unwrap_or(today)
        }
        CalendarRangeType::WeekRange(n, _) => {
            let diff = if rng.strict {
                n
            } else if n > 0 {
                n - 1
            } else {
                n + 1
            };
            today.checked_add_signed(Duration::weeks(diff.into())).unwrap_or(today)
        }
        CalendarRangeType::Months(n) => {
            if n >= 0 {
                if rng.strict {
                    return today;
                }
                return NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
            }
            let (today, diff) = if rng.strict {
                (today, -n)
            } else {
                (NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today), -n - 1)
            };
            let today = add_months(today, diff as u32, true);
            if rng.strict {
                return today.checked_add_signed(Duration::days(1)).unwrap_or(today);
            }
            today
        }
        CalendarRangeType::MonthRange(n, _) => {
            let (today, diff) = if rng.strict {
                (today, n)
            } else {
                (
                    NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today),
                    if n > 0 { n - 1 } else { n + 1 },
                )
            };
            add_months(today, diff.unsigned_abs() as u32, n < 0)
        }
        CalendarRangeType::Years(n) => {
            if n >= 0 {
                if rng.strict {
                    return today;
                }
                return NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
            }
            let (today, diff) = if rng.strict {
                (today, -n)
            } else {
                (NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today), -n - 1)
            };
            add_years(today, diff as u32, n < 0)
        }
        CalendarRangeType::YearRange(n, _) => {
            let (today, diff) =
                if rng.strict { (today, n) } else { (NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today), n) };
            add_years(today, diff.unsigned_abs() as u32, n < 0)
        }
    }
}

pub(crate) fn calendar_last_day(today: NaiveDate, rng: &CalendarRange, first_sunday: bool) -> NaiveDate {
    match rng.rng {
        CalendarRangeType::Days(n) => {
            if n <= 0 {
                return today;
            }
            let n = n - 1;
            today.checked_add_signed(Duration::days(n.into())).unwrap_or(today)
        }
        CalendarRangeType::DayRange(_, n) => today.checked_add_signed(Duration::days(n.into())).unwrap_or(today),
        CalendarRangeType::Weeks(n) => {
            if rng.strict {
                if n <= 0 {
                    return today;
                }
                return match today.checked_add_signed(Duration::weeks(n.into())) {
                    None => today,
                    Some(d) => d.checked_add_signed(Duration::days(-1)).unwrap_or(d),
                };
            }
            let today = match first_sunday {
                true => next_weekday(today, Weekday::Sat),
                false => next_weekday(today, Weekday::Sun),
            };
            if n <= 1 {
                return today;
            }
            let n = n - 1;
            today.checked_add_signed(Duration::weeks(n.into())).unwrap_or(today)
        }
        CalendarRangeType::WeekRange(_, n) => today.checked_add_signed(Duration::weeks(n.into())).unwrap_or(today),
        CalendarRangeType::Months(n) => {
            if rng.strict {
                if n <= 0 {
                    return today;
                }
                let today = add_months(today, n.unsigned_abs() as u32, n < 0);
                return today.checked_add_signed(Duration::days(-1)).unwrap_or(today);
            }
            let last = days_in_month(today.year(), today.month());
            let today = NaiveDate::from_ymd_opt(today.year(), today.month(), last).unwrap_or(today);
            if n <= 1 {
                return today;
            }
            let diff = n - 1;
            add_months(today, diff.unsigned_abs() as u32, diff < 0)
        }
        CalendarRangeType::MonthRange(_, n) => {
            let dt = add_months(today, n.unsigned_abs() as u32, n < 0);
            if rng.strict {
                dt
            } else {
                let y = dt.year();
                let m = dt.month();
                let d = days_in_month(y, m);
                NaiveDate::from_ymd_opt(y, m, d).unwrap_or(dt)
            }
        }
        CalendarRangeType::Years(n) => {
            if rng.strict {
                if n <= 0 {
                    return today;
                }
                return add_years(today, n as u32, false);
            }
            let dt = NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap_or(today);
            if n <= 1 {
                dt
            } else {
                add_years(dt, (n - 1) as u32, false)
            }
        }
        CalendarRangeType::YearRange(_, n) => {
            if rng.strict {
                return add_years(today, n.unsigned_abs() as u32, n < 0);
            }
            let dt = add_years(today, n.unsigned_abs() as u32, n < 0);
            NaiveDate::from_ymd_opt(dt.year(), 12, 31).unwrap_or(today)
        }
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
        let dt = Local::now().date_naive();
        let res = human_to_date(dt, "2010-10-10", 0);
        let must = Err(NO_CHANGE.to_string());
        assert_eq!(res, must)
    }

    #[test]
    fn month_day() {
        let dt = NaiveDate::from_ymd_opt(2020, 7, 9).unwrap();
        let tests: Vec<Test> = vec![
            Test { txt: "7", val: NaiveDate::from_ymd_opt(2020, 8, 7).unwrap() },
            Test { txt: "11", val: NaiveDate::from_ymd_opt(2020, 7, 11).unwrap() },
            Test { txt: "31", val: NaiveDate::from_ymd_opt(2020, 7, 31).unwrap() },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }

        let dt = NaiveDate::from_ymd_opt(2020, 6, 9).unwrap();
        let nm = human_to_date(dt, "31", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 6, 30).unwrap()));
        let dt = NaiveDate::from_ymd_opt(2020, 2, 4).unwrap();
        let nm = human_to_date(dt, "31", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 2, 29).unwrap()));
        let dt = NaiveDate::from_ymd_opt(2020, 2, 29).unwrap();
        let nm = human_to_date(dt, "29", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 3, 31).unwrap()));

        let nm = human_to_date(dt, "32", 0);
        assert!(nm.is_err());
        let nm = human_to_date(dt, "0", 0);
        assert!(nm.is_err());
    }

    #[test]
    fn month_and_day() {
        let dt = NaiveDate::from_ymd_opt(2020, 7, 9).unwrap();
        let nm = human_to_date(dt, "07-08", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2021, 7, 8).unwrap()));
        let nm = human_to_date(dt, "07-11", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 7, 11).unwrap()));
        let nm = human_to_date(dt, "02-31", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2021, 2, 28).unwrap()));
    }

    #[test]
    fn absolute() {
        let dt = NaiveDate::from_ymd_opt(2020, 7, 9).unwrap();
        let tests: Vec<Test> = vec![
            Test { txt: "1w", val: NaiveDate::from_ymd_opt(2020, 7, 16).unwrap() },
            Test { txt: "3d4d", val: NaiveDate::from_ymd_opt(2020, 7, 16).unwrap() },
            Test { txt: "1y", val: NaiveDate::from_ymd_opt(2021, 7, 9).unwrap() },
            Test { txt: "2w2d1m", val: NaiveDate::from_ymd_opt(2020, 8, 25).unwrap() },
            Test { txt: "-1w", val: NaiveDate::from_ymd_opt(2020, 7, 2).unwrap() },
            Test { txt: "-3d4d", val: NaiveDate::from_ymd_opt(2020, 7, 2).unwrap() },
            Test { txt: "-1y", val: NaiveDate::from_ymd_opt(2019, 7, 9).unwrap() },
            Test { txt: "-2w2d1m", val: NaiveDate::from_ymd_opt(2020, 5, 23).unwrap() },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }

        let dt = NaiveDate::from_ymd_opt(2021, 2, 28).unwrap();
        let tests: Vec<Test> = vec![
            Test { txt: "1m", val: NaiveDate::from_ymd_opt(2021, 3, 31).unwrap() },
            Test { txt: "1y", val: NaiveDate::from_ymd_opt(2022, 2, 28).unwrap() },
            Test { txt: "3y", val: NaiveDate::from_ymd_opt(2024, 2, 29).unwrap() },
            Test { txt: "-1m", val: NaiveDate::from_ymd_opt(2021, 1, 31).unwrap() },
            Test { txt: "-1y", val: NaiveDate::from_ymd_opt(2020, 2, 29).unwrap() },
            Test { txt: "-3y", val: NaiveDate::from_ymd_opt(2018, 2, 28).unwrap() },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }
    }

    #[test]
    fn special() {
        let dt = NaiveDate::from_ymd_opt(2020, 2, 29).unwrap();
        let nm = human_to_date(dt, "last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 3, 31).unwrap()));
        let nm = human_to_date(dt, "-last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 1, 31).unwrap()));

        let dt = NaiveDate::from_ymd_opt(2020, 2, 10).unwrap();
        let nm = human_to_date(dt, "last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 2, 29).unwrap()));
        let nm = human_to_date(dt, "-last", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 1, 31).unwrap()));

        let dt = NaiveDate::from_ymd_opt(2020, 2, 1).unwrap();
        let nm = human_to_date(dt, "first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 3, 1).unwrap()));
        let nm = human_to_date(dt, "-first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()));

        let dt = NaiveDate::from_ymd_opt(2020, 2, 10).unwrap();
        let nm = human_to_date(dt, "first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 3, 1).unwrap()));
        let nm = human_to_date(dt, "-first", 0);
        assert_eq!(nm, Ok(NaiveDate::from_ymd_opt(2020, 2, 1).unwrap()));

        let dt = NaiveDate::from_ymd_opt(2020, 7, 9).unwrap(); // thursday
        let tests: Vec<Test> = vec![
            Test { txt: "tmr", val: NaiveDate::from_ymd_opt(2020, 7, 10).unwrap() },
            Test { txt: "tm", val: NaiveDate::from_ymd_opt(2020, 7, 10).unwrap() },
            Test { txt: "tomorrow", val: NaiveDate::from_ymd_opt(2020, 7, 10).unwrap() },
            Test { txt: "today", val: NaiveDate::from_ymd_opt(2020, 7, 9).unwrap() },
            Test { txt: "first", val: NaiveDate::from_ymd_opt(2020, 8, 1).unwrap() },
            Test { txt: "last", val: NaiveDate::from_ymd_opt(2020, 7, 31).unwrap() },
            Test { txt: "mon", val: NaiveDate::from_ymd_opt(2020, 7, 13).unwrap() },
            Test { txt: "tu", val: NaiveDate::from_ymd_opt(2020, 7, 14).unwrap() },
            Test { txt: "wed", val: NaiveDate::from_ymd_opt(2020, 7, 15).unwrap() },
            Test { txt: "thursday", val: NaiveDate::from_ymd_opt(2020, 7, 16).unwrap() },
            Test { txt: "fri", val: NaiveDate::from_ymd_opt(2020, 7, 10).unwrap() },
            Test { txt: "sa", val: NaiveDate::from_ymd_opt(2020, 7, 11).unwrap() },
            Test { txt: "sunday", val: NaiveDate::from_ymd_opt(2020, 7, 12).unwrap() },
            Test { txt: "yesterday", val: NaiveDate::from_ymd_opt(2020, 7, 8).unwrap() },
            Test { txt: "-mon", val: NaiveDate::from_ymd_opt(2020, 7, 6).unwrap() },
            Test { txt: "-tu", val: NaiveDate::from_ymd_opt(2020, 7, 7).unwrap() },
            Test { txt: "-wed", val: NaiveDate::from_ymd_opt(2020, 7, 8).unwrap() },
            Test { txt: "-thursday", val: NaiveDate::from_ymd_opt(2020, 7, 2).unwrap() },
            Test { txt: "-fri", val: NaiveDate::from_ymd_opt(2020, 7, 3).unwrap() },
            Test { txt: "-sa", val: NaiveDate::from_ymd_opt(2020, 7, 4).unwrap() },
            Test { txt: "-sunday", val: NaiveDate::from_ymd_opt(2020, 7, 5).unwrap() },
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt, 0);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }
    }

    #[test]
    fn range_test() {
        let dt = NaiveDate::from_ymd_opt(2020, 7, 9).unwrap();
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
        let dt = NaiveDate::from_ymd_opt(2020, 7, 9).unwrap();
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

    #[test]
    fn parse_calendar() {
        struct TestCal {
            txt: &'static str,
            err: bool,
            val: Option<CalendarRange>,
        }
        let tests: Vec<TestCal> = vec![
            TestCal {
                txt: "",
                err: false,
                val: Some(CalendarRange { strict: false, rng: CalendarRangeType::Days(1) }),
            },
            TestCal {
                txt: "12",
                err: false,
                val: Some(CalendarRange { strict: false, rng: CalendarRangeType::Days(12) }),
            },
            TestCal {
                txt: "w",
                err: false,
                val: Some(CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) }),
            },
            TestCal {
                txt: "+m",
                err: false,
                val: Some(CalendarRange { strict: true, rng: CalendarRangeType::Months(1) }),
            },
            TestCal {
                txt: "+-3d",
                err: false,
                val: Some(CalendarRange { strict: true, rng: CalendarRangeType::Days(-3) }),
            },
            TestCal { txt: "zzz", err: true, val: None },
            TestCal { txt: "*2d", err: true, val: None },
            TestCal { txt: "10r", err: true, val: None },
            TestCal { txt: "100m", err: true, val: None },
        ];
        for test in tests.iter() {
            let res = CalendarRange::parse(test.txt);
            if test.err {
                assert!(res.is_err(), "{}", test.txt);
            } else {
                assert!(!res.is_err(), "{}", test.txt);
                assert_eq!(res.unwrap(), test.val.unwrap(), "{}", test.txt);
            }
        }
    }
    #[test]
    fn calendar_first_date() {
        struct TestCal {
            td: NaiveDate,
            rng: CalendarRange,
            sunday: bool,
            res: NaiveDate,
        }
        let tests: Vec<TestCal> = vec![
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 06, 27).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(), // Monday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 06, 27).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(), // Monday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Weeks(1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Weeks(1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Weeks(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 06, 27).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(15) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(15) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(15) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(15) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(-5) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 06, 29).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(-5) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 06, 29).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(-5) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 06, 29).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(-5) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 06, 29).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 01).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 01).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(-2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 05, 04).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(-2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 05, 04).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(-2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 06, 01).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(-2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 06, 01).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Years(-1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 01, 01).unwrap(),
            },
        ];
        for test in tests.iter() {
            let res = calendar_first_day(test.td, &test.rng, test.sunday);
            assert_eq!(res, test.res, "{} - SUN: {}, RANGE: {:?}", test.td, test.sunday, test.rng);
        }
    }
    #[test]
    fn calendar_last_date() {
        struct TestCal {
            td: NaiveDate,
            rng: CalendarRange,
            sunday: bool,
            res: NaiveDate,
        }
        let tests: Vec<TestCal> = vec![
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 09).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 10).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(), // Monday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 09).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(-1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 10).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 09).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 10).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(), // Monday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 09).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 04).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 10).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Weeks(1) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 09).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Weeks(1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 09).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 05).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Weeks(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 16).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Weeks(2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 16).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(15) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 17).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(15) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 17).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(15) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 17).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(15) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 17).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(-5) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Days(-5) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(-5) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Days(-5) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 09, 02).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 09, 02).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 08, 31).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 08, 31).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(-2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: true, rng: CalendarRangeType::Months(-2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(), // Sunday
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(-2) },
                sunday: true,
                res: NaiveDate::from_ymd_opt(2022, 07, 31).unwrap(),
            },
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Months(-2) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 07, 31).unwrap(),
            },
            // ---
            TestCal {
                td: NaiveDate::from_ymd_opt(2022, 07, 03).unwrap(),
                rng: CalendarRange { strict: false, rng: CalendarRangeType::Years(-1) },
                sunday: false,
                res: NaiveDate::from_ymd_opt(2022, 12, 31).unwrap(),
            },
        ];
        for test in tests.iter() {
            let res = calendar_last_day(test.td, &test.rng, test.sunday);
            assert_eq!(res, test.res, "{} - SUN: {}, RANGE: {:?}", test.td, test.sunday, test.rng);
        }
    }
}
