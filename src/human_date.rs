use chrono::{Datelike, Duration, NaiveDate, Weekday};

const NO_CHANGE: &str = "no change";
const DAYS_PER_WEEK: u32 = 7;

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

fn abs_time_diff(base: NaiveDate, human: &str) -> HumanResult {
    let mut num = 0u32;
    let mut dt = base;

    for c in human.chars() {
        match c.to_digit(10) {
            None => {
                if num != 0 {
                    match c {
                        'd' => {
                            let dur = Duration::days(num as i64);
                            dt += dur;
                        }
                        'w' => {
                            let dur = Duration::weeks(num as i64);
                            dt += dur;
                        }
                        'm' => {
                            let mut y = dt.year();
                            let mut m = dt.month();
                            let mut d = dt.day();
                            let mxd = days_in_month(y, m);
                            m += num;
                            if m > 12 {
                                m -= 1;
                                y += (m / 12) as i32;
                                m = (m % 12) + 1;
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
                            y += num as i32;
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
                d = if n >= days_in_month(y, m) || n >= bdays {
                    days_in_month(y, m)
                } else {
                    n
                };
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
            if n < 1 || n > 12 {
                return Err(format!("month number must be between 1 and 12 ({})", n));
            }
            n
        }
    };
    let d = match parts[1].parse::<u32>() {
        Err(_) => return Err(format!("invalid day number: {}", parts[1])),
        Ok(n) => {
            if n < 1 || n > 31 {
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

fn special_time_point(base: NaiveDate, human: &str) -> HumanResult {
    let s = human.replace(&['-', '_'][..], "").to_lowercase();
    match s.as_str() {
        "today" => Ok(base),
        "tomorrow" | "tmr" | "tm" => Ok(base.succ()),
        "first" => {
            let mut y = base.year();
            let mut m = base.month();
            if m < 12 {
                m += 1;
            } else {
                y += 1;
                m = 1;
            }
            Ok(NaiveDate::from_ymd(y, m, 1))
        }
        "last" => {
            let y = base.year();
            let m = base.month();
            let d = days_in_month(y, m);
            Ok(NaiveDate::from_ymd(y, m, d))
        }
        "monday" | "mon" | "mo" => next_weekday(base, Weekday::Mon),
        "tuesday" | "tue" | "tu" => next_weekday(base, Weekday::Tue),
        "wednesday" | "wed" | "we" => next_weekday(base, Weekday::Wed),
        "thursday" | "thu" | "th" => next_weekday(base, Weekday::Thu),
        "friday" | "fri" | "fr" => next_weekday(base, Weekday::Fri),
        "saturday" | "sat" | "sa" => next_weekday(base, Weekday::Sat),
        "sunday" | "sun" | "su" => next_weekday(base, Weekday::Sun),
        _ => Err(format!("invalid date '{}'", human)),
    }
}

// Converts human-readable date to an absolute date in todo-txt format. If the date is already an
// absolute value, the function returns None. In case of any error None is returned as well.
pub fn human_to_date(base: NaiveDate, human: &str) -> HumanResult {
    if human.is_empty() {
        return Err("empty date".to_string());
    }
    if human.find(|c: char| c < '0' || c > '9').is_none() {
        return day_of_first_month(base, human);
    }
    if human.find(|c: char| (c < '0' || c > '9') && c != '-').is_none() {
        if human.matches('-').count() == 1 {
            // month-day case
            return no_year_date(base, human);
        }
        // normal date, nothing to fix
        return Err(NO_CHANGE.to_string());
    }
    if human
        .find(|c: char| c < '0' || (c > '9' && c != 'd' && c != 'm' && c != 'w' && c != 'y'))
        .is_none()
    {
        return abs_time_diff(base, human);
    }

    // some "special" word like "tomorrow", "tue"
    special_time_point(base, human)
}

pub fn fix_date(base: NaiveDate, orig: &str, look_for: &str) -> Option<String> {
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
    let human = if let Some(p) = substr.find(' ') {
        &substr[..p]
    } else {
        &substr
    };
    match human_to_date(base, human) {
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

#[cfg(test)]
mod humandate_test {
    use super::*;
    use chrono::Local;

    struct Test {
        txt: &'static str,
        val: NaiveDate,
    }

    #[test]
    fn no_change() {
        let dt = Local::now().date().naive_local();
        let res = human_to_date(dt, "2010-10-10");
        let must = Err(NO_CHANGE.to_string());
        assert_eq!(res, must)
    }

    #[test]
    fn month_day() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let tests: Vec<Test> = vec![
            Test{txt: "7", val: NaiveDate::from_ymd(2020, 8, 7)},
            Test{txt: "11", val: NaiveDate::from_ymd(2020, 7, 11)},
            Test{txt: "31", val: NaiveDate::from_ymd(2020, 7, 31)},
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }

        let dt = NaiveDate::from_ymd(2020, 6, 9);
        let nm = human_to_date(dt, "31");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 6, 30)));
        let dt = NaiveDate::from_ymd(2020, 2, 4);
        let nm = human_to_date(dt, "31");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 2, 29)));
        let dt = NaiveDate::from_ymd(2020, 2, 29);
        let nm = human_to_date(dt, "29");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 3, 31)));

        let nm = human_to_date(dt, "32");
        assert!(nm.is_err());
        let nm = human_to_date(dt, "0");
        assert!(nm.is_err());
    }

    #[test]
    fn month_and_day() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let nm = human_to_date(dt, "07-08");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2021, 7, 8)));
        let nm = human_to_date(dt, "07-11");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 7, 11)));
        let nm = human_to_date(dt, "02-31");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2021, 2, 28)));
    }

    #[test]
    fn absolute() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let nm = human_to_date(dt, "1w");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 7, 16)));
        let nm2 = human_to_date(dt, "3d4d");
        assert_eq!(nm, nm2);
        let nm = human_to_date(dt, "1y");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2021, 7, 9)));
        let nm = human_to_date(dt, "2w2d1m");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 8, 25)));

        let dt = NaiveDate::from_ymd(2020, 2, 29);
        let nm = human_to_date(dt, "1m");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 3, 31)));
        let nm = human_to_date(dt, "1y");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2021, 2, 28)));
        let dt = NaiveDate::from_ymd(2021, 2, 28);
        let nm = human_to_date(dt, "3y");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2024, 2, 29)));
    }

    #[test]
    fn special() {
        let dt = NaiveDate::from_ymd(2020, 2, 29);
        let nm = human_to_date(dt, "last");
        assert_eq!(nm, Ok(NaiveDate::from_ymd(2020, 2, 29)));

        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let tests: Vec<Test> = vec![
            Test{txt: "tmr", val: NaiveDate::from_ymd(2020, 7, 10)},
            Test{txt: "tm", val: NaiveDate::from_ymd(2020, 7, 10)},
            Test{txt: "tomorrow", val: NaiveDate::from_ymd(2020, 7, 10)},
            Test{txt: "today", val: NaiveDate::from_ymd(2020, 7, 9)},
            Test{txt: "first", val: NaiveDate::from_ymd(2020, 8, 1)},
            Test{txt: "last", val: NaiveDate::from_ymd(2020, 7, 31)},
            Test{txt: "mon", val: NaiveDate::from_ymd(2020, 7, 13)},
            Test{txt: "tu", val: NaiveDate::from_ymd(2020, 7, 14)},
            Test{txt: "wed", val: NaiveDate::from_ymd(2020, 7, 15)},
            Test{txt: "thursday", val: NaiveDate::from_ymd(2020, 7, 16)},
            Test{txt: "fri", val: NaiveDate::from_ymd(2020, 7, 10)},
            Test{txt: "sa", val: NaiveDate::from_ymd(2020, 7, 11)},
            Test{txt: "sunday", val: NaiveDate::from_ymd(2020, 7, 12)},
        ];
        for test in tests.iter() {
            let nm = human_to_date(dt, test.txt);
            assert_eq!(nm, Ok(test.val), "{}", test.txt);
        }
    }

    #[test]
    fn date_replace() {
        let dt = NaiveDate::from_ymd(2020, 7, 9);
        let s = fix_date(dt, "error due:xxxx next week", "due:");
        assert_eq!(s, None);
        let s = fix_date(dt, "due: next week", "due:");
        assert_eq!(s, None);

        let s = fix_date(dt, "due:1w next week", "due:");
        assert_eq!(s, Some("due:2020-07-16 next week".to_string()));
        let s = fix_date(dt, "next day due:1d", "due:");
        assert_eq!(s, Some("next day due:2020-07-10".to_string()));
        let s = fix_date(dt, "special due:sat in the middle", "due:");
        assert_eq!(s, Some("special due:2020-07-11 in the middle".to_string()));
    }
}
