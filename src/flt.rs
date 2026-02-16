use chrono::NaiveDate;
use regex::Regex;
use todo_lib::*;

use crate::conv::{str_to_bytes, str_to_duration, str_to_time};
use crate::human_date;

const DATE_TAGS: [&str; 3] = ["started", "finished", "completed"];
const STR_TAGS: [&str; 11] =
    ["pri", "priority", "@", "ctx", "context", "+", "prj", "project", "proj", "subj", "subject"];
const INT_TAGS: [&str; 2] = ["ID", "done"];

fn filter_type_by_tag(tag: &str) -> ValueType {
    if tag.ends_with("_time") {
        ValueType::Time
    } else if tag.ends_with("_date") || tag == "due" || tag == "t" {
        ValueType::Date
    } else if tag == "spent" || tag.ends_with("_dur") || tag.ends_with("_duration") {
        ValueType::Duration
    } else if tag.ends_with("_size") || tag.ends_with("_sz") {
        ValueType::Size
    } else if DATE_TAGS.contains(&tag) {
        ValueType::Date
    } else if STR_TAGS.contains(&tag) || tag.starts_with('#') {
        ValueType::String
    } else if INT_TAGS.contains(&tag) {
        ValueType::Integer
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

fn str_match(f_val: &str, t_val: &str, use_regex: bool) -> bool {
    if use_regex {
        let rx = match Regex::new(&format!("(?i){f_val}")) {
            Err(e) => {
                eprintln!("Invalid regex: {e}");
                return false;
            }
            Ok(v) => v,
        };
        return rx.is_match(t_val);
    }
    let f_val = f_val.to_lowercase();
    let t_val = t_val.to_lowercase();
    let left = f_val.starts_with('*');
    let right = f_val.ends_with('*');
    if !left && !right {
        f_val == t_val
    } else if left && right {
        let v = f_val.trim_matches('*');
        t_val.contains(v)
    } else if left {
        let v = f_val.trim_matches('*');
        t_val.ends_with(v)
    } else {
        let v = f_val.trim_matches('*');
        t_val.starts_with(v)
    }
}

fn values_equal(t_val: &str, f_val: &str, t: ValueType, base: NaiveDate, use_regex: bool) -> bool {
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
        _ => str_match(f_val, t_val, use_regex),
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

fn match_none(s: &str) -> bool {
    s == "none" || s == "-"
}

fn match_none_or_empty(s: &str) -> bool {
    s == "none" || s == "-" || s.is_empty()
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
    pub fn matches(
        &self,
        id: usize,
        name: &str,
        task: &todotxt::Task,
        t: ValueType,
        base: NaiveDate,
        use_regex: bool,
    ) -> bool {
        match name {
            "ID" => match self {
                FilterCond::One(self_value) => {
                    if self_value == "-" || self_value == "none" {
                        return false;
                    };
                    let is_negative = is_negative(self_value);
                    let rule_value = if is_negative { &self_value[1..] } else { &self_value[..] };
                    if rule_value == "any" {
                        return !is_negative;
                    } else if rule_value == "none" {
                        return is_negative;
                    }
                    let eq = compare_usize(id, rule_value, Operation::Eq);
                    if is_negative { !eq } else { eq }
                }
                FilterCond::Range(bg, en) => {
                    if match_none(bg) && match_none(en) {
                        false
                    } else if match_none_or_empty(bg) {
                        compare_usize(id, en, Operation::Ls)
                    } else if match_none_or_empty(en) {
                        compare_usize(id, bg, Operation::Gt)
                    } else {
                        compare_usize(id, bg, Operation::Gt) && compare_usize(id, en, Operation::Ls)
                    }
                }
            },
            "pri" | "priority" => match self {
                FilterCond::One(self_value) => {
                    if match_none(self_value) {
                        return task.priority == todotxt::NO_PRIORITY;
                    };
                    if self_value == "any" {
                        return task.priority != todotxt::NO_PRIORITY;
                    };
                    let is_negative = is_negative(self_value);
                    let rule_value = if is_negative { &self_value[1..] } else { &self_value[..] };
                    if rule_value == "any" {
                        return !is_negative;
                    } else if rule_value == "none" {
                        return is_negative;
                    }
                    let rule_pri = todotxt::str_to_priority(rule_value);
                    if rule_pri == todotxt::NO_PRIORITY {
                        false
                    } else {
                        let matched = rule_pri == task.priority;
                        if is_negative {
                            task.priority != todotxt::NO_PRIORITY && !matched
                        } else {
                            task.priority != todotxt::NO_PRIORITY && matched
                        }
                    }
                }
                FilterCond::Range(bg, en) => {
                    if match_none(bg) && match_none(en) {
                        task.priority == todotxt::NO_PRIORITY
                    } else if bg.is_empty() && en.is_empty() {
                        task.priority != todotxt::NO_PRIORITY
                    } else if match_none_or_empty(bg) {
                        let rule_en = todotxt::str_to_priority(en);
                        (rule_en != todotxt::NO_PRIORITY
                            && task.priority <= rule_en
                            && task.priority != todotxt::NO_PRIORITY)
                            || (task.priority == todotxt::NO_PRIORITY && !bg.is_empty())
                    } else if match_none_or_empty(en) {
                        let rule_bg = todotxt::str_to_priority(bg);
                        (rule_bg != todotxt::NO_PRIORITY
                            && task.priority >= rule_bg
                            && task.priority != todotxt::NO_PRIORITY)
                            || (task.priority == todotxt::NO_PRIORITY && !en.is_empty())
                    } else {
                        let rule_en = todotxt::str_to_priority(en);
                        let rule_bg = todotxt::str_to_priority(bg);
                        rule_en != todotxt::NO_PRIORITY
                            && rule_bg != todotxt::NO_PRIORITY
                            && task.priority != todotxt::NO_PRIORITY
                            && task.priority >= rule_bg
                            && task.priority <= rule_en
                    }
                }
            },
            "done" => match self {
                FilterCond::One(self_value) => {
                    if self_value.is_empty() || self_value == "any" {
                        task.finished
                    } else if match_none(self_value) {
                        !task.finished
                    } else {
                        false
                    }
                }
                FilterCond::Range(_bg, _en) => false,
            },
            "subj" | "subject" => {
                match self {
                    FilterCond::One(self_value) => {
                        let is_negative = is_negative(self_value);
                        let rule_value = if is_negative { &self_value[1..] } else { &self_value[..] };
                        let matched = if !use_regex && !rule_value.starts_with('*') && !rule_value.ends_with('*') {
                            let rule_value = format!("*{rule_value}*");
                            str_match(&rule_value, &task.subject, use_regex)
                        } else {
                            str_match(rule_value, &task.subject, use_regex)
                        };
                        if is_negative { !matched } else { matched }
                    }
                    FilterCond::Range(_bg, _en) => {
                        // Range does not make sense for subject
                        false
                    }
                }
            }
            "@" | "+" | "#" | "prj" | "project" | "ctx" | "context" | "hashtag" => {
                let values = if name == "@" || name == "ctx" || name == "context" {
                    &task.contexts
                } else if name == "+" || name == "prj" || name == "project" {
                    &task.projects
                } else {
                    &task.hashtags
                };
                match self {
                    FilterCond::One(self_value) => {
                        if match_none(self_value) {
                            values.is_empty()
                        } else if self_value == "any" {
                            !values.is_empty()
                        } else {
                            let is_negative = is_negative(self_value);
                            let rule_value = if is_negative { &self_value[1..] } else { &self_value[..] };
                            let mut all_matched = !values.is_empty();
                            for val in values {
                                let matched = str_match(rule_value, val, use_regex);
                                if is_negative {
                                    all_matched = all_matched && !matched;
                                } else {
                                    all_matched = all_matched && matched
                                }
                            }
                            all_matched
                        }
                    }
                    FilterCond::Range(_be, _en) => {
                        // Range does not make sense for projects, contexts and hashtags
                        false
                    }
                }
            }
            "completed" | "created" | "create" => {
                let date = if name == "created" || name == "create" { task.create_date } else { task.finish_date };
                match self {
                    FilterCond::One(self_value) => match date {
                        None => match_none(self_value),
                        Some(dt) => {
                            if match_none(self_value) {
                                false
                            } else if self_value == "any" || self_value.is_empty() {
                                true
                            } else {
                                let is_negative = is_negative(self_value);
                                let rule_value = if is_negative { &self_value[1..] } else { &self_value[..] };
                                let matched = compare_dates(dt, rule_value, Operation::Eq, base);
                                if is_negative { !matched } else { matched }
                            }
                        }
                    },
                    FilterCond::Range(bg, en) => match date {
                        None => match_none(bg) || match_none(en),
                        Some(dt) => {
                            if match_none(bg) && match_none(en) {
                                date.is_none()
                            } else if match_none_or_empty(bg) {
                                compare_dates(dt, en, Operation::Ls, base)
                            } else if match_none_or_empty(en) {
                                compare_dates(dt, bg, Operation::Gt, base)
                            } else {
                                compare_dates(dt, en, Operation::Ls, base) && compare_dates(dt, bg, Operation::Gt, base)
                            }
                        }
                    },
                }
            }
            _ => {
                let value = task.tags.get(name);
                match self {
                    FilterCond::One(self_value) => match value {
                        None => match_none(self_value),
                        Some(s) => {
                            let t = if self_value.contains('*') { ValueType::String } else { t };
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
                                let equals = values_equal(s, rule_value, t, base, use_regex);
                                if is_negative { !equals } else { equals }
                            }
                        }
                    },
                    FilterCond::Range(bg, en) => match value {
                        None => match_none(bg) || match_none(en),
                        Some(s) => {
                            if bg == "none" && en == "none" && s == "none" {
                                true
                            } else if match_none(bg) && match_none(en) {
                                false
                            } else if match_none_or_empty(bg) {
                                values_compare(s, en, t, base, true)
                            } else if match_none_or_empty(en) {
                                values_compare(s, bg, t, base, false)
                            } else {
                                values_compare(s, en, t, base, true) && values_compare(s, bg, t, base, false)
                            }
                        }
                    },
                }
            }
        }
    }
}

#[derive(PartialEq)]
enum Operation {
    Eq,
    Ls,
    Gt,
}
fn compare_usize(val: usize, f_value: &str, op: Operation) -> bool {
    if f_value == "any" {
        return true;
    }
    if f_value == "none" || f_value == "-" || f_value.is_empty() {
        return op == Operation::Eq || f_value.is_empty();
    }
    match f_value.parse::<usize>() {
        Err(_) => false,
        Ok(v) => match op {
            Operation::Eq => v == val,
            Operation::Ls => val <= v,
            Operation::Gt => val >= v,
        },
    }
}

fn compare_dates(val: NaiveDate, f_value: &str, op: Operation, base: NaiveDate) -> bool {
    if f_value == "any" {
        return true;
    }
    if f_value == "none" || f_value == "-" || f_value.is_empty() {
        return op == Operation::Eq;
    }
    match str_to_date(f_value, base) {
        Err(_) => false,
        Ok(d) => match op {
            Operation::Eq => d == val,
            Operation::Ls => val <= d,
            Operation::Gt => val >= d,
        },
    }
}

pub struct FilterRule {
    pub tag: String,
    pub flt: Vec<FilterCond>,
}

impl FilterRule {
    pub fn matches(&self, task: &todotxt::Task, id: usize, base: NaiveDate, use_regex: bool) -> bool {
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
            matched |= cond.matches(id, tag_name, task, vt, base, use_regex);
        }
        if is_negative { !matched } else { matched }
    }
}

pub struct Filter {
    use_regex: bool,
    pub rules: Vec<FilterRule>,
}

impl Filter {
    pub fn parse(s: &str, use_regex: bool) -> Filter {
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
                    let mut items: Vec<&str> = rl_value.splitn(2, "..").collect();
                    if rl_value.ends_with("..") {
                        items.push(&rl_value[..0]);
                    }
                    if items.len() == 1 {
                        values.push(FilterCond::One(items[0].to_string()));
                    } else {
                        values.push(FilterCond::Range(items[0].to_string(), items[1].to_string()));
                    }
                }
                rules.push(FilterRule { tag: tag_name, flt: values });
            }
        }
        Filter { use_regex, rules }
    }
    pub fn matches(&self, task: &todotxt::Task, id: usize, base: NaiveDate) -> bool {
        for rule in &self.rules {
            if !rule.matches(task, id, base, self.use_regex) {
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
    fn basic_filter_test() {
        let tasks: Vec<&'static str> = vec![
            "test2 pr:B id:11",
            "test1 val:-2 pr:A id:10",
            "test3 pr:C id:12",
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
            Test { f: "pr", o: vec![1, 2, 3] },
            Test { f: "!pr", o: vec![4, 5] },
            Test { f: "-pr", o: vec![4, 5] },
            Test { f: "val=-2..-2", o: vec![2] },
            Test { f: "-val=-2..-2", o: vec![1, 3, 4, 5] },
            Test { f: "pr=-C", o: vec![1, 2] },
            Test { f: "pr=-C", o: vec![1, 2] },
            Test { f: "pr=B,C;id=-12", o: vec![1] },
            Test { f: "pr=B,C;id=12..17", o: vec![3] },
            Test { f: "id=11..14", o: vec![1, 3, 4, 5] },
            Test { f: "-id=11..14", o: vec![2] },
            Test { f: "-pr=B,C;id=-12", o: vec![2, 4, 5] },
        ];
        for (idx, t) in tests.iter().enumerate() {
            let flt = Filter::parse(t.f, false);
            assert!(!flt.is_empty(), "Failed to parse {0}", t.f);
            let mut res: Vec<usize> = Vec::new();
            for (idx, task) in task_vec.iter().enumerate() {
                if flt.matches(task, idx, base) {
                    res.push(idx + 1);
                }
            }
            assert_eq!(res, t.o, "{idx}. {0}: expected {1:?}, got {2:?}", t.f, t.o, res);
        }
    }

    #[test]
    fn ext_filter_test() {
        let tasks: Vec<&'static str> = vec![
            "(B) test2 #this",
            "x 2020-07-09 2020-07-12 test1 @ctx1 id:250",
            "x (D) 2020-07-08 2020-07-15 test3 something @ctx2",
            "(C) test4 anything #that id:251",
            "(A) 2020-07-14 test5 +proj1",
            "test5 +proj2 @ctx2 id:300",
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
            Test { f: "pri=B,C", o: vec![1, 4] },
            Test { f: "pri=A..B", o: vec![1, 5] },
            Test { f: "pri=none..B", o: vec![1, 2, 5, 6] },
            Test { f: "pri=..B", o: vec![1, 5] },
            Test { f: "pri=B..", o: vec![1, 3, 4] },
            Test { f: "pri=-B", o: vec![3, 4, 5] },
            Test { f: "-pri=B", o: vec![2, 3, 4, 5, 6] },
            Test { f: "pri=any;-pri=A,C", o: vec![1, 3] },
            Test { f: "ID=-", o: vec![] },
            Test { f: "ID=any", o: vec![1, 2, 3, 4, 5, 6] },
            Test { f: "ID=2..3", o: vec![2, 3] },
            Test { f: "-ID=2..3", o: vec![1, 4, 5, 6] },
            Test { f: "ID=-5", o: vec![1, 2, 3, 4, 6] },
            Test { f: "-ID=2,5", o: vec![1, 3, 4, 6] },
            Test { f: "ID=2,5", o: vec![2, 5] },
            Test { f: "subj=test4", o: vec![4] },
            Test { f: "subject=test4,test5", o: vec![4, 5, 6] },
            Test { f: "subj=-", o: vec![] },
            Test { f: "subj=any", o: vec![4] },
            Test { f: "subj=none", o: vec![] },
            Test { f: "#=that", o: vec![4] },
            Test { f: "#=th*", o: vec![1, 4] },
            Test { f: "#=*is", o: vec![1] },
            Test { f: "#=this,that", o: vec![1, 4] },
            Test { f: "#=-that", o: vec![1] },
            Test { f: "@=ctx1,ctx2,ctx3", o: vec![2, 3, 6] },
            Test { f: "ctx=ctx1,ctx2,ctx3", o: vec![2, 3, 6] },
            Test { f: "context=ctx1,ctx2,ctx3", o: vec![2, 3, 6] },
            Test { f: "@=ctx*,ctx3;@=-ctx2", o: vec![2] },
            Test { f: "+=proj2,proj1", o: vec![5, 6] },
            Test { f: "+=proj2;@=ctx2", o: vec![6] },
            Test { f: "done", o: vec![2, 3] },
            Test { f: "done=any", o: vec![2, 3] },
            Test { f: "done=-", o: vec![1, 4, 5, 6] },
            Test { f: "done=none", o: vec![1, 4, 5, 6] },
            Test { f: "created=..2020-08-01", o: vec![2, 3, 5] },
            Test { f: "created=none..2020-07-14", o: vec![1, 2, 4, 5, 6] },
            Test { f: "created=2020-07-14..", o: vec![3, 5] },
            Test { f: "created=2020-07-14..none", o: vec![1, 3, 4, 5, 6] },
            Test { f: "-created=2020-07-14..", o: vec![1, 2, 4, 6] },
            Test { f: "created=any", o: vec![2, 3, 5] },
            Test { f: "created", o: vec![2, 3, 5] },
            Test { f: "created=none", o: vec![1, 4, 6] },
            Test { f: "created=-", o: vec![1, 4, 6] },
            Test { f: "completed=any", o: vec![2, 3] },
            Test { f: "id=250", o: vec![2] },
            Test { f: "id=-250", o: vec![4, 6] },
            Test { f: "id=240..251", o: vec![2, 4] },
            Test { f: "-id=240..251", o: vec![1, 3, 5, 6] },
            Test { f: "id=25*", o: vec![2, 4] },
            Test { f: "id=..251", o: vec![2, 4] },
            Test { f: "id=251..", o: vec![4, 6] },
            Test { f: "id=none..251", o: vec![1, 2, 3, 4, 5] },
        ];
        for (idx, t) in tests.iter().enumerate() {
            let flt = Filter::parse(t.f, false);
            assert!(!flt.is_empty(), "Failed to parse {0}", t.f);
            let mut res: Vec<usize> = Vec::new();
            for (idx, task) in task_vec.iter().enumerate() {
                if flt.matches(task, idx + 1, base) {
                    res.push(idx + 1);
                }
            }
            assert_eq!(res, t.o, "{idx}. {0}: expected {1:?}, got {2:?}", t.f, t.o, res);
        }
    }
}
