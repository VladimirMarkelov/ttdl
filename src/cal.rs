use std::collections::HashMap;
use std::io::{self, Write};

use chrono::{Datelike, NaiveDate, Weekday};
use termcolor::{Color, StandardStream, WriteColor};

use crate::conf;

pub struct CalPrinter {
    start_date: NaiveDate,
    end_date: NaiveDate,
    page_size: u32,    // The number of months fits into screen width
    total_months: u32, // How many months between start and end dates, rounded up
    first_idx: u32,    // the index(between 0..total_months) of the month displayes in the
    // first column
    started: bool, // false = the struct intialized but there was no call to print_next_line
    // yet
    cols: Vec<Option<NaiveDate>>, // The current dates for every month column displayed. If
                                  // it is None, the month is finished
}

const MONTH_WIDTH: u16 = 3 + 7 * 3; // week # + 7 days and a space in-between
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl CalPrinter {
    pub fn new(s: NaiveDate, e: NaiveDate, max_width: u16) -> CalPrinter {
        let mut cp = CalPrinter {
            start_date: s,
            end_date: e,
            cols: Vec::new(),
            page_size: 0,
            first_idx: 0,
            total_months: months_between(s, e),
            started: false,
        };

        let mc = cp.total_months;
        if mc == 0 {
            return cp;
        }
        let nm = (max_width - 1) / MONTH_WIDTH;
        let nm = if cp.total_months < nm.into() { cp.total_months } else { nm as u32 };
        cp.page_size = nm;

        for midx in 0..nm {
            cp.cols.push(Some(next_month(s, midx)));
        }

        cp
    }
    // Returns true if the current line of months (page_size months at most) are printed
    // completely.
    fn is_bunch_done(&self) -> bool {
        if self.page_size == 0 {
            return true;
        }
        for m in &self.cols {
            if m.is_some() {
                return false;
            }
        }
        true
    }
    // Return true if the given range is displayed in full
    fn is_done(&self) -> bool {
        self.is_bunch_done() && (self.first_idx as usize + self.cols.len() >= self.total_months as usize)
    }
    // Prepare the next page_size months to print
    fn next_page(&mut self) -> bool {
        if self.is_done() {
            return false;
        }
        self.first_idx += self.page_size;
        self.cols = Vec::new();
        let cnt = if self.total_months - self.first_idx >= self.page_size {
            self.page_size
        } else {
            self.total_months - self.first_idx
        };
        if cnt == 0 {
            return false;
        }
        for idx in 0..cnt {
            let delta = self.first_idx + idx;
            self.cols.push(Some(next_month(self.start_date, delta)));
        }
        true
    }

    // Display `s` centered within space of `w` charcters. Spaces are added from both sides.
    // If the lenght of `s` is longer than `w`, the string is printed in full.
    fn print_centered(&self, stdout: &mut StandardStream, s: &str, w: u16) -> io::Result<()> {
        let l = s.chars().count();
        if w as usize <= l {
            return write!(stdout, "{s}");
        }
        let d = w as usize - l;
        let first = d / 2;
        write!(stdout, "{0}{1}{2}", " ".repeat(first), s, " ".repeat(d - first))
    }
    // How many days passed since the beginning of a week until `dt`.
    fn days_since_start(&self, dt: NaiveDate, conf: &conf::Conf) -> usize {
        match dt.weekday() {
            Weekday::Mon => {
                if conf.first_sunday {
                    1
                } else {
                    0
                }
            }
            Weekday::Tue => {
                if conf.first_sunday {
                    2
                } else {
                    1
                }
            }
            Weekday::Wed => {
                if conf.first_sunday {
                    3
                } else {
                    2
                }
            }
            Weekday::Thu => {
                if conf.first_sunday {
                    4
                } else {
                    3
                }
            }
            Weekday::Fri => {
                if conf.first_sunday {
                    5
                } else {
                    4
                }
            }
            Weekday::Sat => {
                if conf.first_sunday {
                    6
                } else {
                    5
                }
            }
            Weekday::Sun => {
                if conf.first_sunday {
                    0
                } else {
                    6
                }
            }
        }
    }
    fn is_first_day_of_week(&self, dt: NaiveDate, conf: &conf::Conf) -> bool {
        (dt.weekday() == Weekday::Sun && conf.first_sunday) || (dt.weekday() == Weekday::Mon && !conf.first_sunday)
    }
    // The main loop function that does everything:
    // - print the current line of month columns
    // - if all months are completed, it prints the next header and intializes
    //   the next months to print
    pub fn print_next_line(
        &mut self,
        stdout: &mut StandardStream,
        counter: &HashMap<NaiveDate, u32>,
        today: NaiveDate,
        conf: &conf::Conf,
    ) -> io::Result<bool> {
        if self.page_size == 0 {
            return Ok(true);
        }
        if self.is_done() {
            return Ok(true);
        }
        if !self.started || self.is_bunch_done() {
            self.started = true;
            self.print_header(stdout, conf)?;
            return Ok(false);
        }
        let cols = self.cols.len();
        for i in 0..cols {
            let d = self.cols[i];
            match d {
                None => {
                    write!(stdout, "{}", " ".repeat(MONTH_WIDTH as usize))?;
                }
                Some(dt) => {
                    let mut dt = dt;
                    let mut wk = dt.iso_week().week();
                    if dt.weekday() == Weekday::Sun {
                        let nxdt = dt.succ_opt().unwrap_or(dt);
                        wk = nxdt.iso_week().week();
                    }

                    let mut clr = conf.fmt.colors.default_fg.clone();
                    clr.set_fg(Some(Color::Green));
                    stdout.set_color(&clr)?;
                    write!(stdout, "{wk:>3}")?;

                    let since = self.days_since_start(dt, conf);
                    if since != 0 {
                        write!(stdout, "{}", " ".repeat(since * 3))?;
                    }
                    let m = dt.month();
                    let mut printed = 0usize;
                    loop {
                        let mut clr = conf.fmt.colors.default_fg.clone();
                        if dt == today {
                            clr.set_bg(Some(Color::Blue));
                        }
                        if let Some(n) = counter.get(&dt) {
                            let fg = if n > &1 { Color::Red } else { Color::Magenta };
                            clr.set_fg(Some(fg));
                        }
                        stdout.set_color(&clr)?;

                        write!(stdout, "{:>3}", dt.day())?;
                        dt = dt.succ_opt().unwrap_or_else(|| panic!("the next date must exist for {}", dt));
                        if dt > self.end_date {
                            break;
                        }
                        if dt.month() != m {
                            break;
                        }
                        if self.is_first_day_of_week(dt, conf) {
                            break;
                        }
                        printed += 1;
                    }
                    if dt.month() != m || dt > self.end_date {
                        self.cols[i] = None;
                    } else {
                        self.cols[i] = Some(dt);
                    }
                    if printed != 7 && m != dt.month() {
                        write!(stdout, "{}", " ".repeat((7 - printed - 1) * 3))?;
                    }
                }
            }
        }
        writeln!(stdout)?;
        if self.is_bunch_done() && self.next_page() {
            self.print_header(stdout, conf)?;
        }
        Ok(self.is_done())
    }
    // Prints month and weekday names before a new months page
    fn print_header(&self, stdout: &mut StandardStream, conf: &conf::Conf) -> io::Result<()> {
        for i in 0..self.cols.len() {
            write!(stdout, "   ")?;
            let dt = next_month(self.start_date, self.first_idx + i as u32);
            let idx = (dt.month() - 1) as usize;
            let m_name = MONTH_NAMES[idx];
            self.print_centered(stdout, m_name, MONTH_WIDTH - 3)?;
        }
        writeln!(stdout)?;
        let wdays = if conf.first_sunday { " Su Mo Tu We Th Fr Sa" } else { " Mo Tu We Th Fr Sa Su" };
        for _i in 0..self.cols.len() {
            write!(stdout, "   {}", wdays)?;
        }
        writeln!(stdout)?;
        Ok(())
    }
}

// Returns the first day of a month advancing `m_delta` months ahead.
// If `m_delta` is `0`, the date is returned unchanged.
fn next_month(dt: NaiveDate, m_delta: u32) -> NaiveDate {
    if m_delta == 0 {
        return dt;
    }
    let mut y = dt.year();
    let mut m = dt.month() + m_delta;
    if m > 12 {
        y += (m / 12) as i32;
        m %= 12;
    }
    NaiveDate::from_ymd_opt(y, m, 1).expect("Failed to calculate the next month date")
}

// How many months to print for the given date range.
pub fn months_between(b_day: NaiveDate, e_day: NaiveDate) -> u32 {
    if e_day < b_day {
        return 0;
    }
    let beg = (b_day.year() as u32) * 12 + b_day.month();
    let end = (e_day.year() as u32) * 12 + e_day.month();
    end - beg + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_between_test() {
        struct Test {
            b: NaiveDate,
            e: NaiveDate,
            d: u32,
        }
        let tests: Vec<Test> = vec![
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                e: NaiveDate::from_ymd_opt(2000, 1, 8).unwrap(),
                d: 1,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 8).unwrap(),
                e: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                d: 0,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 8).unwrap(),
                e: NaiveDate::from_ymd_opt(2000, 2, 6).unwrap(),
                d: 2,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 2, 8).unwrap(),
                e: NaiveDate::from_ymd_opt(2001, 1, 6).unwrap(),
                d: 12,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 8).unwrap(),
                e: NaiveDate::from_ymd_opt(2001, 2, 6).unwrap(),
                d: 14,
            },
        ];
        for test in tests.iter() {
            let d = months_between(test.b, test.e);
            assert_eq!(test.d, d, "\n{0} - {1} = {2}, got {d}", test.e, test.b, test.d);
        }
    }
    #[test]
    fn next_month_test() {
        struct Test {
            b: NaiveDate,
            m: u32,
            e: NaiveDate,
        }
        let tests: Vec<Test> = vec![
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                e: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                m: 0,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                e: NaiveDate::from_ymd_opt(2000, 2, 1).unwrap(),
                m: 1,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                e: NaiveDate::from_ymd_opt(2000, 6, 1).unwrap(),
                m: 5,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                e: NaiveDate::from_ymd_opt(2001, 4, 1).unwrap(),
                m: 15,
            },
            Test {
                b: NaiveDate::from_ymd_opt(2000, 1, 6).unwrap(),
                e: NaiveDate::from_ymd_opt(2002, 2, 1).unwrap(),
                m: 25,
            },
        ];
        for test in tests.iter() {
            let e = next_month(test.b, test.m);
            assert_eq!(test.e, e, "{0} --> {1} = {2} != {3}", test.b, test.m, test.e, e);
        }
    }
}
