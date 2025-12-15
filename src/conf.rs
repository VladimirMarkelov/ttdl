use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, IsTerminal, Read, Write, stdout};
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::{env, io};

use anyhow::{Result, anyhow};
use chrono::Local;
use getopts::{Matches, Options};
use termcolor::{Color, ColorSpec};
use unicode_width::UnicodeWidthStr;

use crate::fmt;
use crate::human_date;
use crate::subj_clean::Hide;
use crate::tml;
use todo_lib::{terr, tfilter, todo, todotxt, tsort};

const TODOFILE_VAR: &str = "TTDL_FILENAME";
const APP_DIR: &str = "ttdl";
const CONF_FILE: &str = "ttdl.toml";
const TODO_FILE: &str = "todo.txt";
const DONE_FILE: &str = "done.txt";
const EDITOR: &str = "EDITOR";
const DEFAULT_CONFIG: &str = include_str!("../ttdl.toml");

struct RangeEnds {
    l: usize,
    r: usize,
}
const RANGE_END_SKIP: usize = 1_999_999_999;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RunMode {
    None,
    List,
    Add,
    Done,
    Undone,
    Remove,
    Clean,
    Edit,
    Append,
    Prepend,
    Start,
    Stop,
    Stats,
    Postpone,
    ListProjects,
    ListContexts,
    ListHashtags,
}

#[derive(Debug, Clone)]
pub struct Conf {
    pub mode: RunMode,
    pub verbose: bool,
    pub stdin: bool,
    pub dry: bool,
    pub wipe: bool,
    pub use_done: bool,
    pub first_sunday: bool,
    pub strict_mode: bool,
    pub show_hidden: bool,
    pub todo_file: PathBuf,
    pub done_file: PathBuf,
    pub keep_empty: bool,
    pub keep_tags: bool,
    editor_path: Option<String>,
    pub use_editor: bool,
    pub max_items: Option<usize>,

    pub auto_hide_columns: bool,
    pub auto_show_columns: bool,
    pub always_hide_columns: Vec<String>,
    pub priority_on_done: todotxt::CompletionMode,
    pub add_completion_date_always: bool,

    pub todo: todo::Conf,
    pub fmt: fmt::Conf,
    pub flt: tfilter::Conf,
    pub sort: tsort::Conf,
    pub flt_ext: Option<String>,
    pub postpone_threshold: bool,

    pub calendar: Option<human_date::CalendarRange>,
}

impl Default for Conf {
    fn default() -> Conf {
        Conf {
            mode: RunMode::None,
            stdin: false,
            dry: false,
            verbose: false,
            wipe: false,
            use_done: false,
            first_sunday: true,
            strict_mode: false,
            show_hidden: false,
            todo_file: PathBuf::from(TODO_FILE),
            done_file: PathBuf::from(""),
            keep_empty: false,
            keep_tags: false,
            editor_path: None,
            use_editor: false,
            max_items: None,

            auto_hide_columns: false,
            auto_show_columns: false,
            always_hide_columns: Vec::new(),
            priority_on_done: todotxt::CompletionMode::JustMark,
            add_completion_date_always: false,

            fmt: Default::default(),
            todo: Default::default(),
            flt: Default::default(),
            sort: Default::default(),
            calendar: None,
            flt_ext: None,
            postpone_threshold: false,
        }
    }
}

impl Conf {
    fn new() -> Self {
        Default::default()
    }
    pub fn editor(&self) -> Option<PathBuf> {
        let mut spth: String = env::var(EDITOR).unwrap_or_default();
        if spth.is_empty()
            && let Some(p) = &self.editor_path
        {
            spth = p.clone();
        }
        if spth.is_empty() { None } else { Some(PathBuf::from(spth)) }
    }
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {program} command [ID or ID range] [subject] [filter] [new values] [extra options]");
    print!("{}", opts.usage(&brief));

    println!("\n\n[ID or ID range] - ID is the order number of a todo starting from 1. The range is inclusive. It is OK to use non-existing IDs - all invalid IDs are skipped while processing the command
");

    let filter = r#"Filter options include:
    --all | -a, --complete | -A, --rec, --due, --pri, --regex, --context, --project, --tag | -e, --threshold, --hidden, --hashtag, --filter
    +project - select todos which are related to project "project"; if more than one project name is defined in command line, they are combined with OR;
    @context - select todos which have context "project"; if more than one context is set, they are combined with OR;
    "#;

    let newones = r#"Modifying options are:
    --set-pri, --set-due, --set-rec, --set-proj, --set-ctx, --del-proj, --del-ctx, --repl-proj, --repl-ctx, --set-threshold, --set-tag, --set-hashtag, --del-tag, --del-hashtag, --repl-hashtag, --update-threshold
    "#;

    let extras = r#"Extra options:
    --stdin, --dry-run, --sort | -s, --sort-rev, --wrap, --short, --width, --local, --no-colors, --syntax, --no-syntax, --clean-subject, --auto-hide-cols, --auto-show-cols, --always-hide-cols
    --interactive | -i, --init, --init-local, --group, --no-headers | -H, --hide-fields
    "#;
    let commands = r#"Available commands:
    list | l - list todos
        `ttdl l -s=proj,pri` - show all incomplete todos sorted by their project and by priority inside each project
        `ttdl l "car*"` - list all todos which have substring `car*` in their subject, project or context
        `ttdl l "car*" -e` - list all todos which have subject, project or context matched regular expression `car*`
        `ttdl l "car"` - list all todos which have substring `car` in their subject, project or context
        `ttdl l --max 10` - show only the first 10 incomplete todos
        `ttdl l "car*" --max 5` - list at most 5 todos which have substring `car*` in their subject, project or context
        `ttdl l -s=proj,pri --max 5` - show first 5 incomplete todos sorted by their project and by priority inside each project
        `ttdl l --pri=a` - show all incomplete todos with the highest priority A
        `ttdl l --pri=b+` - show all incomplete todos with priority B and higher (only A and B in this case)
        `ttdl l +car +train` - show all incomplete todos which related either to `car` or to `train` projects
        `ttdl l +my* @*tax` - show all incomplete todos that have a project tag starts with `my` and a context ends with `tax`
        `ttdl l --due=tomorrow -a` - show all todos that are due tomorrow
        `ttdl l --due=soon` - show all incomplete todos which are due are due in less a few days, including overdue ones (the range is configurable and default value is 7 days)
        `ttdl l --due=overdue` - show all incomplete overdue todos
        `ttdl l --due=today -a` - show all todos that are due today
        `ttdl l --due=today..tomorrow -a` - show all todos that are overdue between today and tomorrow(inclusive)
        `ttdl l --due=today:tomorrow -a` - the same as above
        `ttdl l --completed=-1w..today -a` - show all todos that were done the last week(literally: from a week ago through today)
        `ttdl l --created=-3d..` - show active todos that are created 3 days ago or earlier(3 days old and younger)
        `ttdl l --due=..2d..` - show active todos that are either overdue or their due date within 2 days from the current date
        `ttdl l -a +myproj @ui @rest` - show both incomplete and done todos related to project 'myproj' which contains either 'ui' or 'rest' context
        `ttdl l --calendar=m` - show calendar for this month and mark dates that have one or more due todos
        `ttdl l --calendar=2w` - show calendar for this and next week and mark dates that have one or more due todos
        `ttdl l --calendar=+1m` - show calendar for 30 days(one month) starting with today
        `ttdl l --calendar=+-10d` - show calendar for 10 days in the past(one month) ending with today
    add | a - add a new todo
        `ttdl a "send tax declaration +personal @finance @tax due:2018-04-01 rec:1y"` - add a new recurrent todo(yearly todo) with a due date first of April every year
        `ttdl a "(A) send tax return docs due:2018-04-01" - add a new todo with the highest priority `A`
    done | d - mark regular incomplete todos completed, pushes due date for recurrent todos to their next date
        `ttdl d 2-5` - mark todos with IDs from 2 through 5 done
    undone - remove finish date and completion mark from completed todos
    clean | c | archive | arc - move all completed todos to `done.txt`. If option `--wipe` is set then completed todos are removed instead of moving
    remove | rm - delete selected todos. Warning: by default completed todos are not selected, so be careful
        `ttdl rm 2-5` - delete incomplete todos with IDs from 2 thorough 5
        `ttdl rm 2-5 -a` - delete both done and incomplete todos with IDs from 2 through 5
        `ttdl rm 2-5 -A` - delete all done todos with IDs from 2 through 5. The command does the same as `ttdl clean 2-5 --wipe` does
    edit | e - modifies selected todos. Warninig: if you try to change a subject of a few todos, only the first todo would be changed. It is by design. Date-like tags `due` and `threshold` accept simple expressions
        `ttdl e 2-5 "new subject"` - only the first incomplete todo with ID between 2 and 5 changes its subject
        `ttdl e +proj --repl-ctx=bug1010@bug1020` - replace context `bug1010` with `bug1020` for all incomplete todos that related to project `proj`
        `ttdl e @customer_acme --set-due=2018-12-31` - set due date 2018-12-31 for all incomplete todos that has `customer_acme` context
        `ttdl e @customer_acme --set-due=none` - remove due date 2018-12-31 for all incomplete todos that has `customer_acme` context
        `ttdl e --pri=none --set-pri=z` - set the lowest priority for all incomplete todos which do not have a priority set`
        `ttdl e @bug1000 --set-pri=+` - increase priority for all incomplete todos which have context `bug1000`, todos which did not have priority set get the lowest priority `z`
        `ttdl e 2 --set-due=t+1w` - set the due date a week later than the task threshold date
        `ttdl e 2 --set-due=due+2d` - push the due date by 2 days
        `ttdl e 2 --set-due=limit+1d` - takes task's tag `limit` as a date, adds 1 day and sets the result to the due date
        `ttdl e 2 --set-due=t+1w+3d` - push the due date by a week and a half or more accurate by 10 days
        `ttdl e 2-5 -i` - open an external editor of you choice the the incomplete todos with ID between 2 and 5 for interactive editing. After saving the changes and closing the editor, TTDL updates the task list
    append | app - adds a text to the end of todos
    prepend | prep - inserts a text at the beginning of todos
    listprojects [FILTER] | listproj | lp - list all projects
        `ttdl lp ` - show alphabetically sorted list of all projects
        `ttdl lp +un*` - show projects starting with 'un'
    listcontexts [FILTER] | listcon | lc - list all contexts
        `ttdl lc ` - show alphabetically sorted list of all contexts
        `ttdl lc @phon*` - show contexts starting with 'phon'
    start TODO_ID - activates todo's timer
    stop TODO_ID - stops the timer for a todo and updates time spent for it
    stats [--short] - shows todo list summary
        `ttdl stats --short` - displays only the number of total, active, done, overdue, and recurrent todos
        `ttdl stats` - detailed view with additional grouping by project and displaying total time spent on each group
    "#;
    println!("{commands}\n\n{filter}\n\n{newones}\n\n{extras}");
}

fn str_to_mode(s: &str) -> RunMode {
    match s {
        "l" | "list" | "ls" => RunMode::List,
        "a" | "add" | "new" => RunMode::Add,
        "d" | "done" | "complete" | "close" => RunMode::Done,
        "u" | "undone" | "open" => RunMode::Undone,
        "c" | "clean" | "arc" | "archive" => RunMode::Clean,
        "e" | "edit" => RunMode::Edit,
        "rm" | "remove" => RunMode::Remove,
        "app" | "append" => RunMode::Append,
        "prep" | "prepend" => RunMode::Prepend,
        "start" => RunMode::Start,
        "stop" => RunMode::Stop,
        "stats" => RunMode::Stats,
        "postpone" => RunMode::Postpone,
        "lp" | "listproj" | "listprojects" => RunMode::ListProjects,
        "lc" | "listcon" | "listcontexts" => RunMode::ListContexts,
        "lh" | "listhash" | "listhashtags" => RunMode::ListHashtags,
        _ => RunMode::None,
    }
}

fn str_to_hide(s: &str) -> Hide {
    match s.to_lowercase().as_str() {
        "no" | "none" | "nothing" => Hide::Nothing,
        "tags" => Hide::Tags,
        "all" | "yes" => Hide::All,
        _ => {
            eprintln!("Invalid value for 'clean-subject': {s}. Must be one of none, nothing, tags, all");
            exit(1);
        }
    }
}

fn str_to_pri_mode(s: &str) -> Option<todotxt::CompletionMode> {
    match s.to_lowercase().as_str() {
        "keep" => Some(todotxt::CompletionMode::JustMark),
        "move" => Some(todotxt::CompletionMode::MovePriority),
        "erase" => Some(todotxt::CompletionMode::RemovePriority),
        "tag" => Some(todotxt::CompletionMode::PriorityToTag),
        _ => None,
    }
}

fn split_filter(orig: &str) -> (String, tfilter::ValueSpan) {
    if orig.starts_with('+') || orig.ends_with('+') {
        return (orig.trim_matches('+').to_string(), tfilter::ValueSpan::Higher);
    } else if orig.starts_with('-') || orig.ends_with('-') {
        return (orig.trim_matches('-').to_string(), tfilter::ValueSpan::Lower);
    }

    (orig.to_string(), tfilter::ValueSpan::Equal)
}

fn parse_filter_pri(val: &str, c: &mut tfilter::Conf) -> Result<(), terr::TodoError> {
    match val {
        "-" | "none" => {
            c.pri = Some(tfilter::Priority { value: todotxt::NO_PRIORITY, span: tfilter::ValueSpan::None });
        }
        "any" | "+" => {
            c.pri = Some(tfilter::Priority { value: todotxt::NO_PRIORITY, span: tfilter::ValueSpan::Any });
        }
        _ => {
            let (s, modif) = split_filter(val);
            if s.len() != 1 {
                return Err(terr::TodoError::InvalidValue(s, "priority".to_string()));
            }
            let p = s.as_bytes()[0];
            if !p.is_ascii_lowercase() {
                return Err(terr::TodoError::InvalidValue(s, "priority".to_string()));
            }
            c.pri = Some(tfilter::Priority { value: p - b'a', span: modif });
        }
    }
    Ok(())
}

fn parse_filter_rec(val: &str, c: &mut tfilter::Conf) -> Result<(), terr::TodoError> {
    match val {
        "" => {}
        "-" | "none" => c.rec = Some(tfilter::Recurrence { span: tfilter::ValueSpan::None }),
        "+" | "any" => c.rec = Some(tfilter::Recurrence { span: tfilter::ValueSpan::Any }),
        // TODO: add equal?
        _ => {
            return Err(terr::TodoError::InvalidValue(val.to_string(), "recurrence".to_string()));
        }
    }
    Ok(())
}

fn parse_filter_date_range(val: &str, soon_days: u8) -> Result<tfilter::DateRange, terr::TodoError> {
    if human_date::is_range_with_none(val) {
        if (val.starts_with("none..") && val.ends_with("..none"))
            || (val.starts_with("none:") && val.ends_with(":none"))
        {
            return Err(terr::TodoError::InvalidValue(val.to_string(), "date range".to_string()));
        }
        let dt = Local::now().date_naive();
        return human_date::human_to_range_with_none(dt, val, soon_days);
    }
    if human_date::is_range(val) {
        let dt = Local::now().date_naive();
        return human_date::human_to_range(dt, val, soon_days);
    }

    match val {
        "-" | "none" => Ok(tfilter::DateRange { days: Default::default(), span: tfilter::ValueSpan::None }),
        "any" | "+" => Ok(tfilter::DateRange { days: Default::default(), span: tfilter::ValueSpan::Any }),
        "over" | "overdue" => {
            Ok(tfilter::DateRange { days: tfilter::ValueRange { low: 0, high: 0 }, span: tfilter::ValueSpan::Lower })
        }
        "soon" => Ok(tfilter::DateRange {
            days: tfilter::ValueRange { low: 0, high: soon_days as i64 },
            span: tfilter::ValueSpan::Range,
        }),
        "today" => {
            Ok(tfilter::DateRange { days: tfilter::ValueRange { low: 0, high: 0 }, span: tfilter::ValueSpan::Range })
        }
        "tomorrow" => {
            Ok(tfilter::DateRange { days: tfilter::ValueRange { low: 1, high: 1 }, span: tfilter::ValueSpan::Range })
        }
        "yesterday" => {
            Ok(tfilter::DateRange { days: tfilter::ValueRange { low: -1, high: -1 }, span: tfilter::ValueSpan::Range })
        }
        _ => Err(terr::TodoError::InvalidValue(val.to_string(), "date range".to_string())),
    }
}

fn parse_filter_due(val: &str, c: &mut tfilter::Conf, soon_days: u8) -> Result<(), terr::TodoError> {
    let rng = parse_filter_date_range(val, soon_days)?;
    c.due = Some(rng);
    Ok(())
}

fn parse_filter_created(val: &str, c: &mut tfilter::Conf, soon_days: u8) -> Result<(), terr::TodoError> {
    let rng = parse_filter_date_range(val, soon_days)?;
    c.created = Some(rng);
    Ok(())
}

fn parse_filter_completed(val: &str, c: &mut tfilter::Conf, soon_days: u8) -> Result<(), terr::TodoError> {
    let rng = parse_filter_date_range(val, soon_days)?;
    c.finished = Some(rng);
    Ok(())
}

fn parse_filter_threshold(val: &str, c: &mut tfilter::Conf, soon_days: u8) -> Result<(), terr::TodoError> {
    let rng = parse_filter_date_range(val, soon_days)?;
    c.thr = Some(rng);
    Ok(())
}

fn parse_filter(matches: &Matches, c: &mut tfilter::Conf, soon_days: u8) -> Result<(), terr::TodoError> {
    if matches.opt_present("a") {
        c.all = tfilter::TodoStatus::All;
    }
    if matches.opt_present("A") {
        c.all = tfilter::TodoStatus::Done;
    }
    if matches.opt_present("e") {
        c.use_regex = true;
    }
    if matches.opt_present("t") {
        c.tmr = Some(tfilter::Timer { span: tfilter::ValueSpan::Active, value: 0 });
    }
    if matches.opt_present("pri") {
        let s = match matches.opt_str("pri") {
            None => String::new(),
            Some(s_orig) => s_orig.to_lowercase(),
        };
        parse_filter_pri(&s, c)?;
    }
    if matches.opt_present("rec") {
        let rstr = match matches.opt_str("rec") {
            None => String::new(),
            Some(s) => s.to_lowercase(),
        };
        parse_filter_rec(&rstr, c)?;
    }
    if matches.opt_present("due") {
        let dstr = match matches.opt_str("due") {
            None => String::new(),
            Some(s) => s.to_lowercase(),
        };
        parse_filter_due(&dstr, c, soon_days)?;
    }
    if matches.opt_present("created") {
        let dstr = match matches.opt_str("created") {
            None => String::new(),
            Some(s) => s.to_lowercase(),
        };
        parse_filter_created(&dstr, c, soon_days)?;
    }
    if matches.opt_present("completed") {
        let dstr = match matches.opt_str("completed") {
            None => String::new(),
            Some(s) => s.to_lowercase(),
        };
        parse_filter_completed(&dstr, c, soon_days)?;
    }
    if matches.opt_present("threshold") {
        let dstr = match matches.opt_str("threshold") {
            None => String::new(),
            Some(s) => s.to_lowercase(),
        };
        parse_filter_threshold(&dstr, c, soon_days)?;
    }

    if let Some(dstr) = matches.opt_str("context") {
        let (i, e) = comma_list_to_vec(&dstr);
        c.include.contexts = i;
        c.exclude.contexts = e;
    };
    if let Some(dstr) = matches.opt_str("project") {
        let (i, e) = comma_list_to_vec(&dstr);
        c.include.projects = i;
        c.exclude.projects = e;
    };
    if let Some(dstr) = matches.opt_str("tag") {
        let (i, e) = comma_list_to_vec(&dstr);
        c.include.tags = i;
        c.exclude.tags = e;
    };
    if let Some(dstr) = matches.opt_str("hashtag") {
        let (i, e) = comma_list_to_vec(&dstr);
        c.include.hashtags = i;
        c.exclude.hashtags = e;
    };

    Ok(())
}

fn comma_list_to_vec(list: &str) -> (Vec<String>, Vec<String>) {
    let mut incl = Vec::new();
    let mut excl = Vec::new();
    let lstr = list.trim().to_lowercase();
    if lstr.is_empty() {
        incl.push("none".to_string());
    } else {
        for st in lstr.split(',') {
            if st.starts_with('-') {
                excl.push(st.trim_start_matches('-').to_string());
            } else {
                incl.push(st.to_string());
            }
        }
    }
    (incl, excl)
}

fn parse_todo(matches: &Matches, c: &mut todo::Conf) -> Result<(), terr::TodoError> {
    if let Some(s) = matches.opt_str("set-pri") {
        let s = if s.is_empty() { "none".to_owned() } else { s.to_lowercase() };
        match s.as_str() {
            "-" => {
                c.priority.action = todo::Action::Decrease;
            }
            "+" => {
                c.priority.action = todo::Action::Increase;
            }
            "none" => {
                c.priority.action = todo::Action::Delete;
            }
            _ => {
                let p = s.as_bytes()[0];
                if !p.is_ascii_lowercase() {
                    return Err(terr::TodoError::InvalidValue(s, "priority".to_string()));
                }
                c.priority.value = p - b'a';
                c.priority.action = todo::Action::Set;
            }
        }
    }

    if let Some(s) = matches.opt_str("set-rec") {
        let s = s.to_lowercase();
        match s.as_str() {
            "-" | "none" => {
                c.recurrence.action = todo::Action::Delete;
            }
            _ => match todotxt::Recurrence::from_str(&s) {
                Ok(r) => {
                    c.recurrence.value = Some(r);
                    c.recurrence.action = todo::Action::Set;
                }
                Err(_) => {
                    return Err(terr::TodoError::InvalidValue(s, "recurrence".to_string()));
                }
            },
        }
    }

    if let Some(s) = matches.opt_str("set-due") {
        match s.as_str() {
            "-" | "none" => {
                c.due.action = todo::Action::Delete;
            }
            "soon" => {
                return Err(terr::TodoError::InvalidValue(s, "set-due".to_string()));
            }
            _ => {
                let dt = Local::now().date_naive();
                if let Ok(new_date) = human_date::human_to_date(dt, &s, 0) {
                    c.due.value = todo::NewDateValue::Date(new_date);
                    c.due.action = todo::Action::Set;
                } else {
                    match chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                        Ok(d) => {
                            c.due.value = todo::NewDateValue::Date(d);
                            c.due.action = todo::Action::Set;
                        }
                        Err(_) => {
                            c.due.action = todo::Action::Set;
                            c.due.value = todo::NewDateValue::Expr(s);
                        }
                    }
                }
            }
        }
    }

    if let Some(s) = matches.opt_str("set-threshold") {
        match s.as_str() {
            "-" | "none" => {
                c.thr.action = todo::Action::Delete;
            }
            "soon" => {
                return Err(terr::TodoError::InvalidValue(s, "set-threshold".to_string()));
            }
            _ => {
                let dt = Local::now().date_naive();
                if let Ok(new_date) = human_date::human_to_date(dt, &s, 0) {
                    c.thr.value = todo::NewDateValue::Date(new_date);
                    c.thr.action = todo::Action::Set;
                } else {
                    match chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                        Ok(d) => {
                            c.thr.value = todo::NewDateValue::Date(d);
                            c.thr.action = todo::Action::Set;
                        }
                        Err(_) => {
                            c.thr.action = todo::Action::Set;
                            c.thr.value = todo::NewDateValue::Expr(s);
                        }
                    }
                }
            }
        }
    }

    if let Some(s) = matches.opt_str("set-proj") {
        for st in s.split(',') {
            c.projects.value.push(st.to_string());
        }
        c.projects.action = todo::Action::Set;
    }

    if let Some(s) = matches.opt_str("del-proj") {
        for st in s.split(',') {
            c.projects.value.push(st.to_string());
        }
        c.projects.action = todo::Action::Delete;
    }

    if let Some(s) = matches.opt_str("repl-proj") {
        for st in s.split(',') {
            c.projects.value.push(st.to_string());
        }
        c.projects.action = todo::Action::Replace;
    }

    if let Some(s) = matches.opt_str("set-ctx") {
        for st in s.split(',') {
            c.contexts.value.push(st.to_string());
        }
        c.contexts.action = todo::Action::Set;
    }

    if let Some(s) = matches.opt_str("del-ctx") {
        for st in s.split(',') {
            c.contexts.value.push(st.to_string());
        }
        c.contexts.action = todo::Action::Delete;
    }

    if let Some(s) = matches.opt_str("repl-ctx") {
        for st in s.split(',') {
            c.contexts.value.push(st.to_string());
        }
        c.contexts.action = todo::Action::Replace;
    }

    if let Some(s) = matches.opt_str("set-tag") {
        let mut hmap: HashMap<String, String> = HashMap::new();
        for st in s.split(',') {
            if let Some((tag, val)) = todotxt::split_tag(st) {
                if todo::is_tag_special(tag) {
                    return Err(terr::TodoError::InvalidValue(
                        tag.to_string(),
                        "use designated option to change built-in tag".to_string(),
                    ));
                }
                hmap.insert(tag.to_string(), val.to_string());
            } else {
                return Err(terr::TodoError::InvalidValue(st.to_string(), "tag-value pair".to_string()));
            }
        }
        if !hmap.is_empty() {
            c.tags = todo::TagValuesChange { value: Some(hmap), action: todo::Action::Set };
        }
    }

    if let Some(s) = matches.opt_str("del-tag") {
        let mut hmap: HashMap<String, String> = HashMap::new();
        for st in s.split(',') {
            if let Some((tag, _)) = todotxt::split_tag(st) {
                if todo::is_tag_special(tag) {
                    return Err(terr::TodoError::InvalidValue(
                        tag.to_string(),
                        "use designated option to change built-in tag".to_string(),
                    ));
                }
                hmap.insert(tag.to_string(), String::new());
            } else {
                if todo::is_tag_special(st) {
                    return Err(terr::TodoError::InvalidValue(
                        st.to_string(),
                        "use designated option to change built-in tag".to_string(),
                    ));
                }
                hmap.insert(st.to_string(), String::new());
            }
        }
        if !hmap.is_empty() {
            c.tags = todo::TagValuesChange { value: Some(hmap), action: todo::Action::Delete };
        }
    }

    if let Some(s) = matches.opt_str("set-hashtag") {
        let mut v: Vec<String> = Vec::new();
        for st in s.split(',') {
            v.push(st.to_string());
        }
        if !v.is_empty() {
            c.hashtags = todo::ListTagChange { value: v, action: todo::Action::Set };
        }
    }

    if let Some(s) = matches.opt_str("del-hashtag") {
        let mut v: Vec<String> = Vec::new();
        for st in s.split(',') {
            v.push(st.to_string());
        }
        if !v.is_empty() {
            c.hashtags = todo::ListTagChange { value: v, action: todo::Action::Delete }
        }
    }

    if let Some(s) = matches.opt_str("repl-hashtag") {
        let mut v: Vec<String> = Vec::new();
        for st in s.split(',') {
            if st.contains(':') {
                v.push(st.to_string());
            } else {
                return Err(terr::TodoError::InvalidValue(st.to_string(), "old-hashtag:new-hashtag pair".to_string()));
            }
        }
        if !v.is_empty() {
            c.hashtags = todo::ListTagChange { value: v, action: todo::Action::Replace };
        }
    }

    Ok(())
}

fn parse_sort(matches: &Matches, c: &mut tsort::Conf) {
    if matches.opt_present("s") {
        // TODO: check for valid
        match matches.opt_str("s") {
            None => c.fields = Some("priority".to_owned()),
            Some(s) => c.fields = Some(s),
        }
    }
    if matches.opt_present("sort-rev") {
        c.rev = true;
    }
}

fn parse_fmt(matches: &Matches, c: &mut fmt::Conf) {
    if matches.opt_present("short") {
        c.fmt = fmt::Format::Short;
    }
    if matches.opt_present("wrap") {
        c.long = fmt::LongLine::WordWrap;
    }
    if matches.opt_present("human") {
        c.human = true;
    }
    if let Some(s) = matches.opt_str("human")
        && !s.is_empty()
    {
        for f in s.split(',') {
            c.human_fields.push(f.to_lowercase());
        }
    }
    c.atty = stdout().is_terminal();

    if matches.opt_present("no-colors") || !c.atty {
        c.color_term = fmt::TermColorType::None;
    }

    if matches.opt_present("compact") {
        c.compact = true;
        c.long = fmt::LongLine::Cut;
    }
    if let Some(s) = matches.opt_str("width")
        && let Ok(n) = s.parse::<u16>()
    {
        c.width = n;
    }

    if !c.atty && c.width == 0 {
        c.long = fmt::LongLine::Simple;
    } else if c.atty {
        // TODO: sane limit?
        if (c.width > 2000 || c.width < 20)
            && let Some((w, _)) = term_size::dimensions()
        {
            c.width = (w % 65536) as u16 - 1;
        }
    }

    if let Some(s) = matches.opt_str("fields") {
        if s.find(',').is_some() {
            c.fields = s.split(',').map(|s| s.to_string()).collect();
        } else {
            c.fields = s.split(':').map(|s| s.to_string()).collect();
        }
    }

    if let Some(s) = matches.opt_str("group") {
        c.group = Some(s.clone());
    }
    c.hide_headers = matches.opt_present("no-headers");
}

fn detect_filenames(matches: &Matches, conf: &mut Conf) {
    if let Ok(val) = env::var(TODOFILE_VAR)
        && !val.is_empty()
    {
        conf.todo_file = PathBuf::from(val);
    }

    if matches.opt_present("local") {
        conf.todo_file = PathBuf::from(TODO_FILE);
    } else if let Some(val) = matches.opt_str("todo-file")
        && !val.is_empty()
    {
        conf.todo_file = PathBuf::from(val);
    }

    resolve_home_directory(&mut conf.todo_file);

    if conf.todo_file.is_dir() {
        conf.todo_file.push(TODO_FILE);
    }

    if let Some(val) = matches.opt_str("done-file")
        && !val.is_empty()
    {
        let mut pb = PathBuf::from(val.clone());
        if pb.parent() == Some(&PathBuf::from("")) {
            conf.done_file = conf.todo_file.with_file_name(val);
        } else {
            resolve_home_directory(&mut pb);
            conf.done_file = pb;
        }
    }
    if conf.done_file == PathBuf::from("") {
        conf.done_file = conf.todo_file.with_file_name(DONE_FILE);
    }
    if conf.done_file.is_dir() {
        conf.done_file.push(DONE_FILE);
    }
}

fn resolve_home_directory(path: &mut PathBuf) {
    if let Ok(path_striped) = path.strip_prefix("~")
        && let Some(home) = dirs::home_dir()
    {
        *path = home.join(path_striped);
    }
}

fn read_color(clr: &Option<String>) -> Result<ColorSpec> {
    let s = match clr {
        Some(ss) => ss,
        None => return Ok(fmt::default_color()),
    };

    color_from_str(s)
}

fn update_colors_from_conf(tc: &tml::Conf, conf: &mut Conf) -> Result<()> {
    conf.fmt.colors.overdue = read_color(&tc.colors.overdue)?;
    conf.fmt.colors.top = read_color(&tc.colors.top)?;
    conf.fmt.colors.important = read_color(&tc.colors.important)?;
    conf.fmt.colors.today = read_color(&tc.colors.today)?;
    conf.fmt.colors.soon = read_color(&tc.colors.soon)?;
    conf.fmt.colors.done = read_color(&tc.colors.done)?;
    conf.fmt.colors.threshold = read_color(&tc.colors.threshold)?;
    conf.fmt.colors.old = read_color(&tc.colors.old)?;
    conf.fmt.colors.default_fg = read_color(&tc.colors.default_fg)?;

    if conf.fmt.color_term == fmt::TermColorType::Auto
        && let Some(cs) = &tc.colors.color_term
    {
        conf.fmt.color_term = match cs.to_lowercase().as_str() {
            "ansi" => fmt::TermColorType::Ansi,
            "none" => fmt::TermColorType::None,
            _ => fmt::TermColorType::Auto,
        }
    }
    Ok(())
}

fn update_ranges_from_conf(tc: &tml::Conf, conf: &mut Conf) {
    if let Some(imp) = &tc.ranges.important
        && imp.len() == 1
    {
        let lowst = imp.to_lowercase();
        let p = lowst.as_bytes()[0];
        if p >= b'a' || p <= b'z' {
            conf.fmt.colors.important_limit = p - b'a';
        }
    }

    if let Some(soon) = tc.ranges.soon {
        if soon == 0 {
            conf.fmt.colors.soon_days = 7u8;
        } else if soon > 0 && soon < 256 {
            conf.fmt.colors.soon_days = soon as u8;
        }
    }

    if let Some(ref old) = tc.ranges.old
        && let Ok(r) = todotxt::Recurrence::from_str(old)
    {
        conf.fmt.colors.old_period = Some(r);
    }
}

fn update_syntax_from_config(tc: &tml::Conf, conf: &mut Conf) -> Result<()> {
    if let Some(ref cfg) = tc.syntax {
        if let Some(b) = cfg.enabled {
            conf.fmt.syntax = b;
        }
        if let Some(ref clr) = cfg.tag_color {
            conf.fmt.colors.tag = color_from_str(clr)?;
        }
        if let Some(ref clr) = cfg.hashtag_color {
            conf.fmt.colors.hashtag = color_from_str(clr)?;
        }
        if let Some(ref clr) = cfg.project_color {
            conf.fmt.colors.project = color_from_str(clr)?;
        }
        if let Some(ref clr) = cfg.context_color {
            conf.fmt.colors.context = color_from_str(clr)?;
        }
    }
    Ok(())
}

fn update_fields_from_config(tc: &tml::Conf, conf: &mut Conf) -> Result<()> {
    match tc.fields {
        None => {}
        Some(ref fields) => {
            for field in fields.iter() {
                let mut cf = fmt::CustomField {
                    rules: Vec::new(),
                    name: field.name.clone(),
                    title: field.title.clone(),
                    kind: field.kind.clone(),
                    width: field.width,
                };
                if let Some(ref rs) = field.rules {
                    for rule in rs.iter() {
                        let rcolor = color_from_str(&rule.color)?;
                        if let Some((b, e)) = parse_range(&rule.range) {
                            let r =
                                fmt::FmtRule { color: rcolor, range: fmt::FmtSpec::Range(b.to_owned(), e.to_owned()) };
                            cf.rules.push(r);
                        } else {
                            let v: Vec<String> = rule.range.split(',').map(|s| s.to_string()).collect();
                            let r = fmt::FmtRule { color: rcolor, range: fmt::FmtSpec::List(v) };
                            cf.rules.push(r);
                        }
                    }
                }
                conf.fmt.custom_fields.push(cf);
            }
        }
    }
    validate_custom_fields(conf)?;
    for fld in conf.fmt.custom_fields.iter() {
        conf.fmt.custom_names.push(fld.name.to_string());
    }
    Ok(())
}

fn validate_custom_fields(conf: &Conf) -> Result<()> {
    for field in conf.fmt.custom_fields.iter() {
        if field.name.is_empty() {
            return Err(anyhow!("Field name cannot be empty"));
        }
        if field.title.is_empty() {
            return Err(anyhow!("Field '{}' title cannot be empty", field.name));
        }
        if field.width != 0 && field.title.width() > usize::from(field.width) {
            return Err(anyhow!("Field '{}' width is smaller than its title length", field.name));
        }
        if field.width > 20 {
            return Err(anyhow!("Field '{}' length is too long '{}', maximum is 20", field.name, field.width));
        }
        match field.kind.as_str() {
            "" => return Err(anyhow!("Field '{}' type is empty", field.name)),
            "string" | "integer" | "int" | "float" | "date" | "duration" | "bytes" => {}
            _ => return Err(anyhow!("Field '{}' has unknown type '{}'", field.name, field.kind)),
        }
    }
    Ok(())
}

fn update_global_from_conf(tc: &tml::Conf, conf: &mut Conf) {
    if let Some(fname) = &tc.global.filename
        && !fname.is_empty()
    {
        conf.todo_file = PathBuf::from(fname);
    }
    if let Some(auto_date) = tc.global.creation_date_auto {
        conf.todo.auto_create_date = auto_date;
    }
    if let Some(fs) = &tc.global.fields {
        if fs.find(',').is_some() {
            conf.fmt.fields = fs.split(',').map(|s| s.to_string()).collect();
        } else if fs.find(':').is_some() {
            conf.fmt.fields = fs.split(':').map(|s| s.to_string()).collect();
        }
    }

    if let Some(sort_fields) = &tc.global.sort
        && conf.sort.fields.is_none()
    {
        conf.sort.fields = Some(sort_fields.to_owned());
    }

    if let Some(sh) = &tc.global.shell
        && !sh.is_empty()
    {
        conf.fmt.shell.clone_from(sh);
    }

    if let Some(s) = &tc.global.script_ext {
        conf.fmt.script_ext = s.to_string();
    }
    if let Some(s) = &tc.global.script_prefix {
        conf.fmt.script_prefix = s.to_string();
    }
    if let Some(b) = &tc.global.first_sunday {
        conf.first_sunday = *b;
    }
    if let Some(s) = &tc.global.clean_subject {
        conf.fmt.hide = str_to_hide(s);
    }
    if let Some(b) = &tc.global.auto_hide_columns {
        conf.auto_hide_columns = *b;
    }
    if let Some(b) = &tc.global.auto_show_columns {
        conf.auto_show_columns = *b;
    }
    if let Some(l) = &tc.global.always_hide_columns {
        let mut v = Vec::new();
        for item in l.split(',') {
            v.push(item.to_string());
        }
        conf.always_hide_columns = v;
    }
    if let Some(l) = &tc.global.priority_on_done {
        match str_to_pri_mode(l) {
            Some(m) => conf.priority_on_done = m,
            None => eprintln!("Invalid value '{l}' for global.priority_on_done"),
        }
    }
    if let Some(acda) = &tc.global.add_completion_date_always {
        conf.add_completion_date_always = *acda;
    }
    if let Some(p) = &tc.global.editor {
        conf.editor_path = Some(p.clone());
    }
    if let Some(flist) = &tc.global.hide_fields {
        let lst: Vec<String> = flist.split(',').map(|itm| itm.to_string()).collect();
        conf.fmt.hide_fields = lst;
    }
}

fn detect_conf_file_path() -> PathBuf {
    let loc_path = PathBuf::from(CONF_FILE);
    if loc_path.exists() {
        return loc_path;
    }

    match dirs::config_dir() {
        Some(mut d) => {
            d.push(APP_DIR);
            d.push(CONF_FILE);
            d
        }
        None => loc_path,
    }
}

fn init_config_file(local: bool) -> Result<()> {
    let conf_dir = if local {
        PathBuf::from("")
    } else {
        let mut cfg = match dirs::config_dir() {
            None => return Err(anyhow!("Failed to get user's directory for configuration files")),
            Some(dir) => dir,
        };
        cfg.push(APP_DIR);
        cfg
    };
    let mut conf_path = conf_dir.clone();
    conf_path.push(CONF_FILE);
    if conf_path.exists() {
        println!("Configuration file '{0}' already exists.", conf_path.display());
    } else {
        if !local && !conf_dir.exists() {
            fs::create_dir_all(conf_dir.clone())?;
            println!("Created directory '{0}'", conf_dir.display());
        }
        let mut file = File::create(conf_path.clone())?;
        file.write_all(DEFAULT_CONFIG.as_bytes())?;
        println!("Configuration initialized: '{0}'", conf_path.display());
    }
    Ok(())
}

fn load_from_config(conf: &mut Conf, conf_path: Option<PathBuf>) -> Result<()> {
    let path: PathBuf = match conf_path {
        Some(p) => p,
        None => detect_conf_file_path(),
    };

    if conf.verbose {
        println!("Loading configuration from: {path:?}");
    }
    if !path.exists() {
        return Ok(());
    }

    let inp = File::open(path)?;
    let mut buffered = BufReader::new(inp);
    let mut data = String::new();
    buffered.read_to_string(&mut data)?;

    let info_toml: tml::Conf = toml::from_str(&data)?;

    update_colors_from_conf(&info_toml, conf)?;
    update_ranges_from_conf(&info_toml, conf);
    update_global_from_conf(&info_toml, conf);
    update_syntax_from_config(&info_toml, conf)?;
    update_fields_from_config(&info_toml, conf)?;
    if let Some(strict) = info_toml.global.strict_mode {
        conf.strict_mode = strict;
    }
    Ok(())
}

fn preprocess_args(args: &[String]) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    for arg in args {
        if arg.starts_with("-@") || arg.starts_with("-+") {
            res.push(" ".to_string() + arg);
        } else {
            res.push(arg.to_string());
        }
    }
    res
}

pub fn parse_args(args: &[String]) -> Result<Conf> {
    let args = preprocess_args(args);
    let program = args[0].clone();
    let mut conf = Conf::new();

    // Free short options: BCDEFGIJKLMNOPQRSTUVWXYZbdfgjlmnopqruxyz"

    let mut opts = Options::new();
    opts.optflag("h", "help", "Show this help");
    opts.optflag("a", "all", "Select all todos including completed ones");
    opts.optflag("A", "only-completed", "Select only completed todos");
    opts.optflag("t", "active", "Only active records");
    opts.optflag("", "dry-run", "Dry run: do not change todo list, only show which todos would be changed");
    opts.optflag("v", "verbose", "Display extra information (used file names etc)");
    opts.optflag(
        "e",
        "regex",
        "Treat the search string as regular expression. By default simple case-insensitive substring search is done",
    );
    opts.optflag("", "wipe", "'Clean' command deletes todos instead of moving them to 'done.txt'");
    opts.optflagopt(
        "s",
        "sort",
        "Sort todos by the list of fields(if the list is empty todos are sorted by their priority)",
        "FIELD1,FIELD2",
    );
    opts.optflag("", "sort-rev", "Reverse todo list after sorting. It works only if the option 'sort' is set");
    opts.optopt("", "group", "a field name that is used to group the list of tasks", "FIELD");
    opts.optopt("", "rec", "Select only recurrent(any) or non-recurrent(none) todos", "any | none");
    opts.optopt("", "due", "Select records without due date(none), with any due date(any), overdue todos(overdue), today's todos(today), tomorrow's ones(tomorrow), or which are due in a few days(soon)", "any | none | today| tomorrow | yesterday | soon | 'range'");
    opts.optopt(
        "",
        "created",
        "Select records without creation date(none), with any creation date(any), created within a date range",
        "any | none | today| tomorrow | yesterday | soon | 'range'",
    );
    opts.optopt(
        "",
        "completed",
        "Select records without completion date(none), with any completion date(any), completed within a date range",
        "any | none | today| tomorrow | yesterday | soon | 'range'",
    );
    opts.optopt(
        "",
        "threshold",
        "Select records without threshold date(none), with any threshold date(any)",
        "any | none | today| tomorrow | yesterday | soon | 'range'",
    );
    opts.optopt(
        "",
        "project",
        "Comma-separated list of projects. Select records that have any of them. Special values: 'none' - select records with no project, and 'any' - select records that have at least one project. Basic pattern matching supported: '*ab' - project ends with 'ab', 'ab*' - project starts with 'ab', '*ab*' - project contains 'ab'",
        "PROJECT1,PROJECT2",
    );
    opts.optopt(
        "",
        "context",
        "Comma-separated list of contexts. Select records that have any of them. Special values: 'none' - select records with no context, and 'any' - select records that have at least one context. Basic pattern matching supported: '*ab' - context ends with 'ab', 'ab*' - context starts with 'ab', '*ab*' - context contains 'ab'",
        "CONTEXT1,CONTEXT2",
    );
    opts.optopt(
        "",
        "tag",
        "Comma-separated list of tags. Select records that have any of them. Special values: 'none' - select records with no tag, and 'any' - select records that have at least one tag. Basic pattern matching supported: '*ab' - tag ends with 'ab', 'ab*' - tag starts with 'ab', '*ab*' - tag contains 'ab'",
        "TAG1,TAG2",
    );
    opts.optopt("", "pri", "Select todos without priority(none), with any priority(any), with a given priority, with a priority equal to or higher/lower than the given priority", "none | any | a | b+ | c-");
    opts.optopt("", "hashtag", "Select only todos with any of hashtags", "HASHTAG1,HASHTAG2 | any | none");
    opts.optopt(
        "",
        "set-pri",
        "Change priority for selected todos: remove priority, set exact priority, increase or decrease it",
        "none | A-Z | + | -",
    );
    opts.optopt(
        "",
        "set-rec",
        "Change recurrence for selected todos: remove recurrence, or set a new one",
        "none | 1m | 15d",
    );
    opts.optopt(
        "",
        "set-due",
        "Change due date for selected todos: remove due date or set a new one",
        "none | YYYY-MM-DD",
    );
    opts.optopt(
        "",
        "set-threshold",
        "Change threshold date for selected todos: remove threshold date or set a new one",
        "none | YYYY-MM-DD",
    );
    opts.optopt("", "set-proj", "Add projects to selected todos", "PROJ1,PROJ2");
    opts.optopt("", "set-ctx", "Add contexts to selected todos", "CTX1,CTX2");
    opts.optopt("", "set-tag", "Add tags to selected todos", "TAG1:VALUE1,TAG2:VALUE2");
    opts.optopt("", "set-hashtag", "Add hashtags to selected todos", "HASHTAG1,HASHTAG2");
    opts.optopt("", "del-proj", "Remove projects from selected todos", "PROJ");
    opts.optopt("", "del-ctx", "Remove contexts from selected todos", "CTX");
    opts.optopt("", "del-tag", "Remove tags from selected todos", "TAG1,TAG2");
    opts.optopt("", "del-hashtag", "Remove hashtags from selected todos", "HASHTAG1,HASHTAG2");
    opts.optopt(
        "",
        "repl-proj",
        "Replace projects for selected todos: a list of comma separated pairs of old and new project values. Old value and new value are separated with '+'",
        "PROJ1+PROJECT1,PROJ2+PROJECT2",
    );
    opts.optopt(
        "",
        "repl-ctx",
        "Replace contexts for selected todos: a list of comma separated pairs of old and new context values. Old value and new value are separated with '@'",
        "CT1@CTX,CT2@ANOTHER",
    );
    opts.optopt("", "repl-hashtag", "Replace hashtags for selected todos", "HASHTAG1:NEW1,HASHTAG2:NEW2");
    opts.optflag("", "short", "Show only ID, priority and subject for todos");
    opts.optflag("", "wrap", "Word wrap a long subject within subject column");
    opts.optopt("w", "width", "Set terminal width. The application detects terminal width automatically but it is possible to limit the output width manually", "WIDTH");
    opts.optflagopt(
        "",
        "human",
        "Show relative date(for due and threshold dates) instead of default YYYY-MM-DD. Examples: 'today', '4d overdue', or 'in 2m'",
        "empty value or FIELD1,FIELD2",
    );
    opts.optflag("", "compact", "Show relative date in compact mode: without 'in' or 'overdue', overdue and future dates are distinguished by their colors");
    opts.optopt(
        "",
        "fields",
        "Set custom list of fields to display(ID is always visible). The list defines visibility of fields but not their order. The order cannot be changed",
        "FIELD2,FIELD1",
    );
    opts.optflag("", "local", "Use todo from the current working directory. It is the default mode. But if you set environment variable or modified config, the option can be used to override those values and use todo.txt from current working directory");
    opts.optflag("", "no-colors", "Disable all colors");
    opts.optflag(
        "",
        "done",
        "Use file of completed todos - done.txt. In this mode the only available command is 'list'",
    );
    opts.optflag("", "version", "Show TTDL version");
    opts.optopt("c", "config", "Path to configuration file", "CONF FILE PATH");
    opts.optopt(
        "",
        "todo-file",
        "Path to file with todos (if it is directory 'todo.txt' is added automatically) ",
        "TODO FILE PATH",
    );
    opts.optopt("", "done-file", "Path to file with archived todos (if it is directory 'done.txt' is added automatically, if it contains only file name then the directory is the same as for todo.txt) ", "DONE FILE PATH");
    opts.optflag("", "strict", "Enable strict mode");
    opts.optflag("", "hidden", "Include hidden tasks");
    opts.optopt(
        "",
        "calendar",
        "Display a calendar with dates highlighted if any todo is due on that date(foreground color). Today is highlighted with background color, Default values for `NUMBER` is `1` and for `TYPE` is `d`(days). Valid values for type are `d`(days), `w`(weeks), and `m`(months). Prepending plus sign shows the selected interval starting from today, not from Monday or first day of the month",
        "[+][NUMBER][TYPE]",
    );
    opts.optflag("", "syntax", "Enable keyword highlights when printing subject");
    opts.optflag("", "no-syntax", "Disable keyword highlights when printing subject");
    opts.optflag("", "keep-empty", "do not remove empty todos when cleaning up(archiving) the list");
    opts.optopt(
        "",
        "clean-subject",
        "hide the given items in a subject column when printing a task list. Items are hidden only if their corresponding columns are visible",
        "no|none|nothing|tags|all|yes. 'yes' is an alias for all, 'no|none|nothing' are the synonyms",
    );
    opts.optflag("", "auto-hide-cols", "Hide columns that do not have values");
    opts.optflag("", "auto-show-cols", "Show all columns that have at least one value");
    opts.optopt(
        "",
        "always-hide-cols",
        "Comma-separated list of tags that TTDL never show in a separate column. E.g, 'prj,due' or 'pri,created,customtag'",
        "FIELD1,FIELD2",
    );
    opts.optopt(
        "",
        "priority-on-done",
        "what to do with priority on task completion: keep - no special action(default behavior), move - place priority after completion date, tag - convert priority to a tag 'pri:', erase - remove priority. Note that in all modes, except `erase`, the operation is reversible and on task uncompleting, the task gets its priority back",
        "VALUE",
    );
    opts.optflag(
        "",
        "add-completion-date-always",
        "When task is finished, always add completion date, regardless of whether or not creation date is present",
    );
    opts.optflag("k", "keep-tags", "in edit mode a new subject replaces regular text of the todo, everything else(tags, priority etc) is taken from the old and appended to the new subject. A convenient way to replace just text and keep all the tags without typing the tags again");
    opts.optflag("i", "interactive", "Open an external edit to modify all filtered tasks. If the task list is modified inside an editor, the old tasks will be removed and new ones will be added to the end of the task list. If you do not change anything or save an empty file, the edit operation will be canceled. To set editor, change config.global.editor option or set EDITOR environment variable.");
    opts.optflag("", "init", "create a default configuration file in user's configuration directory if the configuration file does not exist yet");
    opts.optflag("", "init-local", "create a default configuration file in the current working directory if the configuration file does not exist yet");
    opts.optflag("", "stdin", "Read new or replacement task content from standard input");
    opts.optflag("H", "no-headers", "Do not show headers and footers");

    opts.optopt("", "max", "Set maximum number of todos to display", "NUMBER");
    opts.optopt(
        "",
        "filter-tag",
        "Custom filter by user-defined tag values",
        "TAG1=RANGE1;TAG2=RANGE2. Deprecated, use 'filter' instead",
    );
    opts.optopt("", "filter", "Custom filter by user-defined tag values", "TAG1=RANGE1;TAG2=RANGE2");
    opts.optflag("", "update-threshold", "Update threshold in addition to changing due date when a task is postponed");
    opts.optopt(
        "",
        "hide-fields",
        "Comma-separated list of fields to hide in both columns and subject",
        "FIELD1,FIELD2",
    );

    let matches: Matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            println!("{e}");
            print_usage(&program, &opts);
            exit(1);
        }
    };

    if matches.opt_present("version") {
        let version = env!("CARGO_PKG_VERSION");
        println!("Terminal Todo List Manager(TTDL) Version {version}");
        exit(0);
    }
    if matches.opt_present("h") {
        print_usage(&program, &opts);
        exit(0);
    }
    if matches.opt_present("init") || matches.opt_present("init-local") {
        match init_config_file(matches.opt_present("init-local")) {
            Ok(()) => exit(0),
            Err(e) => {
                println!("{e}");
                exit(1);
            }
        }
    }
    let conf_file = if matches.opt_present("config") { matches.opt_str("config").map(PathBuf::from) } else { None };

    load_from_config(&mut conf, conf_file)?;
    parse_todo(&matches, &mut conf.todo)?;
    parse_sort(&matches, &mut conf.sort);
    parse_fmt(&matches, &mut conf.fmt);

    conf.stdin = matches.opt_present("stdin");
    conf.dry = matches.opt_present("dry-run");
    conf.verbose = matches.opt_present("verbose");
    conf.wipe = matches.opt_present("wipe");
    conf.use_done = matches.opt_present("done");
    conf.show_hidden = matches.opt_present("hidden");
    if conf.use_done && conf.flt.all == tfilter::TodoStatus::Active {
        conf.flt.all = tfilter::TodoStatus::Done;
    }
    if matches.opt_present("strict") || conf.stdin {
        conf.strict_mode = true;
    }
    if let Some(dstr) = matches.opt_str("calendar") {
        let rng = human_date::CalendarRange::parse(&dstr)?;
        conf.calendar = Some(rng);
    }

    detect_filenames(&matches, &mut conf);
    if conf.verbose {
        println!("Using main file: {:?}\n   archive file: {:?}", conf.todo_file, conf.done_file);
    }
    if matches.opt_present("syntax") {
        conf.fmt.syntax = true;
    } else if matches.opt_present("no-syntax") {
        conf.fmt.syntax = false;
    }
    if let Some(s) = matches.opt_str("clean-subject") {
        conf.fmt.hide = str_to_hide(&s);
    }

    if matches.opt_present("auto-show-cols") {
        conf.auto_show_columns = true;
    }
    if matches.opt_present("auto-hide-cols") {
        conf.auto_hide_columns = true;
    }
    if let Some(dstr) = matches.opt_str("always-hide-cols") {
        let mut v = Vec::new();
        for item in dstr.split(',') {
            v.push(item.to_string());
        }
        conf.always_hide_columns = v;
    }
    if let Some(s) = matches.opt_str("priority-on-done") {
        match str_to_pri_mode(&s) {
            Some(m) => conf.priority_on_done = m,
            None => {
                return Err(anyhow!(terr::TodoError::InvalidValue(
                    s.to_string(),
                    "priority completion mode".to_string()
                )));
            }
        }
    }
    if matches.opt_present("add-completion-date-always") {
        conf.add_completion_date_always = true;
    }
    conf.use_editor = matches.opt_present("interactive");

    if let Some(max_str) = matches.opt_str("max") {
        if let Ok(max) = max_str.parse::<usize>() {
            conf.max_items = Some(max);
        } else {
            return Err(anyhow!(terr::TodoError::InvalidValue(
                max_str.to_string(),
                "maximum number of items".to_string()
            )));
        }
    }
    let soon_days = conf.fmt.colors.soon_days;
    conf.keep_empty = matches.opt_present("keep-empty");
    conf.keep_tags = matches.opt_present("keep-tags");
    parse_filter(&matches, &mut conf.flt, soon_days)?;
    if let Some(f_str) = matches.opt_str("filter-tag") {
        conf.flt_ext = Some(f_str.clone());
    }
    if let Some(f_str) = matches.opt_str("filter") {
        conf.flt_ext = Some(f_str.clone());
    }
    conf.postpone_threshold = matches.opt_present("update-threshold");
    if let Some(f_str) = matches.opt_str("hide-fields") {
        conf.fmt.hide_fields = f_str.split(',').map(|itm| itm.to_string()).collect();
    }

    let mut idx: usize = 0;
    if idx >= matches.free.len() && !conf.stdin {
        conf.mode = RunMode::List;
        return Ok(conf);
    }

    // first should be command. In strict mode, the first argument must be a command
    if idx < matches.free.len() {
        conf.mode = str_to_mode(&matches.free[idx]);
    }
    if conf.mode != RunMode::None {
        idx += 1;
    } else if conf.strict_mode {
        return Err(anyhow!(terr::TodoError::NotCommand));
    }
    if idx >= matches.free.len() && !conf.stdin {
        // TODO: validity check
        return Ok(conf);
    }

    let edit_mode = conf.mode == RunMode::Add
        || conf.mode == RunMode::Edit
        || conf.mode == RunMode::None
        || conf.mode == RunMode::Append
        || conf.mode == RunMode::Prepend
        || conf.mode == RunMode::Postpone;

    if conf.use_editor && conf.mode != RunMode::Edit {
        eprintln!("Option '--interactive' can be used only with `edit` command");
        exit(1);
    }

    if conf.use_editor && conf.stdin {
        eprintln!("Option '--interactive' cannot be combined with --stdin");
        exit(1);
    }

    if idx < matches.free.len() {
        // second should be a range
        if matches.free[idx].find(|c: char| !c.is_ascii_digit()).is_none() {
            // a single ID
            if let Ok(id) = matches.free[idx].parse::<usize>() {
                conf.flt.range = tfilter::ItemRange::One(id - 1);
                idx += 1;
            }
        } else if is_id_range(&matches.free[idx]) {
            // a range in a form "ID1-ID2" or "ID1:ID2"
            let ends = parse_id_range(&matches.free[idx])?;
            if ends.l != RANGE_END_SKIP {
                conf.flt.range = tfilter::ItemRange::Range(ends.l, ends.r);
                idx += 1;
            }
        } else if matches.free[idx].find(|c: char| !c.is_ascii_digit() && c != ',' && c != '-' && c != ':').is_none() {
            // a list, possibly list of ranges
            let mut v: Vec<usize> = Vec::new();
            for s in matches.free[idx].split(',') {
                if is_id_range(s) {
                    let ends = parse_id_range(s)?;
                    if ends.l == RANGE_END_SKIP {
                        continue;
                    }
                    for id in ends.l..=ends.r {
                        if !v.contains(&id) {
                            v.push(id);
                        }
                    }
                    continue;
                }
                if let Ok(id) = s.parse::<usize>() {
                    let id = id - 1;
                    if !v.contains(&id) {
                        v.push(id);
                    }
                }
            }
            conf.flt.range = tfilter::ItemRange::List(v);
            idx += 1;
        }
    }

    while idx < matches.free.len() {
        let raw_arg = &matches.free[idx];
        process_single_free_arg(&mut conf, soon_days, edit_mode, raw_arg);
        idx += 1;
    }

    if conf.stdin && edit_mode {
        // Read all stdin input and process it as if it was free args elements
        let stdin = io::stdin();
        let stdin = stdin.lock();
        BufReader::new(stdin).lines().map_while(Result::ok).for_each(|s| {
            s.split_whitespace().for_each(|arg| process_single_free_arg(&mut conf, soon_days, edit_mode, arg))
        });
    }

    // TODO: validate
    Ok(conf)
}

fn process_single_free_arg(conf: &mut Conf, soon_days: u8, edit_mode: bool, raw_arg: &str) {
    let arg = raw_arg.trim_start();
    let has_space = arg.contains(' ');
    if arg.starts_with('@') && !has_space {
        let context = arg.trim_start_matches('@');
        conf.flt.include.contexts.push(context.to_owned().to_lowercase());
    } else if arg.starts_with("-@") && !has_space {
        let context = arg.trim_start_matches("-@");
        conf.flt.exclude.contexts.push(context.to_owned().to_lowercase());
    } else if arg.starts_with('+') && !has_space {
        let project = arg.trim_start_matches('+');
        conf.flt.include.projects.push(project.to_owned().to_lowercase());
    } else if arg.starts_with("-+") && !has_space {
        let project = arg.trim_start_matches("-+");
        conf.flt.exclude.projects.push(project.to_owned().to_lowercase());
    } else if edit_mode {
        let dt = Local::now().date_naive();
        let subj = match human_date::fix_date(dt, arg, "due:", soon_days) {
            None => raw_arg.to_string(),
            Some(s) => s,
        };
        let subj = match human_date::fix_date(dt, &subj, "t:", soon_days) {
            None => subj,
            Some(s) => s,
        };

        // Append content to the todo text
        conf.todo.subject = conf
            .todo
            .subject
            .as_ref()
            .map_or(Some(subj.to_string()), |old_subj| Some([old_subj.as_str(), subj.as_str()].join(" ")));
    } else {
        conf.flt.regex = Some(arg.to_string());
    }
}

// Parses a range in a form "ID1-ID2" or "ID1:ID2".
// Returns range ends. RangeEnds.l is always less than or equal to RangeEnds.r
fn parse_id_range(s: &str) -> Result<RangeEnds, terr::TodoError> {
    let w: Vec<&str> = if s.find('-').is_none() { s.split(':').collect() } else { s.split('-').collect() };
    if w.len() != 2 {
        return Err(terr::TodoError::InvalidValue(s.to_owned(), "ID range".to_string()));
    }
    match (w[0].parse::<usize>(), w[1].parse::<usize>()) {
        (Ok(id1), Ok(id2)) => {
            if id1 <= id2 {
                Ok(RangeEnds { l: id1 - 1, r: id2 - 1 })
            } else {
                Ok(RangeEnds { l: id2 - 1, r: id1 - 1 })
            }
        }
        (_, _) => Ok(RangeEnds { l: RANGE_END_SKIP, r: RANGE_END_SKIP }),
    }
}

fn is_id_range(s: &str) -> bool {
    if s.find(|c: char| !c.is_ascii_digit() && c != '-' && c != ':').is_some() {
        return false;
    }
    s.contains(['-', ':'])
}

fn color_from_str(s: &str) -> Result<ColorSpec> {
    let mut spc = ColorSpec::new();

    if s.find(' ').is_none() {
        let c = Color::from_str(s)?;
        spc.set_fg(Some(c));
        return Ok(spc);
    }

    let lows = s.to_lowercase();
    for clr in lows.split_whitespace() {
        match clr {
            "black" => {
                spc.set_fg(Some(Color::Black));
            }
            "white" => {
                spc.set_fg(Some(Color::White));
            }
            "red" => {
                spc.set_fg(Some(Color::Red));
            }
            "green" => {
                spc.set_fg(Some(Color::Green));
            }
            "yellow" => {
                spc.set_fg(Some(Color::Yellow));
            }
            "blue" => {
                spc.set_fg(Some(Color::Blue));
            }
            "cyan" => {
                spc.set_fg(Some(Color::Cyan));
            }
            "magenta" => {
                spc.set_fg(Some(Color::Magenta));
            }
            "bright" | "intense" => {
                spc.set_intense(true);
            }
            "underline" => {
                spc.set_underline(true);
            }
            "bold" => {
                spc.set_bold(true);
            }
            _ => return Err(anyhow!("Unknown color '{}'", clr)),
        };
    }

    Ok(spc)
}

pub fn parse_range(s: &str) -> Option<(&str, &str)> {
    s.find("..").map(|pos| (&s[..pos], &s[pos + "..".len()..]))
}

/// Returns true if the given `mode` can be used for `done.txt`.
/// Most of the modes are available exclusively for `todo.txt`.
pub fn can_run_for_done(mode: RunMode) -> bool {
    matches!(mode, RunMode::List | RunMode::Stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_range_test() {
        struct Test {
            input: &'static str,
            res: Vec<&'static str>,
        }
        let tests: Vec<Test> = vec![
            Test { input: "..right", res: vec!["", "right"] },
            Test { input: "left..", res: vec!["left", ""] },
            Test { input: "left..right", res: vec!["left", "right"] },
            Test { input: "left..one..more..right", res: vec!["left", "one..more..right"] },
        ];
        for (idx, test) in tests.iter().enumerate() {
            let (b, e) = parse_range(test.input).unwrap();
            assert_eq!(b, test.res[0], "{}. '{}' != '{}'", idx, b, test.res[0]);
            assert_eq!(e, test.res[1], "{}. '{}' != '{}'", idx, e, test.res[1]);
        }
    }
}
