use chrono::{Datelike, Duration, NaiveDate, Weekday};
use todo_lib::{terr, tfilter, todotxt};

use crate::human_date;

// pub fn dd(dt: NaiveDate) -> String {
//     todotxt::format_date(dt)
// }

#[derive(Debug)]
pub struct ExprItem<'a> {
    pub sign: char,
    pub val: &'a str,
}

// Full: YYYY-MM-DD
fn parse_full_date(s: &str) -> Option<&str> {
    let mut st = s;
    match s.find(|c: char| !c.is_ascii_digit()) {
        None => return None,
        Some(i) => if i != 4 {
            return None;
        } else {
            st = &st[i..];
        },
    }
    if !st.starts_with('-') {
        return None;
    }
    st = &st[1..];
    match st.find(|c: char| !c.is_ascii_digit()) {
        None => return None,
        Some(i) => if i != 2 {
            return None;
        } else {
            st = &st[i..];
        },
    }
    if !st.starts_with('-') {
        return None;
    }
    st = &st[1..];
    match st.find(|c: char| !c.is_ascii_digit()) {
        None => if st.len() == 2 {
            Some(s)
        } else {
            None
        },
        Some(i) => if i != 2 {
            None
        } else {
            let l = "2020-01-01".len();
            let rest = &s[l..];
            if !rest.starts_with('-') && !rest.starts_with('+') {
                return None;
            }
            Some(&s[..l])
        },
    }
}

// Short: MM-DD
fn parse_short_date(s: &str) -> Option<&str> {
    let mut st = s;
    match s.find(|c: char| !c.is_ascii_digit()) {
        None => return None,
        Some(i) => if i != 2 {
            return None;
        } else {
            st = &st[i..];
        },
    }
    if !st.starts_with('-') {
        return None;
    }
    st = &st[1..];
    match st.find(|c: char| !c.is_ascii_digit()) {
        None => if st.len() == 2 {
            Some(s)
        } else {
            None
        },
        Some(i) => if i != 2 {
            None
        } else {
            let l = "01-01".len();
            let rest = &s[l..];
            if !rest.starts_with('-') && !rest.starts_with('+') {
                return None;
            }
            Some(&s[..l])
        },
    }
}

// Single day:DD
fn parse_single_day(s: &str) -> Option<&str> {
    match s.find(|c: char| !c.is_ascii_digit()) {
        None => if s.len() < 3 {
            Some(s)
        } else {
            None
        },
        Some(i) => if i > 2 || i == 0 {
            None
        } else {
            let rest = &s[i..];
            if !rest.starts_with('-') && !rest.starts_with('+') {
                return None;
            }
            Some(&s[..i])
        },
    }
}

// Special: WORD (tue/today/tomorrow/etc)
fn parse_special(s: &str) -> Option<&str> {
    let c = match s.chars().next() {
        None => return None,
        Some(cc) => cc,
    };
    if !('a'..='z').contains(&c) && !('A'..='Z').contains(&c) {
        return None;
    }
    match s.find(|c: char| !('a'..='z').contains(&c) && !('A'..='Z').contains(&c)) {
        None => Some(s),
        Some(idx) => {
            let rest = &s[idx..];
            if !rest.starts_with('-') && !rest.starts_with('+') {
                None
            } else {
                Some(&s[..idx])
            }
        }
    }
}

// Duration: ##L (1-2 digits and duration type DWMY)
fn parse_duration(s: &str) -> Option<&str> {
    let c = match s.chars().next() {
        None => return None,
        Some(cc) => cc,
    };
    let durs = vec!['d', 'D', 'w', 'W', 'm', 'M', 'y', 'Y'];
    if c.is_ascii_digit() {
        let idxl = match s.find(|c: char| !c.is_ascii_digit()) {
            None => return Some(s),
            Some(i) => i,
        };
        let rest = &s[idxl..];
        if rest.starts_with('-') || rest.starts_with('+') {
            Some(&s[..idxl])
        } else {
            if let Some(cc) = rest.chars().next() {
                if durs.contains(&cc) {
                    if s.len() == idxl+1 {
                        Some(s)
                    } else {
                        let rest = &s[idxl+1..];
                        if rest.starts_with('-') || rest.starts_with('+') {
                            Some(&s[..idxl+1])
                        } else {
                            None
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    } else if durs.contains(&c) {
        match s.find(|c: char| !('a'..'z').contains(&c) && !('A'..'Z').contains(&c)) {
            None => if s.len() == 1 {
                Some(s)
            } else {
                None
            },
            Some(idx) => {
                let rest = &s[idx..];
                if rest.starts_with('-') || rest.starts_with('+') {
                    Some(&s[..idx])
                } else {
                    None
                }
            },
        }
    } else {
        None
    }
}

fn parse_base_date(s: &str) -> Result<ExprItem, String> {
    if let Some(st) = parse_special(s) {
        return Ok(ExprItem{sign: '+', val: st});
    }
    if let Some(st) = parse_full_date(s) {
        return Ok(ExprItem{sign: '+', val: st});
    }
    if let Some(st) = parse_short_date(s) {
        return Ok(ExprItem{sign: '+', val: st});
    }
    if let Some(st) = parse_single_day(s) {
        return Ok(ExprItem{sign: '+', val: st});
    }
    Err("Failed to parse base date".to_string())
}

pub fn parse_expression(s: &str) -> Result<Vec<ExprItem>, String> {
    let mut items= Vec::new();
    let mut st = match parse_base_date(s) {
        Err(e) => return Err(e),
        Ok(ei) => {
            let sc = &s[ei.val.len()..];
            items.push(ei);
            sc
        },
    };
    while !st.is_empty() {
        if st.len() < 2 {
            return Err(format!("Incomplete expression: '{s}'"));
        }
        let c = match st.chars().next() {
            Some(cc) => cc,
            None => return Err("Internal error".to_string()),
        };
        if c != '-' && c != '+' {
            return Err(format!("Invalid character '{0}'", c));
        }
        st = &st[1..];
        match parse_duration(st) {
            None => return Err(format!("Invalid duration: '{st}'")),
            Some(v) => {
                let ei = ExprItem{sign: c, val: v};
                st = &st[ei.val.len()..];
                items.push(ei);
            },
        }
    }
    Ok(items)
}

fn parse_abs_date(base: NaiveDate, s: &str, soon_days: u8) -> Result<NaiveDate, String> {
    match human_date::human_to_date(base, s, soon_days) {
        Ok(d) => Ok(d),
        Err(e) => if e == human_date::NO_CHANGE {
            match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Ok(d) => Ok(d),
                Err(e) => Err(format!("Invalid date [{s}]: {e}")),
            }
        } else {
            Err(e)
        }
    }
}

fn calc_base(base: NaiveDate, s: &str, task: &todotxt::Task, soon_days: u8, counter: usize) -> Result<NaiveDate, String> {
    let mut dt = base;
    if s.find(|c: char| !('a'..='z').contains(&c) && !('A'..='Z').contains(&c)).is_none() {
        // Special date case
        let spec = s.to_lowercase();
        match spec.as_str() {
            "due" if task.due_date.is_some() => {
                if let Some(d) = task.due_date {
                    dt = d;
                }
            },
            "created" => {
                match task.create_date {
                    None => return Err("Task does not have creation date".to_string()),
                    Some(d) => dt = d,
                }
            },
            "thr" | "t" | "threshold" if task.threshold_date.is_some() => {
                if let Some(d) = task.threshold_date {
                    dt = d;
                }
            },
            _ => if let Some(val) = task.tags.get(s) {
                    dt = calc_expr(base, &val, task, soon_days, counter+1)?;
                 } else {
                    dt = parse_abs_date(dt, &s, soon_days)?;
                 },
        }
    } else {
        // Absolute date
        dt = parse_abs_date(dt, s, soon_days)?;
    }
    Ok(dt)
}

fn calc_expr(base: NaiveDate, s: &str, task: &todotxt::Task, soon_days: u8, counter: usize) -> Result<NaiveDate, String> {
    if counter > 10 {
        return Err("Recursion stack overflow".to_string());
    }

    let items = parse_expression(s)?;
    if items.is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut dt = base;
    for (idx, item) in items.iter().enumerate() {
        match idx {
            0 => {
                dt = calc_base(base, item.val, task, soon_days, counter)?;
            },
            _ => {
                let rec_str = if item.val.find(|c: char| !('0'..='9').contains(&c)).is_none() {
                    format!("{0}d", item.val)
                } else {
                    item.val.to_string()
                };
                let rc = match todotxt::Recurrence::parse(&rec_str) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(format!("Invalid duration '{0}': {e}", item.val));
                    },
                };
                match rc.period {
                    todotxt::Period::Day => {
                        let dur = if item.sign == '-' {
                            Duration::days(-(rc.count as i64))
                        } else {
                            Duration::days(rc.count as i64)
                        };
                        dt += dur;
                    },
                    todotxt::Period::Week => {
                        let dur = if item.sign == '-' {
                            Duration::days(-(rc.count as i64) * 7)
                        } else {
                            Duration::days(rc.count as i64 * 7)
                        };
                        dt += dur;
                    },
                    todotxt::Period::Month => {
                        dt = human_date::add_months(dt, rc.count.into(), item.sign == '-');
                    },
                    todotxt::Period::Year => {
                        dt = human_date::add_years(dt, rc.count.into(), item.sign == '-');
                    },
                }
            }
        }
    }

    Ok(dt)
}

pub fn calculate_expr(base: NaiveDate, s: &str, task: &todotxt::Task, soon_days: u8) -> Result<NaiveDate, String> {
    calc_expr(base, s, task, soon_days, 1)
}

#[cfg(test)]
mod date_expr_test {
    use super::*;
    use chrono::Local;

    struct Test {
        txt: &'static str,
        err: bool,
        res: &'static str,
    }

    #[test]
    fn parse_full_date_test() {
        let tests: Vec<Test> = vec![
            Test { txt: "1999-20-20", err: false, res: "1999-20-20"},
            Test { txt: "1999-20-20+1d", err: false, res: "1999-20-20"},
            Test { txt: "1999-20-20-2", err: false, res: "1999-20-20"},
            Test { txt: "1999-20-20z", err: true, res: ""},
            Test { txt: "21999-20-20", err: true, res: ""},
            Test { txt: "1999-2-20", err: true, res: ""},
            Test { txt: "1999-20-0", err: true, res: ""},
            Test { txt: "19a9-20-20", err: true, res: ""},
            Test { txt: "1999-20-0a", err: true, res: ""},
            Test { txt: "cccccccccc", err: true, res: ""},
            Test { txt: "19992020", err: true, res: ""},
            Test { txt: "", err: true, res: ""},
            Test { txt: "-1999-20-20", err: true, res: ""},
        ];
        for test in tests.iter() {
            let r = parse_full_date(test.txt);
            match r {
                Some(rr) => {
                    if test.err {
                        assert!(false, "Test [{0}] must fail", test.txt);
                    }
                    assert_eq!(test.res, rr, "Failed [{0}]: {:?}", test.txt);
                },
                None => {
                    if !test.err {
                        assert!(false, "Test [{0}] must pass", test.txt);
                    }
                },
            }
        }
    }

    #[test]
    fn parse_special_test() {
        let tests: Vec<Test> = vec![
            Test { txt: "today", err: false, res: "today"},
            Test { txt: "Today", err: false, res: "Today"},
            Test { txt: "tODAY", err: false, res: "tODAY"},
            Test { txt: "tue", err: false, res: "tue"},
            Test { txt: "tue-2", err: false, res: "tue"},
            Test { txt: "today%2", err: true, res: ""},
            Test { txt: "2+today", err: true, res: ""},
        ];
        for test in tests.iter() {
            let r = parse_special(test.txt);
            match r {
                Some(rr) => {
                    if test.err {
                        assert!(false, "Test [{0}] must fail", test.txt);
                    }
                    assert_eq!(test.res, rr, "Failed [{0}]: {:?}", test.txt);
                },
                None => {
                    if !test.err {
                        assert!(false, "Test [{0}] must pass", test.txt);
                    }
                },
            }
        }
    }

    #[test]
    fn parse_short_date_test() {
        let tests: Vec<Test> = vec![
            Test { txt: "20-20", err: false, res: "20-20"},
            Test { txt: "20-20+1d", err: false, res: "20-20"},
            Test { txt: "20-20-2", err: false, res: "20-20"},
            Test { txt: "20-20z", err: true, res: ""},
            Test { txt: "320-20", err: true, res: ""},
            Test { txt: "2-20", err: true, res: ""},
            Test { txt: "20-0", err: true, res: ""},
            Test { txt: "2a-20", err: true, res: ""},
            Test { txt: "20-0a", err: true, res: ""},
            Test { txt: "ccccc", err: true, res: ""},
            Test { txt: "2020", err: true, res: ""},
            Test { txt: "", err: true, res: ""},
            Test { txt: "-20-20", err: true, res: ""},
        ];
        for test in tests.iter() {
            let r = parse_short_date(test.txt);
            match r {
                Some(rr) => {
                    if test.err {
                        assert!(false, "Test [{0}] must fail", test.txt);
                    }
                    assert_eq!(test.res, rr, "Failed [{0}]: {:?}", test.txt);
                },
                None => {
                    if !test.err {
                        assert!(false, "Test [{0}] must pass", test.txt);
                    }
                },
            }
        }
    }

    #[test]
    fn parse_duration_test() {
        let tests: Vec<Test> = vec![
            Test { txt: "w+1", err: false, res: "w"},
            Test { txt: "200d-1", err: false, res: "200d"},
            Test { txt: "15w", err: false, res: "15w"},
            Test { txt: "y", err: false, res: "y"},
            Test { txt: "2+3", err: false, res: "2"},
            Test { txt: "day", err: true, res: ""},
            Test { txt: "", err: true, res: ""},
            Test { txt: "a20", err: true, res: ""},
            Test { txt: "20days", err: true, res: ""},
            Test { txt: "20/4", err: true, res: ""},
            Test { txt: "20w/4", err: true, res: ""},
        ];
        for test in tests.iter() {
            let r = parse_duration(test.txt);
            match r {
                Some(rr) => {
                    if test.err {
                        assert!(false, "Test [{0}] must fail", test.txt);
                    }
                    assert_eq!(test.res, rr, "Failed [{0}]: {:?}", test.txt);
                },
                None => {
                    if !test.err {
                        assert!(false, "Test [{0}] must pass", test.txt);
                    }
                },
            }
        }
    }

    #[test]
    fn parse_expression_test() {
        struct ETest {
            txt: &'static str,
            l: usize,
            err: bool,
            last: &'static str,
        }
        let tests: Vec<ETest> = vec![
            ETest { txt: "2003-01-01", err: false, l: 1, last: "2003-01-01"},
            ETest { txt: "2003-01-01+2d", err: false, l: 2, last: "2d"},
            ETest { txt: "2003-01-01+2d-9", err: false, l: 3, last: "9"},
            ETest { txt: "2003-01-01+9-10m", err: false, l: 3, last: "10m"},
            ETest { txt: "tue+67", err: false, l: 2, last: "67"},
            ETest { txt: "2003-01-01+abcd", err: true, l: 1, last: ""},
            ETest { txt: "tue+tue", err: true, l: 1, last: ""},
            ETest { txt: "tue/2", err: true, l: 1, last: ""},
            ETest { txt: "2d", err: true, l: 1, last: ""},
        ];
        for test in tests.iter() {
            let r = parse_expression(test.txt);
            match r {
                Ok(rr) => {
                    if test.err {
                        assert!(false, "Test [{0}] must fail", test.txt);
                    }
                    assert_eq!(test.l, rr.len(), "{0} expected {1} items, got {2}", test.txt, test.l, rr.len());
                    assert_eq!(test.last, rr[rr.len()-1].val, "Failed [{0}]: {:?}, [{1}] != [{2}]", test.txt, test.last, rr[rr.len()-1].val);
                },
                Err(e) => {
                    if !test.err {
                        assert!(false, "Test [{0}] must pass: {e:?}", test.txt);
                    }
                },
            }
        }
    }

    #[test]
    fn parse_str_expression_test() {
        struct ETest {
            txt: &'static str,
            err: bool,
            res: NaiveDate,
        }
        let tests: Vec<ETest> = vec![
            ETest { txt: "2021-05-07", err: false, res: NaiveDate::from_ymd_opt(2021, 5, 7).unwrap()},
            ETest { txt: "2021-05-07+10d", err: false, res: NaiveDate::from_ymd_opt(2021, 5, 17).unwrap()},
            ETest { txt: "2021-05-07+2w", err: false, res: NaiveDate::from_ymd_opt(2021, 5, 21).unwrap()},
            ETest { txt: "2021-05-07-7d", err: false, res: NaiveDate::from_ymd_opt(2021, 4, 30).unwrap()},
            ETest { txt: "2021-05-07-2m", err: false, res: NaiveDate::from_ymd_opt(2021, 3, 07).unwrap()},
            ETest { txt: "2021-05-07+1y", err: false, res: NaiveDate::from_ymd_opt(2022, 5, 07).unwrap()},
            ETest { txt: "2021-05-07+12d-2d", err: false, res: NaiveDate::from_ymd_opt(2021, 5, 17).unwrap()},
            ETest { txt: "2021-05-07+12d-1w", err: false, res: NaiveDate::from_ymd_opt(2021, 5, 12).unwrap()},

            ETest { txt: "today", err: false, res: NaiveDate::from_ymd_opt(2020, 3, 15).unwrap()},
            ETest { txt: "yesterday+2d", err: false, res: NaiveDate::from_ymd_opt(2020, 3, 16).unwrap()},
            ETest { txt: "first+1w", err: false, res: NaiveDate::from_ymd_opt(2020, 4, 8).unwrap()},

            ETest { txt: "due+1d", err: false, res: NaiveDate::from_ymd_opt(2020, 4, 9).unwrap()},
            ETest { txt: "t-1d", err: false, res: NaiveDate::from_ymd_opt(2020, 4, 3).unwrap()},
            ETest { txt: "extra+1w", err: false, res: NaiveDate::from_ymd_opt(2022, 9, 23).unwrap()},

            ETest { txt: "2021-05-07*2", err: true, res: NaiveDate::from_ymd_opt(2021, 5, 7).unwrap()},
            ETest { txt: "2021-05-07+1t", err: true, res: NaiveDate::from_ymd_opt(2021, 5, 7).unwrap()},
            ETest { txt: "someday", err: true, res: NaiveDate::from_ymd_opt(2021, 5, 7).unwrap()},
        ];

        let base = NaiveDate::from_ymd_opt(2020, 3, 15).unwrap();
        let task = todotxt::Task::parse("create something due:2020-04-08 t:due-4 extra:2022-09-16", base);
        for (idx, test) in tests.iter().enumerate() {
            let d = calculate_expr(base, test.txt, &task, 8);
            if test.err {
                if d.is_ok() {
                    assert!(false, "Test {idx}.[{0}] must fail", test.txt);
                }
            } else {
                if d.is_err() {
                    assert!(false, "Test {idx}.[{0}] must pass: {1:?}", test.txt, d);
                } else {
                    assert_eq!(d.unwrap(), test.res, "Test {idx}.[{0}]", test.txt);
                }
            }
        }
    }
}
