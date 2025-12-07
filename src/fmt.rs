use std::cmp::Ordering;
use std::io::{self, Write};
use std::process::{Command, Stdio};

use caseless::default_caseless_match_str;
use chrono::{Duration, Local, NaiveDate};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use todo_lib::{timer, todo, todotxt};
use unicode_width::UnicodeWidthStr;

use crate::colauto::cleanup_description;
use crate::conv;
use crate::human_date;
use crate::subj_clean::{Hide, hide_any};

const SPENT_WIDTH: usize = 6;
const JSON_DESC: &str = "description";
const JSON_OPT: &str = "optional";
const JSON_SPEC: &str = "specialTags";
const PLUG_PREFIX: &str = "ttdl-";

// Default sizes for custom field types
const INT_LENGTH: usize = 12;
const FLOAT_LENGTH: usize = 15;
const DURATION_LENGTH: usize = 10;
const STR_LENGTH: usize = 15;
const BYTES_LENGTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Full,
    Short,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LongLine {
    Simple,
    WordWrap,
    Cut,
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub top: ColorSpec,
    pub important: ColorSpec,
    pub overdue: ColorSpec,
    pub threshold: ColorSpec,
    pub today: ColorSpec,
    pub soon: ColorSpec,
    pub done: ColorSpec,
    pub old: ColorSpec,
    pub hashtag: ColorSpec,
    pub tag: ColorSpec,
    pub context: ColorSpec,
    pub project: ColorSpec,
    pub default_fg: ColorSpec,

    pub important_limit: u8,
    pub soon_days: u8,
    pub old_period: Option<todotxt::Recurrence>,
}
fn default_done_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Black));
    spc.set_intense(true);
    spc
}
fn default_top_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Red));
    spc.set_intense(true);
    spc
}
fn default_overdue_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Red));
    spc.set_intense(true);
    spc
}
fn default_today_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Yellow));
    spc.set_intense(true);
    spc
}
fn default_threshold_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Red));
    spc
}
pub(crate) fn default_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(None);
    spc
}
pub(crate) fn default_project_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Green));
    spc.set_intense(true);
    spc
}
pub(crate) fn default_context_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Green));
    spc
}
pub(crate) fn default_tag_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Cyan));
    spc.set_intense(true);
    spc
}
pub(crate) fn default_hashtag_color() -> ColorSpec {
    let mut spc = ColorSpec::new();
    spc.set_fg(Some(Color::Cyan));
    spc
}

impl Default for Colors {
    fn default() -> Colors {
        Colors {
            top: default_top_color(),
            important: default_color(),
            overdue: default_overdue_color(),
            threshold: default_threshold_color(),
            today: default_today_color(),
            soon: default_color(),
            done: default_done_color(),
            old: default_done_color(),
            context: default_context_color(),
            project: default_project_color(),
            tag: default_tag_color(),
            hashtag: default_hashtag_color(),
            default_fg: default_color(),

            important_limit: todotxt::NO_PRIORITY,
            soon_days: 0u8,
            old_period: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermColorType {
    None,
    Auto,
    Ansi,
}

#[derive(Debug, Clone)]
pub enum FmtSpec {
    Range(String, String),
    List(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct FmtRule {
    pub range: FmtSpec,
    pub color: ColorSpec,
}

impl FmtRule {
    fn matches_int(&self, value: &str) -> Option<ColorSpec> {
        let v = value.parse::<i64>().ok()?;
        match &self.range {
            FmtSpec::List(l) => {
                for item in l.iter() {
                    if let Ok(vv) = item.parse::<i64>()
                        && vv == v
                    {
                        return Some(self.color.clone());
                    }
                }
                None
            }
            FmtSpec::Range(b, e) => {
                if b.is_empty() && e.is_empty() {
                    return Some(self.color.clone());
                }
                if b.is_empty() {
                    let e = e.parse::<i64>().ok()?;
                    if v <= e {
                        return Some(self.color.clone());
                    }
                } else if e.is_empty() {
                    let b = b.parse::<i64>().ok()?;
                    if v >= b {
                        return Some(self.color.clone());
                    }
                } else {
                    let e = e.parse::<i64>().ok()?;
                    let b = b.parse::<i64>().ok()?;
                    if e >= b && (b..=e).contains(&v) {
                        return Some(self.color.clone());
                    }
                }
                None
            }
        }
    }
    fn matches_float(&self, value: &str) -> Option<ColorSpec> {
        let v = value.parse::<f64>().ok()?;
        match &self.range {
            FmtSpec::List(l) => {
                for item in l.iter() {
                    if let Ok(vv) = item.parse::<f64>()
                        && vv == v
                    {
                        return Some(self.color.clone());
                    }
                }
                None
            }
            FmtSpec::Range(b, e) => {
                if b.is_empty() && e.is_empty() {
                    return Some(self.color.clone());
                }
                if b.is_empty() {
                    let e = e.parse::<f64>().ok()?;
                    if v <= e {
                        return Some(self.color.clone());
                    }
                } else if e.is_empty() {
                    let b = b.parse::<f64>().ok()?;
                    if v >= b {
                        return Some(self.color.clone());
                    }
                } else {
                    let e = e.parse::<f64>().ok()?;
                    let b = b.parse::<f64>().ok()?;
                    if v >= b && v <= e {
                        return Some(self.color.clone());
                    }
                }
                None
            }
        }
    }
    fn matches_str(&self, value: &str) -> Option<ColorSpec> {
        match &self.range {
            FmtSpec::List(l) => {
                if l.iter().any(|s| s.as_str() == value) {
                    return Some(self.color.clone());
                }
                None
            }
            FmtSpec::Range(b, e) => {
                if b.is_empty() && e.is_empty() {
                    return Some(self.color.clone());
                }
                if b.is_empty() {
                    if value <= e.as_str() {
                        return Some(self.color.clone());
                    }
                } else if e.is_empty() {
                    if value >= b.as_str() {
                        return Some(self.color.clone());
                    }
                } else if value >= b.as_str() && value <= e.as_str() {
                    return Some(self.color.clone());
                }
                None
            }
        }
    }
    fn matches_date(&self, value: &str) -> Option<ColorSpec> {
        let today = Local::now().date_naive();
        let v = todotxt::parse_date(value, today).ok()?;
        match &self.range {
            FmtSpec::List(l) => {
                for item in l.iter() {
                    let vv = human_date::human_to_date(today, item, 7).ok()?;
                    if vv == v {
                        return Some(self.color.clone());
                    }
                }
                None
            }
            FmtSpec::Range(b, e) => {
                if b.is_empty() && e.is_empty() {
                    return Some(self.color.clone());
                }
                if b.is_empty() {
                    let e = human_date::human_to_date(today, e, 7).ok()?;
                    if v <= e {
                        return Some(self.color.clone());
                    }
                } else if e.is_empty() {
                    let b = human_date::human_to_date(today, b, 7).ok()?;
                    if v >= b {
                        return Some(self.color.clone());
                    }
                } else {
                    let e = human_date::human_to_date(today, e, 7).ok()?;
                    let b = human_date::human_to_date(today, b, 7).ok()?;
                    if v >= b && v <= e {
                        return Some(self.color.clone());
                    }
                }
                None
            }
        }
    }
    fn matches_bytes(&self, value: &str) -> Option<ColorSpec> {
        let v = conv::str_to_bytes(value)?;
        match &self.range {
            FmtSpec::List(l) => {
                for item in l.iter() {
                    let vv = conv::str_to_bytes(item)?;
                    if vv == v {
                        return Some(self.color.clone());
                    }
                }
                None
            }
            FmtSpec::Range(b, e) => {
                if b.is_empty() && e.is_empty() {
                    return Some(self.color.clone());
                }
                if b.is_empty() {
                    let e = conv::str_to_bytes(e)?;
                    if v <= e {
                        return Some(self.color.clone());
                    }
                } else if e.is_empty() {
                    let b = conv::str_to_bytes(b)?;
                    if v >= b {
                        return Some(self.color.clone());
                    }
                } else {
                    let e = conv::str_to_bytes(e)?;
                    let b = conv::str_to_bytes(b)?;
                    if e >= b && (b..=e).contains(&v) {
                        return Some(self.color.clone());
                    }
                }
                None
            }
        }
    }
    fn matches_duration(&self, value: &str) -> Option<ColorSpec> {
        let v = conv::str_to_duration(value)?;
        match &self.range {
            FmtSpec::List(l) => {
                for item in l.iter() {
                    let vv = conv::str_to_duration(item)?;
                    if vv == v {
                        return Some(self.color.clone());
                    }
                }
                None
            }
            FmtSpec::Range(b, e) => {
                if b.is_empty() && e.is_empty() {
                    return Some(self.color.clone());
                }
                if b.is_empty() {
                    let e = conv::str_to_duration(e)?;
                    if v <= e {
                        return Some(self.color.clone());
                    }
                } else if e.is_empty() {
                    let b = conv::str_to_duration(b)?;
                    if v >= b {
                        return Some(self.color.clone());
                    }
                } else {
                    let e = conv::str_to_duration(e)?;
                    let b = conv::str_to_duration(b)?;
                    if e >= b && (b..=e).contains(&v) {
                        return Some(self.color.clone());
                    }
                }
                None
            }
        }
    }

    fn matches(&self, value: &str, kind: &str) -> Option<ColorSpec> {
        match kind {
            "int" | "integer" => self.matches_int(value),
            "float" => self.matches_float(value),
            "date" => self.matches_date(value),
            "duration" => self.matches_duration(value),
            "bytes" => self.matches_bytes(value),
            "str" | "string" => self.matches_str(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CustomField {
    pub name: String,
    pub title: String,
    pub width: u16,
    pub kind: String,
    pub rules: Vec<FmtRule>,
}

impl CustomField {
    fn matches(&self, val: &str) -> Option<ColorSpec> {
        for rule in self.rules.iter() {
            let clr = rule.matches(val, &self.kind);
            if clr.is_some() {
                return clr;
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Conf {
    pub fmt: Format,
    pub width: u16,
    pub long: LongLine,
    pub fields: Vec<String>,
    pub human: bool,
    pub human_fields: Vec<String>,
    pub compact: bool,
    pub color_term: TermColorType,
    pub max: usize,
    pub colors: Colors,
    pub atty: bool,
    pub shell: Vec<String>,
    pub script_ext: String,
    pub script_prefix: String,
    pub syntax: bool,
    pub custom_fields: Vec<CustomField>,
    pub custom_names: Vec<String>, // for performance
    pub hide: Hide,
    pub group: Option<String>,
    pub hide_headers: bool,
    pub hide_fields: Vec<String>,
}

impl Default for Conf {
    fn default() -> Conf {
        #[cfg(windows)]
        let shell = vec!["cmd".to_string(), "/c".to_string()];
        #[cfg(not(windows))]
        let shell = vec!["sh".to_string(), "-cu".to_string()];
        Conf {
            fmt: Format::Full,
            width: 0,
            long: LongLine::Simple,
            fields: Vec::new(),
            human: false,
            human_fields: Vec::new(),
            color_term: TermColorType::Auto,
            max: 0,
            compact: false,
            colors: Default::default(),
            atty: true,
            script_ext: String::new(),
            script_prefix: String::new(),
            shell,
            syntax: false,
            custom_fields: Vec::new(),
            custom_names: Vec::new(),
            hide: Hide::Nothing,
            group: None,
            hide_headers: false,
            hide_fields: Vec::new(),
        }
    }
}

impl Conf {
    pub fn is_human(&self, s: &str) -> bool {
        if !self.human {
            return false;
        }

        if self.human_fields.is_empty() {
            return true;
        }

        for f in self.human_fields.iter() {
            if f == s {
                return true;
            }
        }

        false
    }
    fn custom_field_width(&self, name: &str) -> usize {
        for f in self.custom_fields.iter() {
            if name != f.name.as_str() {
                continue;
            }
            if f.width != 0 {
                return usize::from(f.width);
            }
            match f.kind.as_str() {
                "integer" | "int" => return INT_LENGTH,
                "float" => return FLOAT_LENGTH,
                "duration" => return DURATION_LENGTH,
                "date" => return "2002-10-10".width(),
                "bytes" => return BYTES_LENGTH,
                "str" | "string" => return STR_LENGTH,
                _ => return 0,
            }
        }
        0
    }
    pub fn custom_field(&self, name: &str) -> Option<&CustomField> {
        self.custom_fields.iter().find(|&f| name == f.name.as_str())
    }
}

pub fn number_of_digits(val: usize) -> usize {
    match val {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        1000..=9999 => 4,
        _ => 5,
    }
}

// Returns the width of all columns without subject and the length of subject
fn calc_width(c: &Conf, fields: &[String], widths: &[usize]) -> (usize, usize) {
    let mut before: usize = field_width_cached("id", fields, widths) + 1;
    for f in fields {
        if f.as_str() == "id" {
            continue;
        }
        before += field_width_cached(f, fields, widths) + 1;
    }

    // Do not wrap in long mode or when the subject columns gets too narrow
    let swidth = "Subject".width() + 1;
    if c.long == LongLine::Simple || usize::from(c.width) <= before + swidth {
        (before, 1000)
    } else {
        (before, c.width as usize - before)
    }
}

fn print_header_line(stdout: &mut StandardStream, c: &Conf, fields: &[String], widths: &[usize]) -> io::Result<()> {
    write!(stdout, "{:>wid$} ", "#", wid = field_width_cached("id", fields, widths))?;

    let mut subj_printed = false;
    for f in fields {
        let width = field_width_cached(f, fields, widths);
        match f.as_str() {
            "id" => continue,
            "done" => write!(stdout, "D ")?,
            "pri" => write!(stdout, "P ")?,
            "created" => write!(stdout, "{:wid$} ", "Created", wid = width)?,
            "finished" => write!(stdout, "{:wid$} ", "Finished", wid = width)?,
            "due" => write!(stdout, "{:wid$} ", "Due", wid = width)?,
            "thr" => {
                if c.is_human(f) && c.compact {
                    write!(stdout, "{:wid$} ", "Thr", wid = width)?;
                } else {
                    write!(stdout, "{:wid$} ", "Threshold", wid = width)?;
                }
            }
            "spent" => write!(stdout, "{:wid$} ", "Spent", wid = SPENT_WIDTH)?,
            "uid" => write!(stdout, "{:wid$}", "UID", wid = width + 1)?,
            "parent" => write!(stdout, "{:wid$}", "Parent", wid = width + 1)?,
            "prj" => write!(stdout, "{:wid$}", "Project", wid = width + 1)?,
            "ctx" => write!(stdout, "{:wid$}", "Context", wid = width + 1)?,
            "until" => write!(stdout, "{:wid$}", "Until", wid = width + 1)?,
            n => {
                subj_printed = default_caseless_match_str(n, "subject");
                if let Some(f) = c.custom_field(n) {
                    write!(stdout, "{:wid$}", f.title, wid = width + 1)?;
                } else {
                    let title: String = f.chars().take(1).collect();
                    let title = title.to_uppercase();
                    let rest: String = f.chars().skip(1).collect();
                    let title = title + &rest.to_lowercase();
                    write!(stdout, "{:wid$}", title, wid = width + 1)?;
                }
            }
        }
    }

    if !subj_printed { writeln!(stdout, "Subject") } else { writeln!(stdout) }
}

fn color_for_priority(task: &todotxt::Task, c: &Conf) -> ColorSpec {
    if task.finished {
        return c.colors.done.clone();
    }

    if task.priority == 0
        || (task.priority < c.colors.important_limit && c.colors.important_limit != todotxt::NO_PRIORITY)
    {
        if task.priority == 0 {
            return c.colors.top.clone();
        } else {
            return c.colors.important.clone();
        }
    }

    c.colors.default_fg.clone()
}

fn color_for_creation_date(task: &todotxt::Task, c: &Conf) -> ColorSpec {
    let spc = c.colors.default_fg.clone();
    if task.finished {
        return c.colors.done.clone();
    }
    if task.recurrence.is_some() {
        return spc;
    }
    let rec = match &c.colors.old_period {
        None => {
            return spc;
        }
        Some(r) => r.clone(),
    };

    if let Some(ref cd) = task.create_date {
        let today = Local::now().date_naive();
        let mcreate = rec.next_date(*cd);
        if mcreate < today {
            return c.colors.old.clone();
        }
    }

    spc
}

fn color_for_due_date(task: &todotxt::Task, days: i64, c: &Conf) -> ColorSpec {
    let spc = c.colors.default_fg.clone();
    if task.finished {
        return c.colors.done.clone();
    }

    if task.due_date.is_none() || days > i64::from(c.colors.soon_days) {
        return spc;
    }

    if days < 0 {
        return c.colors.overdue.clone();
    }

    if days == 0 {
        return c.colors.today.clone();
    }

    if c.colors.soon_days > 0 && days <= i64::from(c.colors.soon_days) {
        return c.colors.soon.clone();
    }

    spc
}

fn color_for_threshold_date(task: &todotxt::Task, days: i64, c: &Conf) -> ColorSpec {
    let spc = c.colors.default_fg.clone();
    if task.finished {
        return c.colors.done.clone();
    }

    if task.threshold_date.is_none() {
        return spc;
    }

    if days < 0 {
        return c.colors.threshold.clone();
    }

    spc
}

fn print_with_color(stdout: &mut StandardStream, msg: &str, color: &ColorSpec) -> io::Result<()> {
    stdout.set_color(color)?;
    write!(stdout, "{msg}")
}

fn print_with_highlight(stdout: &mut StandardStream, msg: &str, color: &ColorSpec, c: &Conf) -> io::Result<()> {
    if !c.syntax {
        return print_with_color(stdout, msg, color);
    }
    let words = parse_subj(msg);
    for word in words.iter() {
        if is_project(word) {
            stdout.set_color(&c.colors.project)?;
        } else if is_hashtag(word) {
            stdout.set_color(&c.colors.hashtag)?;
        } else if is_context(word) {
            stdout.set_color(&c.colors.context)?;
        } else if is_tag(word) {
            stdout.set_color(&c.colors.tag)?;
        } else {
            stdout.set_color(color)?;
        }
        write!(stdout, "{word}")?;
    }
    Ok(())
}

pub fn done_str(task: &todotxt::Task) -> String {
    if timer::is_timer_on(task) {
        "T ".to_string()
    } else if task.finished {
        "x ".to_string()
    } else if task.recurrence.is_some() {
        "R ".to_string()
    } else {
        "  ".to_string()
    }
}

pub fn priority_str(task: &todotxt::Task) -> String {
    if task.priority < todotxt::NO_PRIORITY { format!("{} ", (b'A' + task.priority) as char) } else { "  ".to_string() }
}

pub fn duration_str(d: Duration) -> String {
    let s = d.num_seconds();
    if s <= 0 {
        return String::new();
    }

    if s < 60 {
        format!("{s}s")
    } else if s < 60 * 60 {
        format!("{:.1}m", (s as f64) / 60.0)
    } else if s < 60 * 60 * 24 {
        format!("{:.1}h", (s as f64) / 60.0 / 60.0)
    } else if s < 60 * 60 * 24 * 30 {
        format!("{:.1}D", (s as f64) / 60.0 / 60.0 / 24.0)
    } else if s < 60 * 60 * 24 * 30 * 12 {
        format!("{:.1}M", (s as f64) / 60.0 / 60.0 / 24.0 / 30.0)
    } else {
        format!("{:.1}Y", (s as f64) / 60.0 / 60.0 / 24.0 / 30.0 / 12.0)
    }
}

fn arg_field_as_str(arg: &json::JsonValue, field: &str) -> Option<String> {
    if arg == &json::JsonValue::Null || !arg.is_array() {
        return None;
    }
    for m in arg.members() {
        if !m.is_object() {
            continue;
        }
        for e in m.entries() {
            let (key, val) = e;
            if key != field {
                continue;
            }
            if let Some(st) = val.as_str() {
                return Some(st.to_string());
            }
        }
    }
    None
}

fn print_done_val(
    stdout: &mut StandardStream,
    task: &todotxt::Task,
    arg: &json::JsonValue,
    def_color: &termcolor::ColorSpec,
) -> io::Result<()> {
    let mut s = done_str(task);
    if !arg.is_empty() {
        let tags = &arg[JSON_OPT];
        if let Some(v) = arg_field_as_str(tags, "done") {
            s = format!("{:wid$.wid$}", v, wid = 2);
        }
    };
    print_with_color(stdout, &s, def_color)
}

fn print_priority_val(
    stdout: &mut StandardStream,
    task: &todotxt::Task,
    arg: &json::JsonValue,
    c: &Conf,
) -> io::Result<()> {
    let mut s = priority_str(task);
    if !arg.is_empty() {
        let tags = &arg[JSON_OPT];
        if let Some(v) = arg_field_as_str(tags, "pri") {
            s = format!("{:wid$.wid$}", v, wid = 2);
        }
    }
    print_with_color(stdout, &s, &color_for_priority(task, c))
}

#[allow(clippy::too_many_arguments)]
fn print_date_val(
    stdout: &mut StandardStream,
    arg: &json::JsonValue,
    c: &Conf,
    field: &str,
    dt: Option<&chrono::NaiveDate>,
    def_color: termcolor::ColorSpec,
    flist: &[String],
    widths: &[usize],
) -> io::Result<()> {
    let width = field_width_cached(field, flist, widths);
    let mut st = if let Some(d) = dt {
        if c.is_human(field) {
            let (s, _) = format_relative_date(*d, c.compact);
            format!("{s:width$} ")
        } else {
            format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = width)
        }
    } else {
        format!("{:wid$} ", " ", wid = width)
    };
    if !arg.is_empty() {
        let tags = if field == "created" || field == "finished" { &arg[JSON_OPT] } else { &arg[JSON_SPEC] };
        if let Some(v) = arg_field_as_str(tags, field) {
            st = format!("{v:width$.width$} ");
        }
    }
    print_with_color(stdout, &st, &def_color)
}

fn print_line(
    stdout: &mut StandardStream,
    task: &todotxt::Task,
    id: usize,
    c: &Conf,
    flist: &[String],
    widths: &[usize],
) -> io::Result<()> {
    let id_width = field_width_cached("id", flist, widths);
    let fg = if task.finished { c.colors.done.clone() } else { c.colors.default_fg.clone() };

    let (mut desc, arg) =
        if let Some(tpl) = external_reconstruct(task, c) { tpl } else { (String::new(), json::JsonValue::Null) };
    if desc.is_empty() {
        desc.clone_from(&task.subject);
    }

    print_with_color(stdout, &format!("{id:>id_width$} "), &fg)?;

    let fs: Vec<&str> = flist.iter().map(|it| it.as_str()).collect();
    cleanup_description(&mut desc, &fs, c);
    if !c.hide_fields.is_empty() {
        for fld in c.hide_fields.iter() {
            let tg = format!(" {fld}:");
            hide_any(&mut desc, &tg);
        }
    }
    let mut subj_printed = false;
    for f in flist.iter() {
        match f.as_str() {
            "id" => {
                continue;
            }
            "done" => {
                print_done_val(stdout, task, &arg, &fg)?;
            }
            "pri" => {
                print_priority_val(stdout, task, &arg, c)?;
            }
            "created" => {
                let clr = color_for_creation_date(task, c);
                let dt = task.create_date.as_ref();
                print_date_val(stdout, &arg, c, "created", dt, clr, flist, widths)?;
            }
            "finished" => {
                let dt = task.finish_date.as_ref();
                let clr = fg.clone();
                print_date_val(stdout, &arg, c, "finished", dt, clr, flist, widths)?;
            }
            "due" => {
                let dt = task.due_date.as_ref();
                let mut clr = if task.finished { c.colors.done.clone() } else { c.colors.default_fg.clone() };
                if let Some(d) = task.due_date.as_ref() {
                    let (_, days) = format_relative_due_date(*d, c.compact);
                    clr = color_for_due_date(task, days, c);
                }
                print_date_val(stdout, &arg, c, "due", dt, clr, flist, widths)?;
            }
            "thr" => {
                let dt = task.threshold_date.as_ref();
                let mut clr = if task.finished { c.colors.done.clone() } else { c.colors.default_fg.clone() };
                if let Some(d) = task.threshold_date.as_ref() {
                    let (_, days) = format_relative_due_date(*d, c.compact);
                    clr = color_for_threshold_date(task, days, c);
                }
                print_date_val(stdout, &arg, c, "thr", dt, clr, flist, widths)?;
            }
            "spent" => {
                print_with_color(
                    stdout,
                    &format!("{:wid$} ", &duration_str(timer::spent_time(task)), wid = SPENT_WIDTH),
                    &fg,
                )?;
            }
            "uid" | "parent" => {
                let width = field_width_cached(f, flist, widths);
                let name = if *f == "uid" { "id" } else { f.as_str() };
                let empty_str = String::new();
                let value = task.tags.get(name).unwrap_or(&empty_str);
                print_with_color(stdout, &format!("{value:width$} "), &fg)?;
            }
            "prj" => {
                let width = field_width_cached(f, flist, widths);
                let mut v = String::new();
                for prj in &task.projects {
                    if !v.is_empty() {
                        v += ",";
                    }
                    v += prj;
                }
                print_with_color(stdout, &format!("{v:width$} "), &fg)?;
            }
            "ctx" => {
                let width = field_width_cached(f, flist, widths);
                let mut v = String::new();
                for ctx in &task.contexts {
                    if !v.is_empty() {
                        v += ",";
                    }
                    v += ctx;
                }
                print_with_color(stdout, &format!("{v:width$} "), &fg)?;
            }
            "until" => {
                let width = field_width_cached(f, flist, widths);
                if let Some(v) = task.tags.get("until") {
                    let now = chrono::Local::now().date_naive();
                    if let Ok(dt) = todotxt::parse_date(v, now) {
                        let clr = fg.clone();
                        let odt = Some(&dt);
                        print_date_val(stdout, &arg, c, "until", odt, clr, flist, widths)?;
                    } else {
                        let v = "";
                        print_with_color(stdout, &format!("{v:width$} "), &fg)?;
                    }
                } else {
                    let v = "";
                    print_with_color(stdout, &format!("{v:width$} "), &fg)?;
                }
            }
            n => {
                if let Some(f) = c.custom_field(n) {
                    let width = c.custom_field_width(n);
                    let value = match task.tags.get(&f.name) {
                        None => String::new(),
                        Some(s) => s.to_string(),
                    };
                    let value = conv::cut_string(&value, width);
                    let clr = f.matches(value).unwrap_or_else(|| fg.clone());
                    print_with_color(stdout, &format!("{value:width$} "), &clr)?;
                } else {
                    let width = field_width_cached(f, flist, widths);
                    let value = match task.tags.get(f) {
                        None => {
                            if default_caseless_match_str(n, "subject") {
                                subj_printed = true;
                                desc.clone()
                            } else {
                                String::new()
                            }
                        }
                        Some(s) => s.to_string(),
                    };
                    let value = conv::cut_string(&value, width);
                    print_with_color(stdout, &format!("{value:width$} "), &fg)?;
                }
            }
        }
    }

    if !subj_printed {
        if c.width != 0 && c.long != LongLine::Simple {
            let (skip, subj_w) = calc_width(c, flist, widths);
            let lines = textwrap::wrap(&desc, subj_w);
            if c.long == LongLine::Cut || lines.len() == 1 {
                print_with_highlight(stdout, &format!("{}\n", &lines[0]), &fg, c)?;
            } else {
                for (i, line) in lines.iter().enumerate() {
                    if i != 0 {
                        print_with_highlight(stdout, &format!("{:width$}", " ", width = skip), &fg, c)?;
                    }
                    print_with_highlight(stdout, &format!("{}\n", &line), &fg, c)?;
                }
            }
        } else {
            print_with_highlight(stdout, &format!("{}\n", &desc), &fg, c)?;
        }
    } else {
        print_with_highlight(stdout, "\n", &fg, c)?;
    }
    stdout.set_color(&c.colors.default_fg)
}

fn customize(task: &todotxt::Task, c: &Conf) -> Option<json::JsonValue> {
    let mut ext_cmds: Vec<String> = Vec::new();
    for (key, _val) in task.tags.iter() {
        if key.starts_with('!') {
            ext_cmds.push(key.to_string());
        }
    }
    if ext_cmds.is_empty() {
        return None;
    }

    let mut arg = build_ext_arg(task);
    for cmd in ext_cmds {
        if !command_in_json(&arg, &cmd) {
            continue;
        }
        let bin_name = format!("{PLUG_PREFIX}{0}", cmd.trim_start_matches('!'));
        let args = json::stringify(arg);
        let output = exec_plugin(c, &bin_name, &args);
        match output {
            Err(e) => {
                eprintln!("Failed to execute plugin '{bin_name}': {e}");
                return None;
            }
            Ok(s) => match json::parse(&s) {
                Ok(j) => arg = j,
                Err(e) => {
                    eprintln!("Failed to parse output of plugin {bin_name}: {e}\nOutput: {s}");
                    return None;
                }
            },
        }
    }
    Some(arg)
}

#[allow(clippy::format_push_string)]
fn external_reconstruct(task: &todotxt::Task, c: &Conf) -> Option<(String, json::JsonValue)> {
    let arg = customize(task, c)?;
    let mut res = if let Some(s) = arg[JSON_DESC].as_str() { s.to_string() } else { task.subject.clone() };
    let tags = &arg[JSON_SPEC];
    if !tags.is_array() || tags.is_empty() {
        return None;
    }
    for m in tags.members() {
        if !m.is_object() {
            continue;
        }
        for e in m.entries() {
            let (key, val) = e;
            let key_s = key.to_string();
            let old_val = match task.tags.get(&key_s) {
                Some(v) => v.to_string(),
                None => String::new(),
            };
            let new_val = val.as_str().unwrap_or("");
            if new_val.is_empty() && old_val.is_empty() {
                continue;
            }
            if old_val.is_empty() {
                res += &(format!(" {key}:{new_val}"));
                continue;
            }
            let old_pair = format!("{key}:{old_val}");
            let new_pair = if new_val.is_empty() { String::new() } else { format!("{key}:{new_val}") };
            todotxt::replace_word(&mut res, old_pair.as_str(), new_pair.as_str());
        }
    }

    Some((res, arg))
}

#[allow(clippy::format_push_string)]
fn exec_plugin(c: &Conf, plugin: &str, args: &str) -> Result<String, String> {
    let mut plugin_bin = plugin.to_string();
    if !c.script_prefix.is_empty() {
        plugin_bin = format!("{0}{plugin_bin}", c.script_prefix);
    }
    if !c.script_ext.is_empty() {
        if c.script_ext.starts_with('.') {
            plugin_bin += &c.script_ext;
        } else {
            plugin_bin += &format!(".{}", c.script_ext);
        }
    }

    let mut cmd = Command::new(&c.shell[0]);
    for shell_arg in c.shell[1..].iter() {
        cmd.arg(shell_arg);
    }

    cmd.arg(plugin_bin);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());

    let mut proc = match cmd.spawn() {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to execute {plugin}: {e}")),
    };
    {
        let stdin = match proc.stdin.as_mut() {
            Some(si) => si,
            None => return Err(format!("Failed to open stdin for {plugin}")),
        };
        if let Err(e) = stdin.write_all(args.as_bytes()) {
            return Err(format!("Failed to write to {plugin} stdin: {e}"));
        }
    }

    let out = match proc.wait_with_output() {
        Ok(o) => o,
        Err(e) => return Err(format!("Failed to read {plugin} stdout: {e}")),
    };
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// {"description": "Desc", "specialTags":[{"tag1": "val"},], "optional":[{"pri": "A"}]}
fn build_ext_arg(task: &todotxt::Task) -> json::JsonValue {
    let mut jarr = json::JsonValue::new_array();
    for (key, val) in task.tags.iter() {
        let _ = jarr.push(json::object! { key => val.to_string() });
    }
    if let Some(d) = task.due_date.as_ref() {
        let s = format!("{}", (*d).format("%Y-%m-%d"));
        let _ = jarr.push(json::object! { "due" => s });
    }
    if let Some(d) = task.threshold_date.as_ref() {
        let s = format!("{}", (*d).format("%Y-%m-%d"));
        let _ = jarr.push(json::object! { "thr" => s });
    }
    let mut optional = json::JsonValue::new_array();
    let _ = optional.push(json::object! { "done" => done_str(task) });
    let _ = optional.push(json::object! { "pri" => priority_str(task) });
    if let Some(d) = task.create_date.as_ref() {
        let s = format!("{}", (*d).format("%Y-%m-%d"));
        let _ = optional.push(json::object! { "created" => s });
    }
    if let Some(d) = task.finish_date.as_ref() {
        let s = format!("{}", (*d).format("%Y-%m-%d"));
        let _ = optional.push(json::object! { "finished" => s });
    }
    json::object! {
        JSON_DESC => task.subject.clone(),
        JSON_SPEC => jarr,
        JSON_OPT => optional,
    }
}

fn command_in_json(val: &json::JsonValue, key: &str) -> bool {
    let arr = &val[JSON_SPEC];
    if !arr.is_array() {
        return false;
    }
    for m in arr.members() {
        if !m.is_object() {
            continue;
        }
        if m.has_key(key) {
            return true;
        }
    }
    false
}

fn format_days(num: i64, compact: bool) -> String {
    let num = if num < 0 { -num } else { num };
    match num {
        0 => {
            if compact {
                "!".to_string()
            } else {
                "today".to_string()
            }
        }
        days @ 1..=6 => format!("{days}d"),
        days @ 7..=29 => format!("{}w", days / 7),
        days @ 30..=364 => format!("{}m", days / 30),
        days => format!("{}y", days / 365),
    }
}

fn format_relative_due_date(dt: NaiveDate, compact: bool) -> (String, i64) {
    let today = Local::now().date_naive();
    let diff = (dt - today).num_days();
    let dstr = format_days(diff, compact);
    let v = if compact {
        dstr
    } else if diff < 0 {
        format!("{dstr} overdue")
    } else if diff == 0 {
        dstr
    } else {
        format!("in {dstr}")
    };
    (v, diff)
}

pub fn format_relative_date(dt: NaiveDate, compact: bool) -> (String, i64) {
    let today = Local::now().date_naive();
    let diff = (dt - today).num_days();
    let dstr = format_days(diff, false);
    if compact {
        return (dstr, diff);
    }

    let v = match diff.cmp(&0) {
        Ordering::Less => format!("{dstr} ago"),
        Ordering::Equal => dstr,
        Ordering::Greater => format!("in {dstr}"),
    };
    (v, diff)
}

pub fn field_list(c: &Conf) -> Vec<&str> {
    match c.fmt {
        Format::Full => {
            if c.fields.is_empty() {
                vec!["done", "pri", "created", "finished", "due"]
            } else {
                let fields: Vec<&str> = c.fields.iter().map(|s| s.as_str()).collect();
                fields
            }
        }
        Format::Short => vec!["done", "pri"],
    }
}

fn header_len(c: &Conf, flist: &[String], widths: &[usize]) -> usize {
    let (other, _) = calc_width(c, flist, widths);
    other + "Subject".width() + 1
}

pub fn print_header(stdout: &mut StandardStream, c: &Conf, flist: &[String], widths: &[usize]) -> io::Result<()> {
    if c.hide_headers {
        return Ok(());
    }
    print_header_line(stdout, c, flist, widths)?;
    writeln!(stdout, "{}", "-".repeat(header_len(c, flist, widths)))
}

pub fn print_body_single(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    idx: usize,
    id: usize,
    c: &Conf,
    flist: &[String],
    widths: &[usize],
) -> io::Result<()> {
    print_line(stdout, &tasks[idx], id, c, flist, widths)?;
    Ok(())
}

fn values_of_field(task: &todotxt::Task, field: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut values = match field {
        "prj" | "project" => task.projects.clone(),
        "ctx" | "context" => task.contexts.clone(),
        "hash" | "hashtag" => task.hashtags.clone(),
        tag => {
            if let Some(s) = task.tags.get(tag) {
                vec![s.clone()]
            } else {
                vec![String::new()]
            }
        }
    };
    if values.is_empty() {
        values.push(String::new());
    }
    for v in values {
        if items.contains(&v) {
            continue;
        }
        items.push(v);
    }
    items.sort();
    items
}
fn select_uniq_values(tasks: &todo::TaskSlice, selected: &todo::IDSlice, only_selected: bool, c: &Conf) -> Vec<String> {
    let group = match &c.group {
        None => return vec![String::new()],
        Some(g) => g.clone(),
    };
    let mut v = Vec::new();
    if only_selected {
        for id in selected.iter() {
            let print = *id < tasks.len();
            if !print {
                continue;
            }
            let task_values = values_of_field(&tasks[*id], &group);
            for tv in task_values {
                if v.contains(&tv) {
                    continue;
                }
                v.push(tv);
            }
        }
    } else {
        for (i, t) in tasks.iter().enumerate() {
            if i < selected.len() {
                break;
            }
            let task_values = values_of_field(t, &group);
            for tv in task_values {
                if v.contains(&tv) {
                    continue;
                }
                v.push(tv);
            }
        }
    }
    if v.is_empty() || (v.len() == 1 && v[0].is_empty()) {
        return vec![String::new()];
    }
    v.sort();
    if v.len() > 1 && v[0].is_empty() {
        v.remove(0);
        v.push(String::new());
    }
    v
}

fn print_body_selected(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    selected: &todo::IDSlice,
    updated: &todo::ChangedSlice,
    c: &Conf,
    flist: &[String],
    widths: &[usize],
) -> io::Result<()> {
    let group = match &c.group {
        None => String::new(),
        Some(g) => g.clone(),
    };
    let groups = select_uniq_values(tasks, selected, true, c);
    let print_groups = groups.len() > 1;
    for g in groups {
        if print_groups {
            let group_name = if g.is_empty() { "[Empty]" } else { g.as_str() };
            match group.as_str() {
                "prj" | "project" => writeln!(stdout, "+{group_name}")?,
                "ctx" | "context" => writeln!(stdout, "@{group_name}")?,
                "hash" | "hashtag" => writeln!(stdout, "#{group_name}")?,
                _ => writeln!(stdout, "{group_name}")?,
            };
        }
        for (i, id) in selected.iter().enumerate() {
            let print = updated.is_empty() || (i < updated.len() && updated[i]);
            let print = print && (*id < tasks.len());
            if print {
                if print_groups {
                    let vals = values_of_field(&tasks[*id], &group);
                    if vals.contains(&g) {
                        print_line(stdout, &tasks[*id], *id + 1, c, flist, widths)?;
                    }
                } else {
                    print_line(stdout, &tasks[*id], *id + 1, c, flist, widths)?;
                }
            }
        }
    }
    Ok(())
}

fn print_body_all(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    selected: &todo::IDSlice,
    updated: &todo::ChangedSlice,
    c: &Conf,
    flist: &[String],
    widths: &[usize],
) -> io::Result<()> {
    let group = match &c.group {
        None => String::new(),
        Some(g) => g.clone(),
    };
    let groups = select_uniq_values(tasks, selected, false, c);
    let print_groups = groups.len() > 1;
    for g in groups {
        if print_groups {
            let group_name = if g.is_empty() { "[Empty]" } else { g.as_str() };
            match group.as_str() {
                "prj" | "project" => writeln!(stdout, "+{group_name}")?,
                "ctx" | "context" => writeln!(stdout, "@{group_name}")?,
                "hash" | "hashtag" => writeln!(stdout, "#{group_name}")?,
                _ => writeln!(stdout, "{group_name}")?,
            };
        }
        for (i, t) in tasks.iter().enumerate() {
            let (id, print) = if i < selected.len() { (selected[i], updated[i]) } else { (0, false) };
            if print {
                if print_groups {
                    let vals = values_of_field(t, &group);
                    if vals.contains(&g) {
                        print_line(stdout, t, id + 1, c, flist, widths)?;
                    }
                } else {
                    print_line(stdout, t, id + 1, c, flist, widths)?;
                }
            }
        }
    }
    Ok(())
}

pub fn print_footer(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    selected: &todo::IDSlice,
    updated: &todo::ChangedSlice,
    c: &Conf,
    flist: &[String],
    widths: &[usize],
) -> io::Result<()> {
    if c.hide_headers {
        return Ok(());
    }
    writeln!(stdout, "{}", "-".repeat(header_len(c, flist, widths)))?;

    if updated.is_empty() && !selected.is_empty() {
        writeln!(stdout, "{} todos (of {} total)", selected.len(), c.max)
    } else if tasks.len() != updated.len() {
        writeln!(stdout, "{} todos (of {} total)", updated.len(), c.max)
    } else {
        writeln!(stdout, "{} todos", updated.len())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn print_todos(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    select: &todo::IDSlice,
    updated: &todo::ChangedSlice,
    c: &Conf,
    fields: &[String],
    widths: &[usize],
    all: bool,
) -> io::Result<()> {
    if tasks.is_empty() || select.is_empty() {
        return Ok(());
    }

    if all {
        print_body_all(stdout, tasks, select, updated, c, fields, widths)
    } else {
        print_body_selected(stdout, tasks, select, updated, c, fields, widths)
    }
}

fn field_width_cached(field: &str, fields: &[String], cached: &[usize]) -> usize {
    for (f, w) in fields.iter().zip(cached.iter()) {
        if default_caseless_match_str(f, field) {
            return *w;
        }
    }
    0
}

fn is_hashtag(s: &str) -> bool {
    !s.contains([' ', '\t', '\n', '\r']) && s.len() > 1 && s.starts_with('#')
}

fn is_project(s: &str) -> bool {
    !s.contains([' ', '\t', '\n', '\r']) && s.len() > 1 && s.starts_with('+')
}

fn is_context(s: &str) -> bool {
    !s.contains([' ', '\t', '\n', '\r']) && s.len() > 1 && s.starts_with('@')
}

fn is_tag(s: &str) -> bool {
    if s.contains([' ', '\t', '\n', '\r']) {
        return false;
    }
    match s.find(':') {
        None => false,
        Some(pos) => pos != 0 && pos < s.len() - 1,
    }
}

fn is_syntax_word(s: &str) -> bool {
    is_hashtag(s) || is_context(s) || is_project(s) || is_tag(s)
}

fn parse_subj(subj: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    if !subj.contains([':', '@', '+', '#']) {
        parts.push(subj);
        return parts;
    }
    let mut curr = subj;
    let mut part = subj;
    let mut part_len = 0;
    loop {
        match curr.find([' ', '\n', '\r']) {
            Some(pos) => {
                if is_syntax_word(&curr[..pos]) {
                    if part_len == 0 {
                        parts.push(&curr[..pos]);
                        part = &curr[pos..]; // include space
                        curr = &part[1..]; // skip space
                        part_len = 1;
                    } else {
                        parts.push(&part[..part_len]);
                        parts.push(&curr[..pos]);
                        part = &curr[pos..]; // include space
                        curr = &part[1..]; // skip space
                        part_len = 1;
                    }
                } else {
                    part_len += pos + 1; // word length + space
                    curr = &curr[pos + 1..];
                }
            }
            None => {
                if is_syntax_word(curr) {
                    parts.push(&part[..part_len]);
                    parts.push(curr);
                } else {
                    parts.push(part);
                }
                break;
            }
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subj_test() {
        struct Test {
            inp: &'static str,
            res: Vec<&'static str>,
        }
        let tests: Vec<Test> = vec![
            Test { inp: "short", res: vec!["short"] },
            Test { inp: "      short   abdcd    ", res: vec!["      short   abdcd    "] },
            Test { inp: "no any syntax", res: vec!["no any syntax"] },
            Test {
                inp: "# @ + : no an#y syntax# proj+ pro+j cont@ con@t :tag sometag:",
                res: vec!["# @ + : no an#y syntax# proj+ pro+j cont@ con@t :tag sometag:"],
            },
            Test { inp: "some @project with in@valid", res: vec!["some ", "@project", " with in@valid"] },
            Test { inp: "some +context with in+valid +", res: vec!["some ", "+context", " with in+valid +"] },
            Test { inp: "a some # #hashtag with inv#alid #", res: vec!["a some # ", "#hashtag", " with inv#alid #"] },
            Test {
                inp: "here are some: good:tags and :bad tags",
                res: vec!["here are some: ", "good:tags", " and :bad tags"],
            },
            Test { inp: "short rec:wrd", res: vec!["short ", "rec:wrd"] },
            Test { inp: "short @proj", res: vec!["short ", "@proj"] },
            Test { inp: "short #hash", res: vec!["short ", "#hash"] },
            Test { inp: "short +ctx", res: vec!["short ", "+ctx"] },
            Test { inp: "short +ctx\n", res: vec!["short ", "+ctx", "\n"] },
            Test {
                inp: "@NVIDIA free day due:2023-03-02\n",
                res: vec!["@NVIDIA", " free day ", "due:2023-03-02", "\n"],
            },
        ];

        for (idx, test) in tests.iter().enumerate() {
            let parts = parse_subj(test.inp);
            assert_eq!(parts.len(), test.res.len(), "{}. {:?} --- {:?}", idx, test.res, parts);
            for (i, p) in parts.iter().enumerate() {
                assert_eq!(p, &test.res[i], "{}. '{}' != '{}'", idx, p, test.res[i])
            }
        }
    }
}
