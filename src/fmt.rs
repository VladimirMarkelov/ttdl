use caseless::default_caseless_match_str;
use textwrap;
use todo_lib::todo;
use todo_txt;

const SEPARATOR: &str = "----------------------------------------------------";
const REL_WIDTH: usize = 12;
const REL_COMPACT_WIDTH: usize = 3;

lazy_static! {
    static ref FIELDS: [&'static str; 6] = ["done", "pri", "created", "finished", "due", "thr"];
}

#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    Full,
    Short,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LongLine {
    Simple,
    WordWrap,
    Cut,
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub top: Option<crossterm::Color>,
    pub important: Option<crossterm::Color>,
    pub overdue: Option<crossterm::Color>,
    pub threshold: Option<crossterm::Color>,
    pub today: Option<crossterm::Color>,
    pub soon: Option<crossterm::Color>,
    pub done: Option<crossterm::Color>,

    pub important_limit: u8,
    pub soon_days: u8,
}
impl Default for Colors {
    fn default() -> Colors {
        Colors {
            top: Some(crossterm::Color::Red),
            important: None,
            overdue: Some(crossterm::Color::Red),
            threshold: Some(crossterm::Color::DarkRed),
            today: Some(crossterm::Color::Yellow),
            soon: None,
            done: Some(crossterm::Color::Grey),

            important_limit: todo::NO_PRIORITY,
            soon_days: 0u8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Conf {
    pub fmt: Format,
    pub width: u16,
    pub long: LongLine,
    pub fields: Vec<String>,
    pub human: bool,
    pub compact: bool,
    pub colored: bool,
    pub header: bool,
    pub footer: bool,
    pub max: usize,
    pub colors: Colors,
    pub atty: bool,
}

impl Default for Conf {
    fn default() -> Conf {
        Conf {
            fmt: Format::Full,
            width: 0,
            long: LongLine::Simple,
            fields: Vec::new(),
            human: false,
            colored: true,
            header: true,
            footer: true,
            max: 0,
            compact: false,
            colors: Default::default(),
            atty: true,
        }
    }
}

fn number_of_digits(val: usize) -> usize {
    match val {
        0...9 => 1,
        10...99 => 2,
        100...999 => 3,
        1000...9999 => 4,
        _ => 5,
    }
}

fn rel_date_width(c: &Conf) -> usize {
    if c.human {
        if c.compact {
            REL_COMPACT_WIDTH
        } else {
            REL_WIDTH
        }
    } else {
        "2018-12-12".len()
    }
}

fn calc_width(c: &Conf, fields: &[&str]) -> (usize, usize) {
    let id_width = number_of_digits(c.max);
    let dt_width = "2018-12-12".len();
    let rel_dt_width = rel_date_width(c);
    let mut before: usize = id_width + 1;

    for f in FIELDS.iter() {
        let mut found = false;
        for pf in fields.iter() {
            if default_caseless_match_str(pf, f) {
                found = true;
                break;
            }
        }

        if found {
            match *f {
                "done" | "pri" => before += 2,
                "created" | "finished" => before += dt_width + 1,
                "due" | "thr" => before += rel_dt_width + 1,
                _ => {}
            }
        }
    }

    (before, c.width as usize - before)
}

fn print_header_line(c: &Conf, fields: &[&str]) {
    let id_width = number_of_digits(c.max);
    let dt_width = "2018-12-12".len();
    let rel_dt_width = rel_date_width(c);
    print!("{:>wid$} ", "#", wid = id_width);

    for f in FIELDS.iter() {
        let mut found = false;
        for pf in fields.iter() {
            if default_caseless_match_str(pf, f) {
                found = true;
                break;
            }
        }

        if found {
            match *f {
                "done" => print!("D "),
                "pri" => print!("P "),
                "created" => print!("{:wid$} ", "Created", wid = dt_width),
                "finished" => print!("{:wid$} ", "Finished", wid = dt_width),
                "due" => print!("{:wid$} ", "Due", wid = rel_dt_width),
                "thr" => {
                    if c.human && c.compact {
                        print!("{:wid$} ", "Thr", wid = rel_dt_width);
                    } else {
                        print!("{:wid$} ", "Threshold", wid = rel_dt_width);
                    }
                }
                _ => {}
            }
        }
    }

    println!("Subject");
}

fn color_for_priority(task: &todo_txt::task::Extended, c: &Conf) -> Option<crossterm::Color> {
    if !c.colored {
        return None;
    }
    if task.finished {
        return c.colors.done;
    }

    if task.priority == 0 || (task.priority < c.colors.important_limit && c.colors.important_limit != todo::NO_PRIORITY)
    {
        if task.priority == 0 {
            c.colors.top
        } else {
            c.colors.important
        }
    } else {
        None
    }
}

fn color_for_due_date(task: &todo_txt::task::Extended, days: i64, c: &Conf) -> Option<crossterm::Color> {
    if !c.colored {
        return None;
    }
    if task.finished {
        return c.colors.done;
    }
    task.due_date?;

    if days > i64::from(c.colors.soon_days) {
        return None;
    }

    if days < 0 {
        return c.colors.overdue;
    }

    if days == 0 {
        return if c.colors.today.is_some() {
            c.colors.today
        } else if c.colors.soon_days != 0 {
            c.colors.soon
        } else {
            None
        };
    }

    if c.colors.soon_days > 0 && days <= i64::from(c.colors.soon_days) {
        return c.colors.soon;
    }

    None
}

fn color_for_threshold_date(task: &todo_txt::task::Extended, days: i64, c: &Conf) -> Option<crossterm::Color> {
    if !c.colored {
        return None;
    }
    if task.finished {
        return c.colors.done;
    }
    task.threshold_date?;

    if days < 0 {
        return c.colors.threshold;
    }

    None
}

fn print_with_color(s: &str, fg: Option<crossterm::Color>) {
    match fg {
        None => print!("{}", crossterm::style(s)),
        Some(c) => {
            print!("{}", crossterm::style(s).with(c));
        }
    }
}

fn print_line(task: &todo_txt::task::Extended, id: usize, c: &Conf, fields: &[&str]) {
    let id_width = number_of_digits(c.max);
    let dt_width = "2018-12-12".len();
    let rel_dt_width = rel_date_width(c);
    let fg = if task.finished && c.colored {
        c.colors.done
    } else {
        None
    };
    print_with_color(&format!("{:>wid$} ", id, wid = id_width), fg);

    for f in FIELDS.iter() {
        let mut found = false;
        for pf in fields.iter() {
            if default_caseless_match_str(pf, f) {
                found = true;
                break;
            }
        }

        if found {
            match *f {
                "done" => {
                    let st = if task.finished {
                        "x "
                    } else if task.recurrence.is_some() {
                        "R "
                    } else {
                        "  "
                    };
                    print_with_color(st, fg);
                }
                "pri" => {
                    let st = if task.priority < todo::NO_PRIORITY {
                        format!("{} ", (b'A' + task.priority) as char)
                    } else {
                        "  ".to_string()
                    };
                    let fg = color_for_priority(task, c);
                    print_with_color(&st, fg);
                }
                "created" => {
                    let st = if let Some(d) = task.create_date.as_ref() {
                        format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = dt_width)
                    } else {
                        format!("{:wid$} ", " ", wid = dt_width)
                    };
                    print_with_color(&st, fg);
                }
                "finished" => {
                    let st = if let Some(d) = task.finish_date.as_ref() {
                        format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = dt_width)
                    } else {
                        format!("{:wid$} ", " ", wid = dt_width)
                    };
                    print_with_color(&st, fg);
                }
                "due" => {
                    if let Some(d) = task.due_date.as_ref() {
                        let (s, days) = format_relative_date(*d, c.compact);
                        let dfg = color_for_due_date(task, days, c);
                        let st = if c.human {
                            s.to_string()
                        } else {
                            format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = rel_dt_width)
                        };
                        print_with_color(&st, dfg);
                    } else {
                        print_with_color(&format!("{:wid$} ", " ", wid = rel_dt_width), fg);
                    };
                }
                "thr" => {
                    if let Some(d) = task.threshold_date.as_ref() {
                        let (s, days) = format_relative_date(*d, c.compact);
                        let dfg = color_for_threshold_date(task, days, c);
                        let st = if c.human {
                            s.to_string()
                        } else {
                            format!("{:wid$} ", (*d).format("%Y-%m-%d"), wid = rel_dt_width)
                        };
                        print_with_color(&st, dfg);
                    } else {
                        print_with_color(&format!("{:wid$} ", " ", wid = rel_dt_width), fg);
                    };
                }
                _ => {}
            }
        }
    }

    let mut subj = task.subject.clone();
    if let Some(r) = task.recurrence.as_ref() {
        subj.push_str(&format!(" rec:{}", *r));
    }
    if c.width != 0 && c.long != LongLine::Simple {
        let (skip, subj_w) = calc_width(c, fields);
        let lines = textwrap::wrap(&subj, subj_w);
        if c.long == LongLine::Cut || lines.len() == 1 {
            print_with_color(&lines[0], fg);
            println!();
        } else {
            for (i, line) in lines.iter().enumerate() {
                if i != 0 {
                    print!("{:width$}", " ", width = skip);
                }
                print_with_color(&line, fg);
                println!();
            }
        }
    } else {
        print_with_color(&subj, fg);
        println!();
    }
}

fn print_full_header(c: &Conf) {
    print_header_line(c, &["done", "pri", "created", "finished", "due", "thr"]);
}

fn print_short_header(c: &Conf) {
    print_header_line(c, &["done", "pri"]);
}

fn print_custom_header(c: &Conf) {
    let fields: Vec<&str> = c.fields.iter().map(|s| s.as_str()).collect();
    print_header_line(c, &fields);
}

fn print_full_info(task: &todo_txt::task::Extended, id: usize, c: &Conf) {
    print_line(task, id, c, &["done", "pri", "created", "finished", "due", "thr"]);
}

fn print_short_info(task: &todo_txt::task::Extended, id: usize, c: &Conf) {
    print_line(task, id, c, &["done", "pri"]);
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
        days @ 1...6 => format!("{}d", days),
        days @ 7...29 => format!("{}w", days / 7),
        days @ 30...364 => format!("{}m", days / 30),
        days => format!("{}y", days / 365),
    }
}

fn format_relative_date(dt: chrono::NaiveDate, compact: bool) -> (String, i64) {
    let today = chrono::Local::now().date().naive_local();
    let diff = (dt - today).num_days();
    let dstr = format_days(diff, compact);
    let mut width = REL_WIDTH;
    let v = if compact {
        width = REL_COMPACT_WIDTH;
        dstr
    } else if diff < 0 {
        format!("{} overdue", dstr)
    } else if diff == 0 {
        dstr
    } else {
        format!("in {}", dstr)
    };
    (format!("{:wid$} ", v, wid = width), diff)
}

fn print_custom_info(task: &todo_txt::task::Extended, id: usize, c: &Conf) {
    let fields: Vec<&str> = c.fields.iter().map(|s| s.as_str()).collect();
    print_line(task, id, c, &fields);
}

pub fn print_header(c: &Conf) {
    match c.fmt {
        Format::Full => print_full_header(c),
        Format::Short => print_short_header(c),
        Format::Custom => print_custom_header(c),
    };
    let uwidth = c.width as usize;
    if c.atty && SEPARATOR.len() > uwidth {
        let slice = &SEPARATOR[..uwidth];
        println!("{}", slice);
    } else {
        println!("{}", SEPARATOR);
    }
}

fn print_body_selected(tasks: &todo::TaskSlice, selected: &todo::IDSlice, updated: &todo::ChangedSlice, c: &Conf) {
    for (i, id) in selected.iter().enumerate() {
        let print = updated.is_empty() || (i < updated.len() && updated[i]);
        let print = print && (*id < tasks.len());
        if print {
            match c.fmt {
                Format::Full => print_full_info(&tasks[*id], *id + 1, c),
                Format::Short => print_short_info(&tasks[*id], *id + 1, c),
                Format::Custom => print_custom_info(&tasks[*id], *id + 1, c),
            }
        }
    }
}

fn print_body_all(tasks: &todo::TaskSlice, selected: &todo::IDSlice, updated: &todo::ChangedSlice, c: &Conf) {
    for (i, t) in tasks.iter().enumerate() {
        let (id, print) = if i < selected.len() {
            (selected[i], updated[i])
        } else {
            (0, false)
        };
        if print {
            match c.fmt {
                Format::Full => print_full_info(t, id + 1, c),
                Format::Short => print_short_info(t, id + 1, c),
                Format::Custom => print_custom_info(t, id + 1, c),
            }
        }
    }
}

pub fn print_footer(tasks: &todo::TaskSlice, selected: &todo::IDSlice, updated: &todo::ChangedSlice, c: &Conf) {
    let uwidth = c.width as usize;
    if c.atty && SEPARATOR.len() > uwidth {
        let slice = &SEPARATOR[..uwidth];
        println!("{}", slice);
    } else {
        println!("{}", SEPARATOR);
    }

    if updated.is_empty() && !selected.is_empty() {
        println!("{} todos (of {} total)", selected.len(), c.max);
    } else if tasks.len() != updated.len() {
        println!("{} todos (of {} total)", updated.len(), c.max);
    } else {
        println!("{} todos", updated.len());
    }
}

pub fn print_todos(tasks: &todo::TaskSlice, select: &todo::IDSlice, updated: &todo::ChangedSlice, c: &Conf, all: bool) {
    if tasks.is_empty() || select.is_empty() {
        return;
    }

    if all {
        print_body_all(tasks, select, updated, c);
    } else {
        print_body_selected(tasks, select, updated, c);
    }
}
