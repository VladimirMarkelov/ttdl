#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

mod cal;
mod colauto;
mod conf;
mod conv;
mod fmt;
mod stats;
mod subj_clean;
mod tml;

use std::collections::HashMap;
use std::env;
use std::fs::{File, read_to_string};
use std::hash::Hasher;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, exit};
use std::str::FromStr;

use chrono::NaiveDate;
use tempfile::{self, NamedTempFile, TempPath};
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
use todotxt::CompletionConfig;

use crate::cal::CalPrinter;
use crate::human_date::{calendar_first_day, calendar_last_day};
use todo_lib::*;

const TASK_HIDDEN_OFF: &str = "0";
const TASK_HIDDEN_FLD: &str = "h";

type FnDoneUndone =
    fn(tasks: &mut Vec<todotxt::Task>, ids: Option<&todo::IDVec>, mode: todotxt::CompletionConfig) -> todo::ChangedVec;

fn task_is_hidden(task: &todotxt::Task) -> bool {
    match task.tags.get(TASK_HIDDEN_FLD) {
        Some(val) => val != TASK_HIDDEN_OFF,
        None => false,
    }
}

fn is_filter_empty(flt: &tfilter::Conf) -> bool {
    if flt.regex.is_some() || flt.due.is_some() || flt.thr.is_some() || flt.rec.is_some() {
        return false;
    }
    if flt.pri.is_some() || flt.tmr.is_some() || flt.created.is_some() || flt.finished.is_some() {
        return false;
    }
    if let tfilter::ItemRange::None = flt.range {
    } else {
        return false;
    }
    if let tfilter::TodoStatus::Active = flt.all {
    } else {
        return false;
    }
    if !flt.include.projects.is_empty()
        || !flt.include.contexts.is_empty()
        || !flt.include.tags.is_empty()
        || !flt.include.hashtags.is_empty()
    {
        return false;
    }
    if !flt.exclude.projects.is_empty()
        || !flt.exclude.contexts.is_empty()
        || !flt.exclude.tags.is_empty()
        || !flt.exclude.hashtags.is_empty()
    {
        return false;
    }
    true
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
    f: FnDoneUndone,
) -> io::Result<bool> {
    let todos = filter_tasks(tasks, c);

    let completion_config = CompletionConfig {
        completion_mode: c.priority_on_done,
        completion_date_mode: match c.add_completion_date_always {
            true => todotxt::CompletionDateMode::AlwaysSet,
            false => todotxt::CompletionDateMode::WhenCreationDateIsPresent,
        },
    };

    if c.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let old_len = clones.len();
        let updated = f(&mut clones, None, completion_config);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, c);
            writeln!(stdout, "Todos to be {action}:")?;
            fmt::print_header(stdout, &c.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &c.fmt, &cols, &widths, false)?;
            writeln!(stdout, "\nReplace with:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &c.fmt, &cols, &widths, true)?;
            if clones.len() > old_len {
                for idx in old_len..clones.len() {
                    fmt::print_body_single(
                        stdout,
                        &clones,
                        idx,
                        tasks.len() + idx - old_len + 1,
                        &c.fmt,
                        &cols,
                        &widths,
                    )?;
                }
            }
            fmt::print_footer(stdout, tasks, &todos, &updated, &c.fmt, &cols, &widths)?;
        }
        Ok(false)
    } else {
        let old_len = tasks.len();
        let updated = f(tasks, Some(&todos), completion_config);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
            Ok(false)
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, c);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &c.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &c.fmt, &cols, &widths, false)?;
            if old_len < tasks.len() {
                writeln!(stdout, "\nAdded todos:")?;
                for idx in old_len..tasks.len() {
                    fmt::print_body_single(stdout, tasks, idx, idx + 1, &c.fmt, &cols, &widths)?;
                }
            }
            fmt::print_footer(stdout, tasks, &todos, &updated, &c.fmt, &cols, &widths)?;
            Ok(true)
        }
    }
}

fn task_add(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &mut conf::Conf) -> io::Result<()> {
    let subj = match &conf.todo.subject {
        None => {
            eprintln!("Subject is empty");
            return Ok(());
        }
        Some(s) => s.clone(),
    };
    let now = chrono::Local::now().date_naive();
    let mut tag_list = date_expr::TaskTagList::from_str(&subj, now);
    let soon = conf.fmt.colors.soon_days;
    let subj = match date_expr::calculate_main_tags(now, &mut tag_list, soon) {
        Err(e) => {
            eprintln!("{e:?}");
            std::process::exit(1);
        }
        Ok(changed) => match changed {
            false => subj,
            true => date_expr::update_tags_in_str(&tag_list, &subj),
        },
    };
    conf.todo.subject = Some(subj.clone());

    if conf.dry {
        let t = todotxt::Task::parse(&subj, now);
        let (cols, widths) = cols_with_width(std::slice::from_ref(&t), &[0], conf);
        writeln!(stdout, "To be added: ")?;
        fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
        fmt::print_todos(stdout, &[t], &[0], &[true], &conf.fmt, &cols, &widths, true)?;
        return Ok(());
    }

    let id = todo::add(tasks, &conf.todo);
    if id == todo::INVALID_ID {
        writeln!(stdout, "Failed to add: parse error '{subj}'")?;
        std::process::exit(1);
    }

    let (cols, widths) = cols_with_width(tasks, &[id], conf);
    writeln!(stdout, "Added todo:")?;
    fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
    fmt::print_todos(stdout, tasks, &[id], &[true], &conf.fmt, &cols, &widths, false)?;
    if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
        writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
        std::process::exit(1);
    }
    Ok(())
}

fn build_col_list(tasks: &todo::TaskSlice, ids: &todo::IDSlice, conf: &conf::Conf) -> Vec<String> {
    let mut cols: Vec<String> = if conf.auto_show_columns {
        let mut c: Vec<String> = conf.fmt.fields.iter().map(|it| it.to_string()).collect();
        for nf in colauto::collect_non_empty(tasks, ids).drain(..) {
            let found = c.iter().any(|it| it.as_str() == nf.as_str());
            if !found {
                c.push(nf);
            }
        }
        c
    } else {
        fmt::field_list(&conf.fmt).iter().map(|it| it.to_string()).collect()
    };
    if conf.auto_hide_columns {
        let f: Vec<&str> = cols.iter().map(|it| it.as_str()).collect();
        cols = colauto::filter_non_empty(tasks, ids, &f);
    }
    if !conf.always_hide_columns.is_empty() {
        cols.retain(|x| !conf.always_hide_columns.iter().any(|it| it == x));
    }
    let id_exists = cols.iter().any(|it| it == "id");
    if !id_exists {
        cols.push("id".to_string());
    }
    cols
}

fn cols_with_width(tasks: &todo::TaskSlice, ids: &todo::IDSlice, conf: &conf::Conf) -> (Vec<String>, Vec<usize>) {
    let cols = build_col_list(tasks, ids, conf);
    let fs: Vec<&str> = cols.iter().map(|it| it.as_str()).collect();
    let widths = colauto::col_widths(tasks, ids, &fs, &conf.fmt);
    (cols, widths)
}

fn print_task_table(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let mut todos = filter_tasks(tasks, conf);
    let (cols, widths) = cols_with_width(tasks, &todos, conf);
    tsort::sort(&mut todos, tasks, &conf.sort);
    // Apply limitations of maximum numbers of todos shown
    if let Some(max) = conf.max_items {
        todos.truncate(max);
    }
    fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
    fmt::print_todos(stdout, tasks, &todos, &[], &conf.fmt, &cols, &widths, false)?;
    fmt::print_footer(stdout, tasks, &todos, &[], &conf.fmt, &cols, &widths)
}

fn task_list(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let err = print_task_table(stdout, tasks, conf);
    reset_colors(stdout);
    err
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

fn reset_colors(stdout: &mut StandardStream) {
    let mut clr = ColorSpec::new();
    clr.set_fg(None);
    clr.set_bg(None);
    clr.set_reset(true);
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
    let (w, _) = term_size::dimensions().unwrap_or((0, 0));
    if w == 0 {
        eprintln!("Failed to detect terminal dimensions");
        return Ok(());
    }
    let mut cp = CalPrinter::new(start_date, end_date, w as u16);
    loop {
        let done = cp.print_next_line(stdout, counter, today, conf)?;
        if done {
            break;
        }
    }
    Ok(())
}

fn task_list_calendar(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let todos = filter_tasks(tasks, conf);
    let now = chrono::Local::now().date_naive();
    let rng = conf.calendar.expect("calendar range must be set");
    let start_date = calendar_first_day(now, &rng, conf.first_sunday);
    let end_date = calendar_last_day(now, &rng, conf.first_sunday);
    let counter = fill_calendar(start_date, end_date, tasks, &todos);

    let res = print_calendar_body(stdout, now, start_date, end_date, &counter, conf);
    reset_colors(stdout);
    if res.is_err() { res } else { writeln!(stdout) }
}

fn task_done(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    if is_filter_empty(&conf.flt) {
        writeln!(stdout, "Warning: you are going to mark all the tasks 'done'. Please specify tasks to complete.")?;
        std::process::exit(1);
    }
    let processed = process_tasks(stdout, tasks, conf, "completed", todo::done)?;
    if processed && let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
        eprintln!("Failed to save to '{0:?}': {e}", &conf.todo_file);
        std::process::exit(1);
    }
    Ok(())
}

fn task_undone(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    if is_filter_empty(&conf.flt) {
        writeln!(
            stdout,
            "Warning: you are going to undone all the tasks completed tasks. Please specify tasks to undone."
        )?;
        std::process::exit(1);
    }
    let mut flt_conf = conf.clone();
    if flt_conf.flt.all == tfilter::TodoStatus::Active {
        flt_conf.flt.all = tfilter::TodoStatus::Done;
    }
    let undone_adapter: FnDoneUndone = |tasks, ids, config| todo::undone(tasks, ids, config.completion_mode);
    let processed = process_tasks(stdout, tasks, &flt_conf, "uncompleted", undone_adapter)?;
    if processed && let Err(e) = todo::save(tasks, Path::new(&flt_conf.todo_file)) {
        writeln!(stdout, "Failed to save to '{0:?}': {e}", &flt_conf.todo_file)?;
        std::process::exit(1);
    }
    Ok(())
}

fn task_remove(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    if is_filter_empty(&conf.flt) {
        writeln!(stdout, "Warning: deletion of all tasks requested. Please specify tasks to delete.")?;
        std::process::exit(1);
    }
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
        let (cols, widths) = cols_with_width(tasks, &todos, conf);
        fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &[], &conf.fmt, &cols, &widths, false)?;
        fmt::print_footer(stdout, tasks, &todos, &[], &conf.fmt, &cols, &widths)?;
        if !flt_conf.dry {
            let removed = todo::remove(tasks, Some(&todos));
            if calculate_updated(&removed) != 0
                && let Err(e) = todo::save(tasks, Path::new(&flt_conf.todo_file))
            {
                writeln!(stdout, "Failed to save to '{0:?}': {e}", &flt_conf.todo_file)?;
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn task_clean(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    let mut conf = conf.clone();
    conf.flt = tfilter::Conf { all: tfilter::TodoStatus::Done, ..conf.flt.clone() };
    let mut todos = filter_tasks(tasks, &conf);
    let done_todos = todos.clone();
    if !conf.keep_empty {
        let mut empty_conf = conf.clone();
        empty_conf.flt = tfilter::Conf { all: tfilter::TodoStatus::Empty, ..conf.flt.clone() };
        let mut empty_todos = filter_tasks(tasks, &empty_conf);
        for et in empty_todos.drain(..) {
            todos.push(et);
        }
    }
    if todos.is_empty() {
        writeln!(stdout, "No todo archived")?
    } else {
        if conf.dry {
            writeln!(stdout, "Todos to be archived:")?
        } else {
            writeln!(stdout, "Archived todos:")?
        }
        let (cols, widths) = cols_with_width(tasks, &todos, &conf);
        fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &[], &conf.fmt, &cols, &widths, false)?;
        fmt::print_footer(stdout, tasks, &todos, &[], &conf.fmt, &cols, &widths)?;
        if !conf.dry {
            let cloned = todo::clone_tasks(tasks, &done_todos);
            if !conf.wipe
                && let Err(e) = todo::archive(&cloned, &conf.done_file)
            {
                eprintln!("{e:?}");
                exit(1);
            }
            let removed = todo::remove(tasks, Some(&todos));
            if calculate_updated(&removed) != 0
                && let Err(e) = todo::save(tasks, Path::new(&conf.todo_file))
            {
                writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn copy_tags_from_task(subj: &str, task: &mut todotxt::Task) -> String {
    let now = chrono::Local::now().date_naive();
    let mut tsk = todotxt::Task::parse(subj, now);
    tsk.priority = task.priority;
    tsk.create_date = task.create_date;
    tsk.finish_date = task.finish_date;
    tsk.finished = task.finished;
    let mut sbj = format!("{tsk}");

    for prj in &task.projects {
        sbj += &format!(" +{prj}");
    }
    for ctx in &task.contexts {
        sbj += &format!(" @{ctx}");
    }
    for (k, v) in &task.tags {
        sbj += &format!(" {k}:{v}");
    }
    for htag in &task.hashtags {
        sbj += &format!(" #{htag}");
    }
    sbj
}

fn create_temp_file(tasks: &mut todo::TaskVec, ids: &todo::IDVec) -> io::Result<TempPath> {
    let named = NamedTempFile::new()?;
    let filetmp = named.into_temp_path();
    println!("Temp: {filetmp:?}",);
    let mut file = File::create(filetmp.as_os_str())?;
    for idx in ids {
        writeln!(file, "{0}", tasks[*idx])?;
    }
    Ok(filetmp)
}

fn tmp_file_hash(f: &TempPath) -> io::Result<Option<u64>> {
    let mut hasher = std::hash::DefaultHasher::new();
    let mut file = File::open(f.as_os_str())?;
    let mut data = vec![];
    file.read_to_end(&mut data)?;
    let has_some = data.iter().any(|&c| c != b' ' && c != b'\n' && c != b'\r');
    if !has_some {
        Ok(None)
    } else {
        hasher.write(&data);
        Ok(Some(hasher.finish()))
    }
}

fn task_edit(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    if conf.use_editor && conf.dry {
        writeln!(stdout, "Interactive editing does not support dry run")?;
        std::process::exit(1);
    }
    let editor = conf.editor();
    if conf.use_editor && conf.editor().is_none() {
        writeln!(
            stdout,
            "Interactive editing requires setting up a path to an editor. Either set environment variable 'EDITOR' or define a path to an editor in TTDL config in the section 'global'"
        )?;
        std::process::exit(1);
    }
    if is_filter_empty(&conf.flt) && !conf.use_editor {
        writeln!(stdout, "Warning: modifying of all tasks requested. Please specify tasks to edit.")?;
        std::process::exit(1);
    }
    let todos = filter_tasks(tasks, conf);
    let action = "changed";
    if todos.is_empty() {
        writeln!(stdout, "No todo changed")?
    } else if conf.use_editor {
        // unwrap cannot fail here as we already check it for 'Some' before.
        let editor = editor.unwrap();
        let filepath = create_temp_file(tasks, &todos)?;
        let orig_hash = tmp_file_hash(&filepath)?;
        let mut child = Command::new(editor).arg(filepath.as_os_str()).spawn()?;
        if let Err(e) = child.wait() {
            writeln!(stdout, "Failed to execute editor: {e:?}")?;
            exit(1);
        }
        let new_hash = tmp_file_hash(&filepath)?;
        match (orig_hash, new_hash) {
            (_, None) => {
                writeln!(stdout, "Empty file detected. Edit operation canceled")?;
                exit(0);
            }
            (_, Some(b)) => {
                let now = chrono::Local::now().date_naive();
                let content = read_to_string(filepath.as_os_str())?;
                let mut removed_cnt = 0;
                if let Some(a) = orig_hash {
                    if a == b {
                        // The temporary file was not changed. Nothing to do
                        writeln!(stdout, "No changes detected. Edit operation canceled")?;
                        exit(0);
                    }
                    let removed = todo::remove(tasks, Some(&todos));
                    removed_cnt = calculate_updated(&removed);
                }
                let mut added_cnt = 0;
                for line in content.lines() {
                    let subj = line.trim();
                    if subj.is_empty() {
                        continue;
                    }
                    // TODO: move duplicated code (here and in task_add) to a separate fn
                    let mut tag_list = date_expr::TaskTagList::from_str(subj, now);
                    let soon = conf.fmt.colors.soon_days;
                    let subj = match date_expr::calculate_main_tags(now, &mut tag_list, soon) {
                        Err(e) => {
                            writeln!(stdout, "{e:?}")?;
                            exit(1);
                        }
                        Ok(changed) => match changed {
                            false => subj.to_string(),
                            true => date_expr::update_tags_in_str(&tag_list, subj),
                        },
                    };
                    let mut cnf = conf.clone();
                    cnf.todo.subject = Some(subj.clone());
                    let id = todo::add(tasks, &cnf.todo);
                    if id == todo::INVALID_ID {
                        writeln!(stdout, "Failed to add: parse error '{subj}'")?;
                    } else {
                        added_cnt += 1;
                    }
                }
                if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                    writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                    std::process::exit(1);
                }
                writeln!(stdout, "Removed {removed_cnt} tasks, added {added_cnt} tasks.")?;
            }
        }
        filepath.close()?;
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = if conf.keep_tags {
            if let Some(ref subj) = conf.todo.subject {
                let sbj = copy_tags_from_task(subj, &mut clones[0]);
                let now = chrono::Local::now().date_naive();
                let tsk = todotxt::Task::parse(&sbj, now);
                clones[0] = tsk;
                let mut changed = vec![false; clones.len()];
                changed[0] = true;
                changed
            } else {
                writeln!(stdout, "The option keep-tags can be used only when setting a new subject")?;
                std::process::exit(1);
            }
        } else {
            todo::edit(&mut clones, None, &conf.todo)
        };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, conf);
            writeln!(stdout, "Todos to be {action}:")?;
            fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
            writeln!(stdout, "\nNew todos:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &cols, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
        }
    } else {
        let updated = if conf.keep_tags {
            if let Some(ref subj) = conf.todo.subject {
                let id = todos[0];
                let sbj = copy_tags_from_task(subj, &mut tasks[id]);
                let now = chrono::Local::now().date_naive();
                let tsk = todotxt::Task::parse(&sbj, now);
                tasks[id] = tsk;
                let mut changed = vec![false; todos.len()];
                changed[0] = true;
                changed
            } else {
                writeln!(stdout, "The option keep-tags can be used only when setting a new subject")?;
                std::process::exit(1);
            }
        } else {
            todo::edit(tasks, Some(&todos), &conf.todo)
        };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, conf);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
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
    if is_filter_empty(&conf.flt) {
        writeln!(stdout, "Warning: you are going to add text to all tasks. Please specify tasks to modify.")?;
        std::process::exit(1);
    }
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

        let (cols, widths) = cols_with_width(tasks, &todos, conf);
        writeln!(stdout, "Todos to be changed:")?;
        fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
        writeln!(stdout, "\nNew todos:")?;
        fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &cols, &widths, true)?;
        fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
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

        let (cols, widths) = cols_with_width(tasks, &todos, conf);
        writeln!(stdout, "Changed todos:")?;
        fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
        fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
        fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
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
            let (cols, widths) = cols_with_width(tasks, &todos, conf);
            writeln!(stdout, "Todos to be {action}:")?;
            fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
            writeln!(stdout, "\nNew todos:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &cols, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
        }
    } else {
        let updated = if start { todo::start(tasks, Some(&todos)) } else { todo::stop(tasks, Some(&todos)) };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was {action}")?;
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, conf);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                writeln!(stdout, "Failed to save to '{0:?}': {e}", &conf.todo_file)?;
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn postpone_and_stop_recurrence(todo: &mut todotxt::Task, rec: &todotxt::Recurrence) -> NaiveDate {
    let now = chrono::Local::now().date_naive();
    let dt = if let Some(dd) = todo.due_date { dd } else { now };
    let new_date = rec.next_date(dt);
    todo.update_tag_with_value(todotxt::DUE_TAG, &todotxt::format_date(new_date));
    todo.update_tag_with_value(todotxt::REC_TAG, "");
    todo.recurrence = None;
    todo.due_date = Some(new_date);
    new_date
}

fn postpone_move_date_after(todo: &mut todotxt::Task, rec: &todotxt::Recurrence, after: NaiveDate) -> NaiveDate {
    let now = chrono::Local::now().date_naive();
    let dt = if let Some(dd) = todo.due_date { dd } else { now };
    let mut new_due = rec.next_date(dt);
    while new_due < after {
        new_due = rec.next_date(new_due);
    }
    todo.update_tag_with_value(todotxt::DUE_TAG, &todotxt::format_date(new_due));
    todo.due_date = Some(new_due);
    new_due
}

fn task_postpone(stdout: &mut StandardStream, tasks: &mut todo::TaskVec, conf: &conf::Conf) -> io::Result<()> {
    if is_filter_empty(&conf.flt) {
        writeln!(stdout, "Warning: postponing of all tasks requested. Please specify tasks to postpone.")?;
        std::process::exit(1);
    }
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
    let mut todos = filter_tasks(tasks, conf);
    if todos.is_empty() {
        writeln!(stdout, "No todo postponed")?
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let mut updated: Vec<bool> = Vec::new();
        let mut new_tasks: todo::TaskVec = Vec::new();
        for clone in clones.iter_mut() {
            if clone.finished || clone.due_date.is_none() {
                updated.push(false);
            } else if let Some(dt) = clone.due_date {
                let task_rec = if let Some(rr) = clone.recurrence {
                    rr
                } else {
                    todotxt::Recurrence { period: todotxt::Period::Day, count: 0, strict: false }
                };
                if task_rec.strict {
                    let mut new_task = clone.clone();
                    let new_date = postpone_and_stop_recurrence(&mut new_task, &rec);
                    let new_due = postpone_move_date_after(clone, &task_rec, new_date);
                    updated.push(true);
                    if new_due != new_date {
                        new_tasks.push(new_task);
                    }
                } else {
                    let new_due = rec.next_date(dt);
                    clone.update_tag_with_value(todotxt::DUE_TAG, &todotxt::format_date(new_due));
                    updated.push(true);
                }
            }
        }
        for (idx, t) in new_tasks.drain(..).enumerate() {
            clones.push(t);
            todos.push(tasks.len() + idx);
            updated.push(true);
        }
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            writeln!(stdout, "No todo was postponed")?;
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, conf);
            writeln!(stdout, "Todos to be postponed:")?;
            fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
            writeln!(stdout, "\nNew todos:")?;
            fmt::print_todos(stdout, &clones, &todos, &updated, &conf.fmt, &cols, &widths, true)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
        }
    } else {
        let mut updated: Vec<bool> = Vec::new();
        let mut new_tasks: todo::TaskVec = Vec::new();
        for idx in todos.iter() {
            if *idx >= tasks.len() || tasks[*idx].finished {
                updated.push(false);
            } else if let Some(dt) = tasks[*idx].due_date {
                let task_rec = if let Some(rr) = tasks[*idx].recurrence {
                    rr
                } else {
                    todotxt::Recurrence { period: todotxt::Period::Day, count: 0, strict: false }
                };
                if task_rec.strict {
                    let mut new_task = tasks[*idx].clone();
                    let new_date = postpone_and_stop_recurrence(&mut new_task, &rec);
                    let new_due = postpone_move_date_after(&mut tasks[*idx], &task_rec, new_date);
                    updated.push(true);
                    if new_due != new_date {
                        new_tasks.push(new_task);
                    }
                } else {
                    let new_due = rec.next_date(dt);
                    tasks[*idx].update_tag_with_value(todotxt::DUE_TAG, &todotxt::format_date(new_due));
                    tasks[*idx].due_date = Some(new_due);
                    updated.push(true);
                }
            } else {
                updated.push(false);
            }
        }
        for t in new_tasks.drain(..) {
            tasks.push(t.clone());
            todos.push(tasks.len() - 1);
            updated.push(true);
        }
        let updated_cnt = calculate_updated(&updated);
        if updated_cnt == 0 {
            writeln!(stdout, "No todo was postponed")?;
        } else {
            let (cols, widths) = cols_with_width(tasks, &todos, conf);
            writeln!(stdout, "Changed todos:")?;
            fmt::print_header(stdout, &conf.fmt, &cols, &widths)?;
            fmt::print_todos(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths, false)?;
            fmt::print_footer(stdout, tasks, &todos, &updated, &conf.fmt, &cols, &widths)?;
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

fn task_list_hashtags(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &conf::Conf) -> io::Result<()> {
    let mut conf = conf.clone();
    conf.show_hidden = true;
    let todos = filter_tasks(tasks, &conf);
    // no tsort::sort() here since multiple contexts in one task
    // would mess up the alphabetical output sort

    for item in collect_unique_items(tasks, &todos, |task| &task.hashtags) {
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
        conf::RunMode::Add => task_add(&mut stdout, &mut tasks, &mut conf),
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
        conf::RunMode::ListHashtags => task_list_hashtags(&mut stdout, &tasks, &conf),
        _ => Ok(()),
    };
    if let Err(e) = err
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
