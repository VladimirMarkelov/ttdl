use std::io::Write;
use std::process::Command;

use caseless::default_caseless_match_str;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use textwrap;
use todo_lib::timer;
use todo_lib::todo;
use todo_txt;
use json;

const REL_WIDTH_DUE: usize = 12;
const REL_WIDTH_DATE: usize = 8; // FINISHED - the shortest
const REL_COMPACT_WIDTH: usize = 3;
const SPENT_WIDTH: usize = 6;
const JSON_DESC: &'static str = "description";
const JSON_SPEC: &'static str = "specialTags";
const PLUG_PREFIX: &'static str = "ttdl-";

lazy_static! {
    static ref FIELDS: [&'static str; 7] = ["done", "pri", "created", "finished", "due", "thr", "spent"];
}

#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    Full,
    Short,
}

#[derive(Debug, Clone, PartialEq)]
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

    pub important_limit: u8,
    pub soon_days: u8,
    pub old_period: Option<todo_txt::task::Recurrence>,
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
    spc.set_fg(Some(Color::White));
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

            important_limit: todo::NO_PRIORITY,
            soon_days: 0u8,
            old_period: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TermColorType {
    None,
    Auto,
    Ansi,
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
    pub header: bool,
    pub footer: bool,
    pub max: usize,
    pub colors: Colors,
    pub atty: bool,
    pub shell: Vec<String>,
    pub script_ext: String,
    pub script_prefix: String,
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
            header: true,
            footer: true,
            max: 0,
            compact: false,
            colors: Default::default(),
            atty: true,
            script_ext: String::new(),
            script_prefix: String::new(),
            shell,
        }
    }
}

impl Conf {
    fn is_human(&self, s: &str) -> bool {
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
}

fn number_of_digits(val: usize) -> usize {
    match val {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        1000..=9999 => 4,
        _ => 5,
    }
}

fn rel_due_date_width(field: &str, c: &Conf) -> usize {
    if c.is_human(field) {
        if c.compact {
            REL_COMPACT_WIDTH
        } else {
            REL_WIDTH_DUE
        }
    } else {
        "2018-12-12".len()
    }
}

fn rel_date_width(field: &str, c: &Conf) -> usize {
    if c.is_human(field) {
        REL_WIDTH_DATE
    } else {
        "2018-12-12".len()
    }
}

fn field_width(field: &str, c: &Conf) -> usize {
    let id_width = number_of_digits(c.max);
    let dt_width = "2018-12-12".len();
    let rel_due_dt_width = rel_due_date_width(field, c);
    let rel_dt_width = rel_date_width(field, c);

    match field {
        "id" => id_width,
        "done" | "pri" => 1,
        "created" | "finished" => {
            if c.is_human(field) {
                rel_dt_width
            } else {
                dt_width
            }
        }
        "due" | "thr" => {
            if c.is_human(field) {
                rel_due_dt_width
            } else {
                dt_width
            }
        }
        "spent" => SPENT_WIDTH,
        _ => 0,
    }
}

fn calc_width(c: &Conf, fields: &[&str]) -> (usize, usize) {
    let mut before: usize = field_width("id", c) + 1;

    for f in FIELDS.iter() {
        let mut found = false;
        for pf in fields.iter() {
            if default_caseless_match_str(pf, f) {
                found = true;
                break;
            }
        }

        if found {
            before += field_width(*f, c) + 1;
        }
    }

    (before, c.width as usize - before)
}

fn print_header_line(c: &Conf, fields: &[&str]) {
    print!("{:>wid$} ", "#", wid = field_width("id", c));

    for f in FIELDS.iter() {
        let mut found = false;
        for pf in fields.iter() {
            if default_caseless_match_str(pf, f) {
                found = true;
                break;
            }
        }

        if found {
            let width = field_width(*f, c);
            match *f {
                "done" => print!("D "),
                "pri" => print!("P "),
                "created" => print!("{:wid$} ", "Created", wid = width),
                "finished" => print!("{:wid$} ", "Finished", wid = width),
                "due" => print!("{:wid$} ", "Due", wid = width),
                "thr" => {
                    if c.is_human(*f) && c.compact {
                        print!("{:wid$} ", "Thr", wid = width);
                    } else {
                        print!("{:wid$} ", "Threshold", wid = width);
                    }
                }
                "spent" => print!("{:wid$} ", "Spent", wid = SPENT_WIDTH),
                _ => {}
            }
        }
    }

    println!("Subject");
}

fn color_for_priority(task: &todo_txt::task::Extended, c: &Conf) -> ColorSpec {
    if task.finished {
        return c.colors.done.clone();
    }

    if task.priority == 0 || (task.priority < c.colors.important_limit && c.colors.important_limit != todo::NO_PRIORITY)
    {
        if task.priority == 0 {
            return c.colors.top.clone();
        } else {
            return c.colors.important.clone();
        }
    }

    default_color()
}

fn color_for_creation_date(task: &todo_txt::task::Extended, c: &Conf) -> ColorSpec {
    let spc = default_color();
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
        let today = chrono::Local::now().date().naive_local();
        let mcreate = rec + *cd;
        if mcreate < today {
            return c.colors.old.clone();
        }
    }

    spc
}

fn color_for_due_date(task: &todo_txt::task::Extended, days: i64, c: &Conf) -> ColorSpec {
    let spc = default_color();
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

fn color_for_threshold_date(task: &todo_txt::task::Extended, days: i64, c: &Conf) -> ColorSpec {
    let spc = default_color();
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

fn print_with_color(stdout: &mut StandardStream, msg: &str, color: &ColorSpec) {
    if let Err(e) = stdout.set_color(color) {
        eprintln!("Failed to set color: {:?}", e);
        return;
    }
    if let Err(e) = write!(stdout, "{}", msg) {
        eprintln!("Failed to print to stdout: {:?}", e);
    }
}

fn done_str(task: &todo_txt::task::Extended) -> String {
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

fn priority_str(task: &todo_txt::task::Extended) -> String {
    if task.priority < todo::NO_PRIORITY {
        format!("{} ", (b'A' + task.priority) as char)
    } else {
        "  ".to_string()
    }
}

pub fn duration_str(d: chrono::Duration) -> String {
    let s = d.num_seconds();
    if s <= 0 {
        return String::new();
    }

    if s < 60 {
        format!("{}s", s)
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

fn print_line(stdout: &mut StandardStream, task: &todo_txt::task::Extended, id: usize, c: &Conf, fields: &[&str]) {
    let id_width = field_width("id", c);
    let fg = if task.finished {
        c.colors.done.clone()
    } else {
        default_color()
    };

    print_with_color(stdout, &format!("{:>wid$} ", id, wid = id_width), &fg);

    for f in FIELDS.iter() {
        let mut found = false;
        for pf in fields.iter() {
            if default_caseless_match_str(pf, f) {
                found = true;
                break;
            }
        }

        if found {
            let width = field_width(*f, c);
            match *f {
                "done" => {
                    print_with_color(stdout, &done_str(task), &fg);
                }
                "pri" => {
                    print_with_color(stdout, &priority_str(task), &color_for_priority(task, c));
                }
                "created" => {
                    let dfg = color_for_creation_date(task, c);
                    let st = if let Some(d) = task.create_date.as_ref() {
                        if c.is_human(*f) {
                            let (s, _) = format_relative_date(*d, c.compact);
                            format!("{:wid$} ", s, wid = width)
                        } else {
                            format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = width)
                        }
                    } else {
                        format!("{:wid$} ", " ", wid = width)
                    };
                    print_with_color(stdout, &st, &dfg);
                }
                "finished" => {
                    let st = if let Some(d) = task.finish_date.as_ref() {
                        if c.is_human(*f) {
                            let (s, _) = format_relative_date(*d, c.compact);
                            format!("{:wid$} ", s, wid = width)
                        } else {
                            format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = width)
                        }
                    } else {
                        format!("{:wid$} ", " ", wid = width)
                    };
                    print_with_color(stdout, &st, &fg);
                }
                "due" => {
                    if let Some(d) = task.due_date.as_ref() {
                        let (s, days) = format_relative_due_date(*d, c.compact);
                        let dfg = color_for_due_date(task, days, c);
                        let st = if c.is_human(*f) {
                            s.to_string()
                        } else {
                            format!("{}", (*d).format("%Y-%m-%d"))
                        };
                        print_with_color(stdout, &format!("{:wid$} ", &st, wid = width), &dfg);
                    } else {
                        print_with_color(stdout, &format!("{:wid$} ", " ", wid = width), &fg);
                    };
                }
                "thr" => {
                    if let Some(d) = task.threshold_date.as_ref() {
                        let (s, days) = format_relative_due_date(*d, c.compact);
                        let dfg = color_for_threshold_date(task, days, c);
                        let st = if c.is_human(*f) {
                            s.to_string()
                        } else {
                            format!("{}", (*d).format("%Y-%m-%d"))
                        };
                        print_with_color(stdout, &format!("{:wid$} ", &st, wid = width), &dfg);
                    } else {
                        print_with_color(stdout, &format!("{:wid$} ", " ", wid = width), &fg);
                    };
                }
                "spent" => {
                    print_with_color(
                        stdout,
                        &format!("{:wid$} ", &duration_str(timer::spent_time(&task)), wid = SPENT_WIDTH),
                        &fg,
                    );
                }
                _ => {}
            }
        }
    }

    let mut subj = task.subject.clone();
    if let Some(r) = task.recurrence.as_ref() {
        subj.push_str(&format!(" rec:{}", *r));
    }
    if let Some(s) = external_reconstruct(task, c) {
        subj = s;
    }
    if c.width != 0 && c.long != LongLine::Simple {
        let (skip, subj_w) = calc_width(c, fields);
        let lines = textwrap::wrap(&subj, subj_w);
        if c.long == LongLine::Cut || lines.len() == 1 {
            print_with_color(stdout, &format!("{}\n", lines[0]), &fg);
        } else {
            for (i, line) in lines.iter().enumerate() {
                if i != 0 {
                    print_with_color(stdout, &format!("{:width$}", " ", width = skip), &fg);
                }
                print_with_color(stdout, &format!("{}\n", &line), &fg);
            }
        }
    } else {
        print_with_color(stdout, &format!("{}\n", &subj), &fg);
    }
    if let Err(e) = stdout.set_color(&default_color()) {
        eprintln!("Failed to set color: {:?}", e);
    }
}

fn external_reconstruct(task: &todo_txt::task::Extended, c: &Conf) -> Option<String> {
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
    let orig_subj = task.subject.clone();
    for cmd in ext_cmds {
        if !command_in_json(&arg, &cmd) {
            continue;
        }
        let bin_name = format!("{}{}", PLUG_PREFIX, cmd.trim_start_matches('!'));
        let args = json::stringify(arg);
        let output = exec_plugin(c, &bin_name, &args);
        match output {
            Err(e) => {
                eprintln!("Failed to execute plugin '{}': {}", bin_name, e);
                return None;
            },
            Ok(s) => {
                match json::parse(&s) {
                    Ok(j) => arg = j,
                    Err(e) => {
                        eprintln!("Failed to parse output of plugin {}: {}\nOutput: {}", bin_name, e, s);
                        return None;
                    },
                }
            }
        }
    }

    // TODO: last arg -> desc + arg
    let mut res = if let Some(s) = arg[JSON_DESC].as_str() {
        s.to_string()
    } else {
        orig_subj
    };
    let tags = &arg[JSON_SPEC];
    if !tags.is_array() || tags.is_empty() {
        return Some(res);
    }
    for m in tags.members() {
        if !m.is_object() {
            continue;
        }
        for e in m.entries() {
            let (key, val) = e;
            if let Some(st) = val.as_str() {
                res += &(format!(" {}:{}", key, st));
            }
        }
    }

    Some(res)
}

fn exec_plugin(c: &Conf, plugin: &str, args: &str) -> Result<String, String> {
    let mut plugin_bin = plugin.to_string();
    if !c.script_prefix.is_empty() {
        plugin_bin = format!("{}{}", c.script_prefix, plugin_bin);
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
    cmd.arg(&plugin_bin);
    cmd.arg(&format!("'{}'", args));
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => return Err(e.to_string()),
    };

    if !out.status.success() {
        if let Ok(s) = String::from_utf8(out.stderr) {
            return Err(s.trim_end().to_string());
        }
        return Err(format!("failed to execute plugin '{}'", plugin));
    }

    if let Ok(s) = String::from_utf8(out.stdout) {
        Ok(s.trim_end().to_string())
    } else {
        Err(format!("Non-UTF-8 Output from '{}'", plugin))
    }
}

// {"description": "This is an example", "specialTags":[{"tag1": "value"},]}
fn build_ext_arg(task: &todo_txt::task::Extended) -> json::JsonValue {
    let mut jarr = json::JsonValue::new_array();
    for (key, val) in task.tags.iter() {
        let o = json::object!{ key => val.to_string() };
        let _ = jarr.push(o);
    }
    json::object!{
        JSON_DESC => task.subject.clone(),
        JSON_SPEC => jarr,
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
        days @ 1..=6 => format!("{}d", days),
        days @ 7..=29 => format!("{}w", days / 7),
        days @ 30..=364 => format!("{}m", days / 30),
        days => format!("{}y", days / 365),
    }
}

fn format_relative_due_date(dt: chrono::NaiveDate, compact: bool) -> (String, i64) {
    let today = chrono::Local::now().date().naive_local();
    let diff = (dt - today).num_days();
    let dstr = format_days(diff, compact);
    let v = if compact {
        dstr
    } else if diff < 0 {
        format!("{} overdue", dstr)
    } else if diff == 0 {
        dstr
    } else {
        format!("in {}", dstr)
    };
    (v, diff)
}

fn format_relative_date(dt: chrono::NaiveDate, compact: bool) -> (String, i64) {
    let today = chrono::Local::now().date().naive_local();
    let diff = (dt - today).num_days();
    let dstr = format_days(diff, false);
    if compact {
        return (dstr, diff);
    }

    let v = if diff < 0 {
        format!("{} ago", dstr)
    } else if diff == 0 {
        dstr
    } else {
        format!("in {}", dstr)
    };
    (v, diff)
}

fn field_list(c: &Conf) -> Vec<&str> {
    match c.fmt {
        Format::Full => {
            if c.fields.is_empty() {
                vec!["done", "pri", "created", "finished", "due", "thr", "spent"]
            } else {
                let fields: Vec<&str> = c.fields.iter().map(|s| s.as_str()).collect();
                fields
            }
        }
        Format::Short => vec!["done", "pri"],
    }
}

fn header_len(c: &Conf, flist: &[&str]) -> usize {
    let (other, _) = calc_width(c, flist);
    other + "Subject".len() + 1
}

pub fn print_header(c: &Conf) {
    let flist = field_list(c);
    print_header_line(c, &flist);
    println!("{}", "-".repeat(header_len(c, &flist)));
}

fn print_body_selected(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    selected: &todo::IDSlice,
    updated: &todo::ChangedSlice,
    c: &Conf,
) {
    let flist = field_list(c);
    for (i, id) in selected.iter().enumerate() {
        let print = updated.is_empty() || (i < updated.len() && updated[i]);
        let print = print && (*id < tasks.len());
        if print {
            print_line(stdout, &tasks[*id], *id + 1, c, &flist);
        }
    }
}

fn print_body_all(
    stdout: &mut StandardStream,
    tasks: &todo::TaskSlice,
    selected: &todo::IDSlice,
    updated: &todo::ChangedSlice,
    c: &Conf,
) {
    let flist = field_list(c);
    for (i, t) in tasks.iter().enumerate() {
        let (id, print) = if i < selected.len() {
            (selected[i], updated[i])
        } else {
            (0, false)
        };
        if print {
            print_line(stdout, t, id + 1, c, &flist);
        }
    }
}

pub fn print_footer(tasks: &todo::TaskSlice, selected: &todo::IDSlice, updated: &todo::ChangedSlice, c: &Conf) {
    let flist = field_list(c);
    println!("{}", "-".repeat(header_len(c, &flist)));

    if updated.is_empty() && !selected.is_empty() {
        println!("{} todos (of {} total)", selected.len(), c.max);
    } else if tasks.len() != updated.len() {
        println!("{} todos (of {} total)", updated.len(), c.max);
    } else {
        println!("{} todos", updated.len());
    }
}

pub fn print_todos(tasks: &todo::TaskSlice, select: &todo::IDSlice, updated: &todo::ChangedSlice, c: &Conf, all: bool) {
    let mut stdout = match c.color_term {
        TermColorType::Ansi => StandardStream::stdout(ColorChoice::AlwaysAnsi),
        TermColorType::Auto => StandardStream::stdout(ColorChoice::Always),
        TermColorType::None => StandardStream::stdout(ColorChoice::Never),
    };

    if tasks.is_empty() || select.is_empty() {
        return;
    }

    if all {
        print_body_all(&mut stdout, tasks, select, updated, c);
    } else {
        print_body_selected(&mut stdout, tasks, select, updated, c);
    }
}
