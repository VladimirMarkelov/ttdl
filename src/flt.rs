use chrono::NaiveDate;
use regex::Regex;
use todo_lib::*;

use crate::conv::{str_to_bytes, str_to_duration, str_to_time};
use crate::human_date;

fn filter_type_by_tag(tag: &str) -> ValueType {
    if tag.ends_with("_time") {
        ValueType::Time
    } else if tag.ends_with("_date") || tag == "due" || tag == "t" {
        ValueType::Date
    } else if tag == "spent" || tag.ends_with("_dur") || tag.ends_with("_duration") {
        ValueType::Duration
    } else if tag.ends_with("_size") || tag.ends_with("_sz") {
        ValueType::Size
    } else {
        ValueType::Unknown
    }
}
fn filter_type_by_value(st: Option<&String>) -> ValueType {
    if let Some(s_orig) = st {
        let s = s_orig.to_lowercase();
        let rx_date = Regex::new(r"^\d\d\d\d-\d\d-\d\d$").unwrap();
        let rx_duration = Regex::new(r"^(\d+w)?(\d+d)?(\d+h)?(\d+m)?(\d+s)?$").unwrap();
        let rx_size = Regex::new(r"^(\d+([ptgmk]i?b?)|(\d+b))$").unwrap();
        let rx_time = Regex::new(r"^\d{3,4}(pm|am)$").unwrap();
        if s.parse::<i64>().is_ok() {
            ValueType::Integer
        } else if s.parse::<f64>().is_ok() {
            ValueType::Float
        } else if rx_date.is_match(s.as_str()) {
            ValueType::Date
        } else if rx_duration.is_match(s.as_str()) {
            ValueType::Duration
        } else if rx_size.is_match(s.as_str()) {
            ValueType::Size
        } else if rx_time.is_match(s.as_str()) {
            ValueType::Time
        } else {
            ValueType::String
        }
    } else {
        ValueType::String
    }
}

fn str_to_date(s: &str, base: NaiveDate) -> Result<NaiveDate, i32> {
    if let Ok(d) = human_date::human_to_date(base, s, 7) {
        Ok(d)
    } else {
        match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            Ok(d) => Ok(d),
            _ => Err(1),
        }
    }
}

fn is_negative(s: &str) -> bool {
    s.starts_with('-') || s.starts_with('!')
}

fn values_equal(t_val: &str, f_val: &str, t: ValueType, base: NaiveDate) -> bool {
    match t {
        ValueType::Date => {
            let res = str_to_date(f_val, base);
            let f_date = if let Ok(h_date) = res { todotxt::format_date(h_date) } else { f_val.to_string() };
            t_val == f_date.as_str()
        }
        ValueType::Size => {
            let t_res = str_to_bytes(t_val);
            let f_res = str_to_bytes(f_val);
            match (t_res, f_res) {
                (Some(tv), Some(fv)) => tv == fv,
                _ => false,
            }
        }
        ValueType::Duration => {
            let t_res = str_to_duration(t_val);
            let f_res = str_to_duration(f_val);
            match (t_res, f_res) {
                (Some(tv), Some(fv)) => tv == fv,
                _ => false,
            }
        }
        ValueType::Float => {
            let t_res = t_val.parse::<f64>();
            let f_res = f_val.parse::<f64>();
            match (t_res, f_res) {
                (Ok(tv), Ok(fv)) => tv == fv,
                _ => false,
            }
        }
        ValueType::Integer => {
            let t_res = t_val.parse::<i64>();
            let f_res = f_val.parse::<i64>();
            match (t_res, f_res) {
                (Ok(tv), Ok(fv)) => tv == fv,
                _ => false,
            }
        }
        _ => t_val == f_val,
    }
}

fn values_compare(t_val: &str, f_val: &str, t: ValueType, base: NaiveDate, less_eq: bool) -> bool {
    match t {
        ValueType::Date => {
            let t_res = str_to_date(t_val, base);
            let f_res = str_to_date(f_val, base);
            match (t_res, f_res) {
                (Ok(tv), Ok(fv)) => {
                    if less_eq {
                        tv <= fv
                    } else {
                        tv >= fv
                    }
                }
                _ => false,
            }
        }
        ValueType::Size => {
            let t_res = str_to_bytes(t_val);
            let f_res = str_to_bytes(f_val);
            match (t_res, f_res) {
                (Some(tv), Some(fv)) => {
                    if less_eq {
                        tv <= fv
                    } else {
                        tv >= fv
                    }
                }
                _ => false,
            }
        }
        ValueType::Duration => {
            let t_res = str_to_duration(t_val);
            let f_res = str_to_duration(f_val);
            match (t_res, f_res) {
                (Some(tv), Some(fv)) => {
                    if less_eq {
                        tv <= fv
                    } else {
                        tv >= fv
                    }
                }
                _ => false,
            }
        }
        ValueType::String => {
            if less_eq {
                t_val <= f_val
            } else {
                t_val >= f_val
            }
        }
        ValueType::Integer => {
            let t_res = t_val.parse::<i64>();
            let f_res = f_val.parse::<i64>();
            match (t_res, f_res) {
                (Ok(tv), Ok(fv)) => {
                    if less_eq {
                        tv <= fv
                    } else {
                        tv >= fv
                    }
                }
                _ => false,
            }
        }
        ValueType::Float => {
            let t_res = t_val.parse::<f64>();
            let f_res = f_val.parse::<f64>();
            match (t_res, f_res) {
                (Ok(tv), Ok(fv)) => {
                    if less_eq {
                        tv <= fv
                    } else {
                        tv >= fv
                    }
                }
                _ => false,
            }
        }
        ValueType::Time => {
            let t_res = str_to_time(t_val);
            let f_res = str_to_time(f_val);
            match (t_res, f_res) {
                (Some(tv), Some(fv)) => {
                    if less_eq {
                        tv <= fv
                    } else {
                        tv >= fv
                    }
                }
                _ => false,
            }
        }
        ValueType::Unknown => false,
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum ValueType {
    Unknown,
    Integer,
    Float,
    String,
    Date,
    Time,
    Duration,
    Size,
}

pub enum FilterCond {
    Range(String, String),
    One(String),
}

impl FilterCond {
    pub fn matches(&self, value: Option<&String>, t: ValueType, base: NaiveDate) -> bool {
        match self {
            FilterCond::One(self_value) => match value {
                None => self_value == "none" || self_value == "-",
                Some(s) => {
                    let is_negative = is_negative(self_value);
                    let rule_value = if is_negative { &self_value[1..] } else { &self_value[..] };

                    if self_value == "-" {
                        false
                    } else if rule_value.is_empty() {
                        true
                    } else if rule_value == "any" {
                        !is_negative
                    } else if self_value == "none" {
                        is_negative
                    } else {
                        let equals = values_equal(s, rule_value, t, base);
                        if is_negative { !equals } else { equals }
                    }
                }
            },
            FilterCond::Range(bg, en) => match value {
                None => bg == "none" || en == "none",
                Some(s) => {
                    if bg == "none" && en == "none" && s == "none" {
                        true
                    } else if bg == "none" && en == "none" {
                        false
                    } else if bg.is_empty() || bg == "none" {
                        values_compare(s, en, t, base, true)
                    } else if en.is_empty() || en == "none" {
                        values_compare(s, bg, t, base, false)
                    } else {
                        values_compare(s, en, t, base, true) && values_compare(s, bg, t, base, false)
                    }
                }
            },
        }
    }
}

pub struct FilterRule {
    pub tag: String,
    pub flt: Vec<FilterCond>,
}

impl FilterRule {
    pub fn matches(&self, task: &todotxt::Task, base: NaiveDate) -> bool {
        let is_negative = is_negative(&self.tag);
        let tag_full_name = self.tag.as_str();
        let tag_name = if is_negative { &tag_full_name[1..] } else { tag_full_name };
        let tag_opt = task.tags.get(tag_name);

        let vt = if tag_opt.is_none() {
            ValueType::String
        } else {
            let t = filter_type_by_tag(tag_name);
            if t == ValueType::Unknown { filter_type_by_value(tag_opt) } else { t }
        };
        let mut matched = false;
        for cond in &self.flt {
            matched |= cond.matches(tag_opt, vt, base);
        }
        if is_negative { !matched } else { matched }
    }
}

pub struct Filter {
    pub rules: Vec<FilterRule>,
}

impl Filter {
    pub fn parse(s: &str) -> Filter {
        let mut rules: Vec<FilterRule> = Vec::new();
        for rl in s.split(';') {
            if rl.is_empty() {
                continue;
            }
            let rl_values: Vec<&str> = rl.splitn(2, '=').collect();
            let tag_name = rl_values[0].to_string();
            // Case when only tag name is set. In this case filter only ones that have the tag
            if rl_values.len() == 1 {
                rules.push(FilterRule { tag: tag_name, flt: vec![FilterCond::One("any".to_string())] });
            } else {
                let mut values: Vec<FilterCond> = Vec::new();
                for rl_value in rl_values[1].split(',') {
                    if rl_value.is_empty() {
                        continue;
                    }
                    let items: Vec<&str> = rl_value.splitn(2, "..").collect();
                    if items.len() == 1 {
                        values.push(FilterCond::One(items[0].to_string()));
                    } else {
                        values.push(FilterCond::Range(items[0].to_string(), items[1].to_string()));
                    }
                }
                rules.push(FilterRule { tag: tag_name, flt: values });
            }
        }
        Filter { rules }
    }
    pub fn matches(&self, task: &todotxt::Task, base: NaiveDate) -> bool {
        for rule in &self.rules {
            if !rule.matches(task, base) {
                return false;
            }
        }
        true
    }
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use todo_lib::todotxt;

    #[test]
    fn filter_test() {
        let tasks: Vec<&'static str> = vec![
            "test2 pri:B id:11",
            "test1 val:-2 pri:A id:10",
            "test3 pri:C id:12",
            "test4 val:10 id:13",
            "test5 val:15 id:14",
        ];
        let mut task_vec: Vec<todotxt::Task> = Vec::new();
        let base = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
        for t in &tasks {
            let task = todotxt::Task::parse(t, base);
            task_vec.push(task);
        }

        struct Test {
            f: &'static str,
            o: Vec<usize>,
        }
        let tests: Vec<Test> = vec![
            Test { f: "pri", o: vec![1, 2, 3] },
            Test { f: "!pri", o: vec![4, 5] },
            Test { f: "-pri", o: vec![4, 5] },
            Test { f: "val=-2..-2", o: vec![2] },
            Test { f: "-val=-2..-2", o: vec![1, 3, 4, 5] },
            Test { f: "pri=-C", o: vec![1, 2] },
            Test { f: "pri=-C", o: vec![1, 2] },
            Test { f: "pri=B,C;id=-12", o: vec![1] },
            Test { f: "pri=B,C;id=12..17", o: vec![3] },
            Test { f: "id=11..14", o: vec![1, 3, 4, 5] },
            Test { f: "-id=11..14", o: vec![2] },
            Test { f: "-pri=B,C;id=-12", o: vec![2, 4, 5] },
        ];
        for (idx, t) in tests.iter().enumerate() {
            let flt = Filter::parse(t.f);
            assert!(!flt.is_empty(), "Failed to parse {0}", t.f);
            let mut res: Vec<usize> = Vec::new();
            for (idx, task) in task_vec.iter().enumerate() {
                if flt.matches(task, base) {
                    res.push(idx + 1);
                }
            }
            assert_eq!(res, t.o, "{idx}. {0}: expected {1:?}, got {2:?}", t.f, t.o, res);
        }
    }
}
