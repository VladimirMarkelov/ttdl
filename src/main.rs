#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

mod conf;
mod fmt;
mod human_date;
mod stats;
mod tml;

use std::env;
use std::path::Path;
use std::process::exit;
use std::str::FromStr;

use todo_lib::*;

type FnUpdateData = fn(tasks: &mut Vec<todotxt::Task>, ids: Option<&todo::IDVec>) -> todo::ChangedVec;

fn calculate_updated(v: &todo::ChangedSlice) -> u32 {
    let mut cnt = 0u32;
    for b in v.iter() {
        if *b {
            cnt += 1;
        }
    }

    cnt
}

fn process_tasks(tasks: &mut todo::TaskVec, c: &conf::Conf, action: &str, f: FnUpdateData) -> bool {
    let todos = tfilter::filter(tasks, &c.flt);

    if c.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = f(&mut clones, None);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            println!("No todo was {}", action);
        } else {
            let widths = fmt::field_widths(&c.fmt, tasks, &todos);
            println!("Todos to be {}:", action);
            fmt::print_header(&c.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &c.fmt, &widths, false);
            println!("\nReplace with:");
            fmt::print_todos(&clones, &todos, &updated, &c.fmt, &widths, true);
            fmt::print_footer(tasks, &todos, &updated, &c.fmt, &widths);
        }
        false
    } else {
        let updated = f(tasks, Some(&todos));
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            println!("No todo was {}", action);
            false
        } else {
            let widths = fmt::field_widths(&c.fmt, tasks, &todos);
            println!("Changed todos:");
            fmt::print_header(&c.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &c.fmt, &widths, false);
            fmt::print_footer(tasks, &todos, &updated, &c.fmt, &widths);
            true
        }
    }
}

fn task_add(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    let subj = match &conf.todo.subject {
        None => {
            eprintln!("Subject is empty");
            return;
        }
        Some(s) => s.clone(),
    };
    let now = chrono::Local::now().date().naive_local();
    if conf.dry {
        let t = todotxt::Task::parse(&subj, now);
        let widths = fmt::field_widths(&conf.fmt, &[t.clone()], &[tasks.len()]);
        println!("To be added: ");
        fmt::print_header(&conf.fmt, &widths);
        fmt::print_todos(&[t], &[tasks.len()], &[true], &conf.fmt, &widths, true);
        return;
    }

    let id = todo::add(tasks, &conf.todo);
    if id == todo::INVALID_ID {
        println!("Failed to add: parse error '{}'", subj);
        std::process::exit(1);
    }

    let widths = fmt::field_widths(&conf.fmt, tasks, &[id]);
    println!("Added todo:");
    fmt::print_header(&conf.fmt, &widths);
    fmt::print_todos(tasks, &[id], &[true], &conf.fmt, &widths, false);
    if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
        println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
        std::process::exit(1);
    }
}

fn task_list(tasks: &todo::TaskSlice, conf: &conf::Conf) {
    let mut todos = tfilter::filter(tasks, &conf.flt);
    let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
    tsort::sort(&mut todos, tasks, &conf.sort);
    fmt::print_header(&conf.fmt, &widths);
    fmt::print_todos(tasks, &todos, &[], &conf.fmt, &widths, false);
    fmt::print_footer(tasks, &todos, &[], &conf.fmt, &widths);
}

fn task_done(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    if process_tasks(tasks, conf, "completed", todo::done) {
        if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
            println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
            std::process::exit(1);
        }
    }
}

fn task_undone(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    let mut flt_conf = conf.clone();
    if flt_conf.flt.all == tfilter::TodoStatus::Active {
        flt_conf.flt.all = tfilter::TodoStatus::Done;
    }

    if process_tasks(tasks, &flt_conf, "uncompleted", todo::undone) {
        if let Err(e) = todo::save(tasks, Path::new(&flt_conf.todo_file)) {
            println!("Failed to save to '{:?}': {}", &flt_conf.todo_file, e);
            std::process::exit(1);
        }
    }
}

fn task_remove(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    let mut flt_conf = conf.clone();
    if flt_conf.flt.all == tfilter::TodoStatus::Active {
        flt_conf.flt.all = tfilter::TodoStatus::All;
    }
    let todos = tfilter::filter(tasks, &flt_conf.flt);
    if todos.is_empty() {
        println!("No todo deleted")
    } else {
        if flt_conf.dry {
            println!("Todos to be removed:")
        } else {
            println!("Removed todos:")
        }
        let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
        fmt::print_header(&conf.fmt, &widths);
        fmt::print_todos(tasks, &todos, &[], &conf.fmt, &widths, false);
        fmt::print_footer(tasks, &todos, &[], &conf.fmt, &widths);
        if !flt_conf.dry {
            let removed = todo::remove(tasks, Some(&todos));
            if calculate_updated(&removed) != 0 {
                if let Err(e) = todo::save(tasks, Path::new(&flt_conf.todo_file)) {
                    println!("Failed to save to '{:?}': {}", &flt_conf.todo_file, e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn task_clean(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    let flt_conf = tfilter::Conf { all: tfilter::TodoStatus::Done, ..conf.flt.clone() };
    let todos = tfilter::filter(tasks, &flt_conf);
    if todos.is_empty() {
        println!("No todo archived")
    } else {
        if conf.dry {
            println!("Todos to be archived:")
        } else {
            println!("Archived todos:")
        }
        let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
        fmt::print_header(&conf.fmt, &widths);
        fmt::print_todos(tasks, &todos, &[], &conf.fmt, &widths, false);
        fmt::print_footer(tasks, &todos, &[], &conf.fmt, &widths);
        if !conf.dry {
            let cloned = todo::clone_tasks(tasks, &todos);
            if !conf.wipe {
                if let Err(e) = todo::archive(&cloned, &conf.done_file) {
                    eprintln!("{:?}", e);
                    exit(1);
                }
            }
            let removed = todo::remove(tasks, Some(&todos));
            if calculate_updated(&removed) != 0 {
                if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                    println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn task_edit(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    let todos = tfilter::filter(tasks, &conf.flt);
    let action = "changed";
    if todos.is_empty() {
        println!("No todo changed")
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = todo::edit(&mut clones, None, &conf.todo);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            println!("No todo was {}", action);
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            println!("Todos to be {}:", action);
            fmt::print_header(&conf.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
            println!("\nNew todos:");
            fmt::print_todos(&clones, &todos, &updated, &conf.fmt, &widths, true);
            fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
        }
    } else {
        let updated = todo::edit(tasks, Some(&todos), &conf.todo);
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            println!("No todo was {}", action);
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            println!("Changed todos:");
            fmt::print_header(&conf.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
            fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
                std::process::exit(1);
            }
        }
    }
}

fn task_add_text(tasks: &mut todo::TaskVec, conf: &conf::Conf, to_end: bool) {
    let subj = match &conf.todo.subject {
        None => {
            println!("Subject is empty");
            return;
        }
        Some(s) => s,
    };
    let todos = tfilter::filter(tasks, &conf.flt);
    if todos.is_empty() {
        println!("No todo changed");
        return;
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
        println!("Todos to be changed:");
        fmt::print_header(&conf.fmt, &widths);
        fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
        println!("\nNew todos:");
        fmt::print_todos(&clones, &todos, &updated, &conf.fmt, &widths, true);
        fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
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
        println!("Changed todos:");
        fmt::print_header(&conf.fmt, &widths);
        fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
        fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
        if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
            println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
            std::process::exit(1);
        }
    }
}

fn task_start_stop(tasks: &mut todo::TaskVec, conf: &conf::Conf, start: bool) {
    let todos = tfilter::filter(tasks, &conf.flt);
    let action = if start { "started" } else { "stopped" };
    if todos.is_empty() {
        println!("No todo {}", action)
    } else if conf.dry {
        let mut clones = todo::clone_tasks(tasks, &todos);
        let updated = if start { todo::start(&mut clones, None) } else { todo::stop(&mut clones, None) };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            println!("No todo was {}", action);
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            println!("Todos to be {}:", action);
            fmt::print_header(&conf.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
            println!("\nNew todos:");
            fmt::print_todos(&clones, &todos, &updated, &conf.fmt, &widths, true);
            fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
        }
    } else {
        let updated = if start { todo::start(tasks, Some(&todos)) } else { todo::stop(tasks, Some(&todos)) };
        let updated_cnt = calculate_updated(&updated);

        if updated_cnt == 0 {
            println!("No todo was {}", action);
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            println!("Changed todos:");
            fmt::print_header(&conf.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
            fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
                std::process::exit(1);
            }
        }
    }
}

fn task_postpone(tasks: &mut todo::TaskVec, conf: &conf::Conf) {
    let subj = match conf.todo.subject {
        Some(ref s) => s,
        None => {
            println!("Postpone range is not defined");
            return;
        }
    };
    let rec = match todotxt::Recurrence::from_str(subj) {
        Ok(r) => r,
        Err(e) => {
            println!("Invalid recurrence format: {:?}", e);
            return;
        }
    };
    let todos = tfilter::filter(tasks, &conf.flt);
    if todos.is_empty() {
        println!("No todo postponed")
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
            println!("No todo was postponed");
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            println!("Todos to be postponed:");
            fmt::print_header(&conf.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
            println!("\nNew todos:");
            fmt::print_todos(&clones, &todos, &updated, &conf.fmt, &widths, true);
            fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
        }
    } else {
        let mut updated: Vec<bool> = Vec::new();
        for idx in todos.iter() {
            if *idx >= tasks.len() || tasks[*idx].finished {
                updated.push(false);
            } else if let Some(dt) = tasks[*idx].due_date {
                tasks[*idx].due_date = Some(rec.next_date(dt));
                updated.push(true);
            } else {
                updated.push(false);
            }
        }
        let updated_cnt = calculate_updated(&updated);
        if updated_cnt == 0 {
            println!("No todo was postponed");
        } else {
            let widths = fmt::field_widths(&conf.fmt, tasks, &todos);
            println!("Changed todos:");
            fmt::print_header(&conf.fmt, &widths);
            fmt::print_todos(tasks, &todos, &updated, &conf.fmt, &widths, false);
            fmt::print_footer(tasks, &todos, &updated, &conf.fmt, &widths);
            if let Err(e) = todo::save(tasks, Path::new(&conf.todo_file)) {
                println!("Failed to save to '{:?}': {}", &conf.todo_file, e);
                std::process::exit(1);
            }
        }
    }
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

fn task_list_projects(tasks: &todo::TaskSlice, conf: &conf::Conf) {
    let todos = tfilter::filter(tasks, &conf.flt);
    // no tsort::sort() here since multiple projects in one task
    // would mess up the alphabetical output sort

    for item in collect_unique_items(tasks, &todos, |task| &task.projects) {
        println!("{}", item);
    }
}

fn task_list_contexts(tasks: &todo::TaskSlice, conf: &conf::Conf) {
    let todos = tfilter::filter(tasks, &conf.flt);
    // no tsort::sort() here since multiple contexts in one task
    // would mess up the alphabetical output sort

    for item in collect_unique_items(tasks, &todos, |task| &task.contexts) {
        println!("{}", item);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut conf = match conf::parse_args(&args) {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };

    let mut tasks: todo::TaskVec = if conf.use_done {
        match todo::load(Path::new(&conf.done_file)) {
            Ok(tlist) => tlist,
            Err(e) => {
                eprintln!("Failed to load todo list: {:?}", e);
                exit(1);
            }
        }
    } else {
        match todo::load(Path::new(&conf.todo_file)) {
            Ok(tlist) => tlist,
            Err(e) => {
                eprintln!("Failed to load done list: {:?}", e);
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

    if conf.mode != conf::RunMode::List && conf.use_done {
        eprintln!("Invalid command: when using done.txt the only available command is `list`");
        exit(1);
    }

    match conf.mode {
        conf::RunMode::Add => task_add(&mut tasks, &conf),
        conf::RunMode::List => task_list(&tasks, &conf),
        conf::RunMode::Done => task_done(&mut tasks, &conf),
        conf::RunMode::Undone => task_undone(&mut tasks, &conf),
        conf::RunMode::Remove => task_remove(&mut tasks, &conf),
        conf::RunMode::Clean => task_clean(&mut tasks, &conf),
        conf::RunMode::Edit => task_edit(&mut tasks, &conf),
        conf::RunMode::Append => task_add_text(&mut tasks, &conf, true),
        conf::RunMode::Prepend => task_add_text(&mut tasks, &conf, false),
        conf::RunMode::Start => task_start_stop(&mut tasks, &conf, true),
        conf::RunMode::Stop => task_start_stop(&mut tasks, &conf, false),
        conf::RunMode::Stats => stats::show_stats(&tasks, &conf.fmt),
        conf::RunMode::Postpone => task_postpone(&mut tasks, &conf),
        conf::RunMode::ListProjects => task_list_projects(&tasks, &conf),
        conf::RunMode::ListContexts => task_list_contexts(&tasks, &conf),
        _ => {}
    }
}
