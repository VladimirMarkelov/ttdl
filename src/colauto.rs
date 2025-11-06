use caseless::default_caseless_match_str;
use todo_lib::{timer, todo, todotxt};
use unicode_width::UnicodeWidthStr;

use crate::fmt::{Conf, done_str, duration_str, format_relative_date, number_of_digits, priority_str};
use crate::subj_clean::{hide_contexts, hide_projects, hide_tags};

// Through the entire module `field` is either a tag or special task field like `priority` or `project`.

fn fmt_date(v: Option<chrono::NaiveDate>, field: &str, c: &Conf) -> String {
    match v {
        None => String::new(),
        Some(dt) => {
            if c.is_human(field) {
                let (s, _) = format_relative_date(dt, c.compact);
                s
            } else {
                dt.format("%Y-%m-%d").to_string()
            }
        }
    }
}

fn get_field(task: &todotxt::Task, field: &str, c: &Conf) -> Option<String> {
    if !has_field(task, field) {
        return None;
    }
    match field {
        "done" => Some(done_str(task)),
        "pri" => Some(priority_str(task)),
        "created" => Some(fmt_date(task.create_date, field, c)),
        "finished" => Some(fmt_date(task.finish_date, field, c)),
        "due" => Some(fmt_date(task.due_date, field, c)),
        "thr" => Some(fmt_date(task.threshold_date, field, c)),
        "ctx" => {
            let mut s = String::new();
            for ctx in &task.contexts {
                if !s.is_empty() {
                    s += ",";
                }
                s += ctx;
            }
            Some(s)
        }
        "prj" => {
            let mut s = String::new();
            for prj in &task.projects {
                if !s.is_empty() {
                    s += ",";
                }
                s += prj;
            }
            Some(s)
        }
        "spent" => Some(duration_str(timer::spent_time(task))),
        key => task.tags.get(key).cloned(),
    }
}

fn has_field(task: &todotxt::Task, field: &str) -> bool {
    match field {
        "done" => task.finished || task.recurrence.is_some(),
        "pri" => task.priority != todotxt::NO_PRIORITY,
        "created" => task.create_date.is_some(),
        "finished" => task.finish_date.is_some(),
        "due" => task.due_date.is_some(),
        "thr" => task.threshold_date.is_some(),
        "ctx" => !task.contexts.is_empty(),
        "prj" => !task.projects.is_empty(),
        key => task.tags.contains_key(key),
    }
}

fn is_field_empty(tasks: &todo::TaskSlice, ids: &todo::IDSlice, field: &str) -> bool {
    ids.iter().all(|&id| !has_field(&tasks[id], field))
}

fn header_width(field: &str) -> usize {
    match field {
        "done" | "pri" => 1,
        "thr" => "threshold".width(),
        "prj" => "project".width(),
        "ctx" => "context".width(),
        _ => field.width(),
    }
}

fn max_field_width(tasks: &todo::TaskSlice, ids: &todo::IDSlice, field: &str, fields: &[&str], c: &Conf) -> usize {
    let mut max = 0;
    for id in ids.iter() {
        if *id >= tasks.len() {
            continue;
        }
        let w = if default_caseless_match_str(field, "id") {
            number_of_digits(*id)
        } else if default_caseless_match_str(field, "subject") {
            let mut desc = tasks[*id].subject.clone();
            cleanup_description(&mut desc, fields, c);
            desc.width()
        } else if let Some(val) = get_field(&tasks[*id], field, c) {
            val.trim().width()
        } else {
            0
        };
        if w > max {
            max = w;
        }
    }
    let hw = header_width(field);
    if max == 0 || hw > max { hw } else { max }
}

// Removes from `fields` all fields that are empty for all todos
pub fn filter_non_empty(tasks: &todo::TaskSlice, ids: &todo::IDSlice, fields: &[&str]) -> Vec<String> {
    let mut res = Vec::new();
    for field in fields.iter() {
        if !is_field_empty(tasks, ids, field) {
            res.push(field.to_string());
        }
    }
    res
}

pub fn cleanup_description(desc: &mut String, fields: &[&str], c: &Conf) {
    for f in fields.iter() {
        match *f {
            "id" | "done" | "pri" | "created" | "finished" => continue,
            "thr" => hide_tags(desc, "t", c),
            "spent" => {
                hide_tags(desc, "tmr", c);
                hide_tags(desc, "spent", c)
            }
            "ctx" => hide_contexts(desc, c),
            "prj" => hide_projects(desc, c),
            fname => {
                if let Some(fld) = c.custom_field(fname) {
                    hide_tags(desc, &fld.name, c)
                } else {
                    hide_tags(desc, fname, c)
                }
            }
        }
    }
}

// Calculate them maximum width of all fields in `fields` list.
pub fn col_widths(tasks: &todo::TaskSlice, ids: &todo::IDSlice, fields: &[&str], c: &Conf) -> Vec<usize> {
    let mut widths = Vec::new();
    for field in fields.iter() {
        let w = max_field_width(tasks, ids, field, fields, c);
        widths.push(w);
    }
    let mut subj_found = false;
    let mut subj_idx = 0usize;
    for (idx, f) in fields.iter().enumerate() {
        if default_caseless_match_str(f, "subject") {
            subj_found = true;
            subj_idx = idx;
            break;
        }
    }
    if subj_found && c.width != 0 {
        let mut total_width = 0;
        for (f, w) in fields.iter().zip(widths.iter()) {
            if !default_caseless_match_str(f, "subject") {
                total_width += w;
            }
        }
        if total_width + 10 < c.width.into() {
            widths[subj_idx] = usize::from(c.width) - total_width;
        }
    }
    widths
}

// Iterate through `tasks` and extract field names which values are non-empty.
// User-calculated tags, which start with '!', are ignored.
pub fn collect_non_empty(tasks: &todo::TaskSlice, ids: &todo::IDSlice) -> Vec<String> {
    let mut res = Vec::new();
    let builtin = ["done", "pri", "created", "finished", "due", "thr", "spent", "prj", "ctx"];
    for bf in &builtin {
        if !is_field_empty(tasks, ids, bf) {
            res.push(bf.to_string());
        }
    }
    for id in ids.iter() {
        let t = &tasks[*id];
        for (k, v) in &t.tags {
            match k.as_str() {
                "due" | "t" | "spent" => continue,
                _ if k.starts_with('!') => continue,
                _ => {
                    if !v.is_empty() && !res.iter().any(|it| it == k) {
                        res.push(k.to_string());
                    }
                }
            }
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt;
    use chrono::NaiveDate;
    use todo_lib::todotxt;

    #[test]
    fn hide_field_test() {
        struct Test {
            i: &'static str,
            fin: Vec<&'static str>,
            fout: Vec<&'static str>,
        }
        let tests: Vec<Test> = vec![
            Test {
                i: "x test due:2020-10-10",
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec!["done", "due"],
            },
            Test {
                i: "(B) test due:2020-10-10",
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec!["pri", "due"],
            },
            Test {
                i: "2020-10-10 test t:2020-10-10",
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec!["created", "thr"],
            },
            Test {
                i: "x 2020-10-10 2020-10-10 test due:2020-10-10",
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec!["done", "created", "finished", "due"],
            },
            Test {
                i: "x (A) 2020-10-10 2020-10-10 t:2020-10-10 test due:2020-10-10",
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec!["pri", "done", "created", "finished", "due", "thr"],
            },
            Test {
                i: "test tag:2020-10-10",
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec![],
            },
        ];
        let base = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
        for (idx, test) in tests.iter().enumerate() {
            let task = todotxt::Task::parse(test.i, base);
            let tasks: todo::TaskVec = vec![task.clone()];
            let ids: Vec<usize> = vec![0];
            let fields = filter_non_empty(&tasks, &ids, &test.fin);
            assert_eq!(
                fields.len(),
                test.fout.len(),
                "{}. '{}' != '{}'\n{:?}\n{:?}",
                idx,
                test.fout.len(),
                fields.len(),
                fields,
                task
            );
            for (idxf, f) in fields.iter().enumerate() {
                assert_eq!(f, test.fout[idxf], "{}. {} != {}", idx, f, test.fout[idxf]);
            }
        }
    }

    #[test]
    fn collect_field_test() {
        struct Test {
            i: &'static str,
            fout: Vec<&'static str>,
        }
        let tests: Vec<Test> = vec![
            Test { i: "x test due:2020-10-10", fout: vec!["done", "due"] },
            Test { i: "(B) test due:2020-10-10", fout: vec!["pri", "due"] },
            Test { i: "2020-10-10 test t:2020-10-10", fout: vec!["created", "thr"] },
            Test { i: "x 2020-10-10 2020-10-10 test due:2020-10-10", fout: vec!["done", "created", "finished", "due"] },
            Test {
                i: "x (A) 2020-10-10 2020-10-10 t:2020-10-10 test due:2020-10-10",
                fout: vec!["done", "pri", "created", "finished", "due", "thr"],
            },
            Test { i: "x tag:one test tag:2020", fout: vec!["done", "tag"] },
            Test { i: "empty tags test", fout: vec![] },
        ];
        let base = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
        for (idx, test) in tests.iter().enumerate() {
            let task = todotxt::Task::parse(test.i, base);
            let tasks: todo::TaskVec = vec![task.clone()];
            let ids: Vec<usize> = vec![0];
            let fields = collect_non_empty(&tasks, &ids);
            assert_eq!(
                fields.len(),
                test.fout.len(),
                "{}. '{}' != '{}'\n{:?}\n{:?}",
                idx,
                test.fout.len(),
                fields.len(),
                fields,
                task
            );
            for (idxf, f) in fields.iter().enumerate() {
                assert_eq!(f, test.fout[idxf], "{}. {} != {}", idx, f, test.fout[idxf]);
            }
        }
    }

    #[test]
    fn width_field_test() {
        struct Test {
            i: Vec<&'static str>,
            fin: Vec<&'static str>,
            fout: Vec<usize>,
        }
        let tests: Vec<Test> = vec![
            Test {
                i: vec!["test due:2020-10-10", "x test due:2020-10-10", "x test"],
                fin: vec!["pri", "done", "created", "finished", "due", "thr"],
                fout: vec![1, 1, 7, 8, 10, 9],
            },
            Test {
                i: vec!["test due:2020-10-10 t:2020-10-10", "x test tag:first", "x test tag:second", "x test some:tag"],
                fin: vec!["done", "due", "thr", "tag", "some", "nothing"],
                fout: vec![1, 10, 10, 6, 4, 7],
            },
        ];
        let base = NaiveDate::from_ymd_opt(2020, 2, 2).unwrap();
        let c: fmt::Conf = Default::default();
        for (idx, test) in tests.iter().enumerate() {
            let mut tasks: todo::TaskVec = Vec::new();
            let mut ids: Vec<usize> = Vec::new();
            for (idx, ts) in test.i.iter().enumerate() {
                let task = todotxt::Task::parse(ts, base);
                tasks.push(task);
                ids.push(idx);
            }
            let widths = col_widths(&tasks, &ids, &test.fin, &c);
            assert_eq!(
                widths.len(),
                test.fout.len(),
                "{}. '{}' != '{}'\n{:?}",
                idx,
                test.fout.len(),
                widths.len(),
                widths
            );
            for (idxf, f) in widths.iter().enumerate() {
                assert_eq!(*f, test.fout[idxf], "{}[{}]. {} != {}", idx, test.fin[idxf], f, test.fout[idxf]);
            }
        }
    }
}
