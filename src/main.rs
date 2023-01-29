#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

mod conf;
mod conv;
mod fmt;
mod human_date;
mod stats;
mod tml;

use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::exit;
use std::str::FromStr;

use chrono::{Datelike, NaiveDate, Weekday};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::human_date::{calendar_first_day, calendar_last_day, prev_weekday};
use todo_lib::*;

const TASK_HIDDEN_OFF: &str = "0";
const TASK_HIDDEN_FLD: &str = "h";

type FnUpdateData = fn(tasks: &mut Vec<todotxt::Task>, ids: Option<&todo::IDVec>) -> todo::ChangedVec;

fn task_is_hidden(task: &todotxt::Task) -> bool {
    match task.tags.get(TASK_HIDDEN_FLD) {
        Some(val) => val != TASK_HIDDEN_OFF,
        None => false,
    }
}

fn filter_tasks(tasks: &[todotxt::Task], c: &conf::Conf) -> todo::IDVec {
    let mut todos = tfilter::filter(tasks, &c.flt);
    if !c.show_hidden {
        todos.retain(|&id| !task_is_hidden(&tasks[id]));
    }
    todos
}

fn calculate_updated(v: &todo::ChangedSlice) -> u32 {
    let mut cnt = 0u32;
    for b in v.iter() {
        if *b {
            cnt += 1;
        }
    }

    cnt
}

fn process_tasks(
    stdout: &mut StandardStream,
    tasks: &mut todo::TaskVec,
    c: &conf::Conf,
    action: &str,
    f: FnUpdateData,
) -> io::Result<bool> {
    let todos = filter_tasks(tasks, c);

    if c.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = f(&mut clones, None);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let widths = fmt::field_widths(&c.fmt, tasks, &todos);
            writeln!(stdout, "Todos to be {action}:")?;
            fmt::print_header(stdout, &c.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &c.fmt, &widths, false)?;
            writeln!(stdout, "\nReplace with:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &c.fmt, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &c.fmt, &widths)?;
        }
        Ok(false)
    } else {
        let updated = f(tasks, Some(&todos));
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
            Ok(false)
        } else {
            let widths = fmt::field_widths(&c.fmt, tasks, &todos);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &c.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &c.fmt, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &c.fmt, &widths)?;
            Ok(true)
        }
    }
}

fn task_add(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let subj = match &conf.todo.subject {
        None => {
            eprintln!("Subject is empty");
            return Ok(());
        }
        Some(s) => s.clone(),
    };
    let now = chrono::Local::now().date_naive();
    if conf.dry {
        let t = todotxt::Task::parse(&subj, now);
        let widths = fmt::field_widths(&conf.fmt, &[t.clone()], &[tasks.len()]);
        writeln!(stdout, "To be added: ")?;
        fmt::print_header(stdout, &conf.fmt, &widths)?;
        fmt::print_todos(stdout, &[t], &[tasks.len()], &[true], &conf.fmt, &widths, true)?;
        return Ok(());
    }

    let id = todo::add(tasks, &conf.todo);
    if id == todo::INVALID_ID {
        writeln!(stdout, "Failed to add: parse error '{subj}'")?;
        std::process::exit(1);
    }

    let widths = fmt::field_widths(&conf.fmt, tasks, &[id]);
    writeln!(stdout, "Added todo:")?;
    fmt::print_header(stdout, &conf.fmt, &widths)?;
    fmt::print_todos(stdout, tasks, &[id], &[true], &conf.fmt, &widths, false)?;
    if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
        writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
        std::process::exit(1);
    }
    Ok(())
}

fn task_list(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let mut todos = filter_tasks(tasks, conf);
    let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
    tsort::sort(&mut todos, tasks, &conf.sort);
    fmt::print_header(stdout, &conf.fmt, &widths)?;
    fmt::print_todos(stdout, tasks, &todos, &[], &conf.fmt, &widths, false)?;
    fmt::print_footer(stdout, tasks, &todos, &[], &conf.fmt, &widths)
}

fn fill_calendar(
    first_date: NaiveDate,
    last_date: NaiveDate,
    tasks: &todo::TaskSlice,
    selected: &todo::IDSlice,
) -> HashMap<NaiveDate, u32> {
    let mut res = HashMap::new();
    for id in selected.iter() {
        if let Some(dt) = tasks[*id].due_date {
            if dt < first_date || dt > last_date {
                continue;
            }
            *res.entry(dt).or_insert(0) += 1;
        }
    }
    res
}

fn print_calendar_header(stdout: &mut StandardStream, conf: &conf::Conf) -> io::Result<()> {
    if conf.first_sunday {
        writeln!(stdout, " Su Mo Tu We Th Fr Sa")
    } else {
        writeln!(stdout, " Mo Tu We Th Fr Sa Su")
    }
}

fn reset_colors(stdout: &mut StandardStream) {
    let mut clr = ColorSpec::new();
    clr.set_fg(Some(Color::White));
    clr.set_bg(Some(Color::Black));
    let _ = stdout.set_color(&clr);
}

fn print_calendar_body(
    stdout: &mut StandardStream,
    today: NaiveDate,
    start_date: NaiveDate,
    end_date: NaiveDate,
    counter: &HashMap<NaiveDate, u32>,
    conf: &conf::Conf,
) -> io::Result<()> {
    let is_first = (start_date.weekday() == Weekday::Sun && conf.first_sunday)
        || (start_date.weekday() == Weekday::Mon && !conf.first_sunday);
    let mut from_date = if is_first {
        start_date
    } else if conf.first_sunday {
        prev_weekday(start_date, Weekday::Sun)
    } else {
        prev_weekday(start_date, Weekday::Mon)
    };
    while from_date <= end_date {
        if from_date < start_date {
            write!(stdout, "   ")?;
            from_date = from_date.succ_opt().unwrap_or(from_date);
            continue;
        }
        let bg = if from_date == today { Color::Blue } else { Color::Black };
        let fg = match counter.get(&from_date) {
            None => Color::White,
            Some(n) => {
                if n > &1 {
                    Color::Red
                } else {
                    Color::Magenta
                }
            }
        };
        let st = format!(" {:>2}", from_date.day());
        let mut clr = ColorSpec::new();
        clr.set_fg(Some(fg));
        clr.set_bg(Some(bg));
        let _ = stdout.set_color(&clr);
        let _ = stdout.write(st.as_bytes());
        let wd = from_date.weekday();
        if (wd == Weekday::Sun && !conf.first_sunday) || (wd == Weekday::Sat && conf.first_sunday) {
            reset_colors(stdout);
            let _ = stdout.write(b"\n");
        }
        from_date = from_date.succ_opt().unwrap_or(from_date);
    }
    reset_colors(stdout);
    writeln!(stdout)
}

fn task_list_calendar(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let todos = filter_tasks(tasks, conf);
    let now = chrono::Local::now().date_naive();
    let rng = conf.calendar.expect("calendar range must be set");
    let start_date = calendar_first_day(now, &rng, conf.first_sunday);
    let end_date = calendar_last_day(now, &rng, conf.first_sunday);
    let counter = fill_calendar(start_date, end_date, tasks, &todos);

    print_calendar_header(stdout, conf)?;
    print_calendar_body(stdout, now, start_date, end_date, &counter, conf)?;
    writeln!(stdout)
}

fn task_done(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let processed = process_tasks(stdout, tasks, conf, "completed", todo::done)?;
    if processed {
        if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
            eprintln!("Failed to save to '{0:?}': {e}", &conf.todo_file);
            std::process::exit(1);
        }
    }
    Ok(())
}

fn task_undone(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let mut flt_conf = conf.clone();
    if flt_conf.flt.all == tfilter::TodoStatus::Active {
        flt_conf.flt.all = tfilter::TodoStatus::Done;
    }

    let processed = process_tasks(stdout, tasks, &flt_conf, "uncompleted", todo::undone)?;
    if processed {
        if let Err(e) = todo::save(tasks, Path::new(&flt_conf.todo_file)) {
            writeln!(stdout, "Failed to save to '{0:?}': {e}", &flt_conf.todo_file)?;
            std::process::exit(1);
        }
    }
    Ok(())
}

fn task_remove(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let mut flt_conf = conf.clone();
    if flt_conf.flt.all == tfilter::TodoStatus::Active {
        flt_conf.flt.all = tfilter::TodoStatus::All;
    }
    let todos = filter_tasks(tasks, conf);
    if todos.is_empty() {
        writeln!(stdout, "No todo deleted")?
    } else {
        if flt_conf.dry {
            writeln!(stdout, "Todos to be removed:")?
        } else {
            writeln!(stdout, "Removed todos:")?
        }
        let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
        fmt::print_header(stdout, &conf.fmt, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &[], &conf.fmt, &widths, false)?;
        fmt::print_footer(stdout, tasks, &todos, &[], &conf.fmt, &widths)?;
        if !flt_conf.dry {
            let removed = todo::remove(tasks, Some(&todos));
            if calculate_updated(&removed) != 0 {
                if let Err(e) = todo::save(tasks, Path::new(&flt_conf.todo_file)) {
                    writeln!(stdout, "Failed to save to '{0:?}': {e}", &flt_conf.todo_file)?;
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

fn task_clean(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let mut conf = conf.clone();
    conf.flt = tfilter::Conf { all: tfilter::TodoStatus::Done, ..conf.flt.clone() };
    let todos = filter_tasks(tasks, &conf);
    if todos.is_empty() {
        writeln!(stdout, "No todo archived")?
    } else {
        if conf.dry {
            writeln!(stdout, "Todos to be archived:")?
        } else {
            writeln!(stdout, "Archived todos:")?
        }
        let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
        fmt::print_header(stdout, &conf.fmt, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &[], &conf.fmt, &widths, false)?;
        fmt::print_footer(stdout, tasks, &todos, &[], &conf.fmt, &widths)?;
        if !conf.dry {
            let cloned = todo::clone_tasks(tasks, &todos);
            if !conf.wipe {
                if let Err(e) = todo::archive(&cloned, &conf.done_file) {
                    eprintln!("{e:?}");
                    exit(1);
                }
            }
            let removed = todo::remove(tasks, Some(&todos));
            if calculate_updated(&removed) != 0 {
                if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                    writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}

fn task_edit(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let todos = filter_tasks(tasks, conf);
    let action = "changed";
    if todos.is_empty() {
        writeln!(stdout, "No todo changed")?
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = todo::edit(&mut clones, None, &conf.todo);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            writeln!(stdout, "Todos to be {action}:")?;
            fmt::print_header(stdout, &conf.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
            writeln!(stdout, "\nNew todos:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
        }
    } else {
        let updated = todo::edit(tasks, Some(&todos), &conf.todo);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &conf.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn task_add_text(
    stdout: &mut StandardStream,
    tasks: &mut todo::TaskVec,
    conf: &conf::Conf,
    to_end: bool,
) -> io::Result<()> {
    let subj = match &conf.todo.subject {
        None => {
            writeln!(stdout, "Subject is empty")?;
            return Ok(());
        }
        Some(s) => s,
    };
    let todos = filter_tasks(tasks, conf);
    if todos.is_empty() {
        writeln!(stdout, "No todo changed")?;
        return Ok(());
    }

    if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated: Vec<bool> = vec![true; clones.len()];

        for t in clones.iter_mut() {
            if to_end {
                if !subj.starts_with(' ') {
                    t.subject.push(' ');
                }
                t.subject.push_str(subj);
            } else if subj.ends_with(' ') {
                t.subject = format!("{}{}", subj, t.subject);
            } else {
                t.subject = format!("{} {}", subj, t.subject);
            }
        }

        let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
        writeln!(stdout, "Todos to be changed:")?;
        fmt::print_header(stdout, &conf.fmt, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
        writeln!(stdout, "\nNew todos:")?;
        fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &widths, true)?;
        fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
    } else {
        let updated: Vec<bool> = vec![true; todos.len()];

        for idx in todos.iter() {
            if *idx >= tasks.len() {
                continue;
            }

            if to_end {
                tasks[*idx].subject.push(' ');
                tasks[*idx].subject.push_str(subj);
            } else {
                tasks[*idx].subject = format!("{} {}", subj, tasks[*idx].subject);
            }
        }

        let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
        writeln!(stdout, "Changed todos:")?;
        fmt::print_header(stdout, &conf.fmt, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
        fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
        if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
            writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
            std::process::exit(1);
        }
    }
    Ok(())
}

fn task_start_stop(
    stdout: &mut StandardStream,
    tasks: &mut todo::TaskVec,
    conf: &conf::Conf,
    start: bool,
) -> io::Result<()> {
    let todos = filter_tasks(tasks, conf);
    let action = if start { "started" } else { "stopped" };
    if todos.is_empty() {
        writeln!(stdout, "No todo {action}")?
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = if start { todo::start(&mut clones, None) } else { todo::stop(&mut clones, None) };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            writeln!(stdout, "Todos to be {action}:")?;
            fmt::print_header(stdout, &conf.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
            writeln!(stdout, "\nNew todos:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
        }
    } else {
        let updated = if start { todo::start(tasks, Some(&todos)) } else { todo::stop(tasks, Some(&todos)) };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &conf.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn task_postpone(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let subj = match conf.todo.subject {
        Some(ref s) => s,
        None => {
            writeln!(stdout, "Postpone range is not defined")?;
            return Ok(());
        }
    };
    let rec = match todotxt::Recurrence::from_str(subj) {
        Ok(r) => r,
        Err(e) => {
            writeln!(stdout, "Invalid recurrence format: {e:?}")?;
            return Ok(());
        }
    };
    let todos = filter_tasks(tasks, conf);
    if todos.is_empty() {
        writeln!(stdout, "No todo postponed")?
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let mut updated: Vec<bool> = Vec::new();
        for clone in clones.iter_mut() {
            if clone.finished || clone.due_date.is_none() {
                updated.push(false);
            } else if let Some(dt) = clone.due_date {
                let new_due = rec.next_date(dt);
                clone.update_tag_with_value(todotxt::DUE_TAG, &todotxt::format_date(new_due));
                updated.push(true);
            }
        }
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was postponed")?;
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            writeln!(stdout, "Todos to be postponed:")?;
            fmt::print_header(stdout, &conf.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
            writeln!(stdout, "\nNew todos:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
        }
    } else {
        let mut updated: Vec<bool> = Vec::new();
        for idx in todos.iter() {
            if *idx >= tasks.len() || tasks[*idx].finished {
                updated.push(false);
            } else if let Some(dt) = tasks[*idx].due_date {
                let new_due = rec.next_date(dt);
                tasks[*idx].update_tag_with_value(todotxt::DUE_TAG, &todotxt::format_date(new_due));
                tasks[*idx].due_date = Some(new_due);
                updated.push(true);
            } else {
                updated.push(false);
            }
        }
        let updated_cnt = calculate_updated(&updated);
        if updated_cnt == 0 {
            writeln!(stdout, "No todo was postponed")?;
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &conf.fmt, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &widths)?;
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

// helper function to collect list of unique project tags / context tags
fn collect_unique_items<F>(tasks: &todo::TaskSlice, selected: &todo::IDSlice, get_items: F) -> Vec<String>
where
    F: Fn(&todotxt::Task) -> &Vec<String>,
{
    let mut items: Vec<String> = Vec::new();

    for idx in selected {
        if *idx >= tasks.len() {
            continue;
        }

        // get items from closure function
        for item in get_items(&tasks[*idx]).iter() {
            if !item.is_empty() && !items.contains(item) {
                items.push(item.clone());
            }
        }
    }
    items.sort();

    items
}

fn task_list_projects(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let mut conf = conf.clone();
    conf.show_hidden = true;
    let todos = filter_tasks(tasks, &conf);
    // no tsort::sort() here since multiple projects in one task
    // would mess up the alphabetical output sort

    for item in collect_unique_items(tasks, &todos, |task| &task.projects) {
        writeln!(stdout, "{item}")?;
    }
    Ok(())
}

fn task_list_contexts(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let mut conf = conf.clone();
    conf.show_hidden = true;
    let todos = filter_tasks(tasks, &conf);
    // no tsort::sort() here since multiple contexts in one task
    // would mess up the alphabetical output sort

    for item in collect_unique_items(tasks, &todos, |task| &task.contexts) {
        writeln!(stdout, "{item}")?;
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut conf = match conf::parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    };

    let mut tasks: todo::TaskVec = if conf.use_done {
        match todo::load(Path::new(&conf.done_file)) {
            Ok(tlist) => tlist,
            Err(e) => {
                eprintln!("Failed to load todo list: {e:?}");
                exit(1);
            }
        }
    } else {
        match todo::load(Path::new(&conf.todo_file)) {
            Ok(tlist) => tlist,
            Err(e) => {
                eprintln!("Failed to load done list: {e:?}");
                exit(1);
            }
        }
    };
    conf.fmt.max = tasks.len();

    if conf.mode == conf::RunMode::None {
        if conf.todo.subject.is_none() {
            conf.mode = conf::RunMode::List;
        } else {
            conf.mode = conf::RunMode::Add;
        }
    }

    if conf.use_done && !conf::can_run_for_done(conf.mode) {
        eprintln!("Invalid command: when using done.txt the only available command is `list`");
        exit(1);
    }

    let mut stdout = match conf.fmt.color_term {
        fmt::TermColorType::Ansi => StandardStream::stdout(ColorChoice::AlwaysAnsi),
        fmt::TermColorType::Auto => StandardStream::stdout(ColorChoice::Always),
        fmt::TermColorType::None => StandardStream::stdout(ColorChoice::Never),
    };

    let err = match conf.mode {
        conf::RunMode::Add => task_add(&mut stdout, &mut tasks, &conf),
        conf::RunMode::List => {
            if conf.calendar.is_none() {
                task_list(&mut stdout, &tasks, &conf)
            } else {
                task_list_calendar(&mut stdout, &tasks, &conf)
            }
        }
        conf::RunMode::Done => task_done(&mut stdout, &mut tasks, &conf),
        conf::RunMode::Undone => task_undone(&mut stdout, &mut tasks, &conf),
        conf::RunMode::Remove => task_remove(&mut stdout, &mut tasks, &conf),
        conf::RunMode::Clean => task_clean(&mut stdout, &mut tasks, &conf),
        conf::RunMode::Edit => task_edit(&mut stdout, &mut tasks, &conf),
        conf::RunMode::Append => task_add_text(&mut stdout, &mut tasks, &conf, true),
        conf::RunMode::Prepend => task_add_text(&mut stdout, &mut tasks, &conf, false),
        conf::RunMode::Start => task_start_stop(&mut stdout, &mut tasks, &conf, true),
        conf::RunMode::Stop => task_start_stop(&mut stdout, &mut tasks, &conf, false),
        conf::RunMode::Stats => stats::show_stats(&mut stdout, &tasks, &conf.fmt),
        conf::RunMode::Postpone => task_postpone(&mut stdout, &mut tasks, &conf),
        conf::RunMode::ListProjects => task_list_projects(&mut stdout, &tasks, &conf),
        conf::RunMode::ListContexts => task_list_contexts(&mut stdout, &tasks, &conf),
        _ => Ok(()),
    };
    if let Err(e) = err {
        if e.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
