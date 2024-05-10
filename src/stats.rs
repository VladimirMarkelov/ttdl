use std::cmp::Ordering;
use std::io::{self, Write};

use crate::fmt;
use caseless::default_caseless_match_str;
use termcolor::StandardStream;
use todo_lib::*;

const TOTAL: &str = "";
const PRJ_WIDTH: usize = 15;
const CTX_WIDTH: usize = 10;
const NUM_WIDTH: usize = 10;

fn is_task_overdue(task: &todotxt::Task, today: chrono::NaiveDate) -> bool {
    match task.due_date {
        None => false,
        Some(d) => {
            if let Some(f) = task.finish_date {
                d > f
            } else {
                !task.finished && d < today
            }
        }
    }
}

pub fn show_stats(stdout: &mut StandardStream, tasks: &todo::TaskSlice, conf: &fmt::Conf) -> io::Result<()> {
    show_short_stats(stdout, tasks)?;
    if conf.fmt != fmt::Format::Short {
        writeln!(stdout)?;
        show_full_stats(stdout, tasks)?;
    }
    Ok(())
}

fn show_short_stats(stdout: &mut StandardStream, tasks: &todo::TaskSlice) -> io::Result<()> {
    let mut overdue: usize = 0;
    let mut recurrent: usize = 0;
    let mut done: usize = 0;
    let today = chrono::Utc::now().naive_utc().date();

    for t in tasks.iter() {
        if is_task_overdue(t, today) {
            overdue += 1;
        }
        if t.finished {
            done += 1;
            continue;
        }
        if t.recurrence.is_some() {
            recurrent += 1;
        }
    }

    let width = 19;
    let length = tasks.len();
    writeln!(stdout, "{:wid$}{:4}", "Total todos:", length, wid = width)?;
    if done > 0 {
        writeln!(stdout, "{:wid$}{:>4} ({}%)", "Done:", done, done * 100 / length, wid = width)?;
    }
    if overdue > 0 {
        writeln!(stdout, "{:wid$}{:4} ({}%)", "Overdue:", overdue, overdue * 100 / length, wid = width)?;
    }
    if recurrent > 0 {
        writeln!(stdout, "{:wid$}{:4}", "Recurrent:", recurrent, wid = width)?;
    }
    Ok(())
}

struct Stat {
    prj: String,
    ctx: String,
    total: usize,
    done: usize,
    overdue: usize,
    spent: chrono::Duration,
}
struct Stats {
    stats: Vec<Stat>,
}
impl Stats {
    fn update_stat(&mut self, prj: &str, ctx: &str, done: bool, overdue: bool, spent: chrono::Duration) {
        let mut found = false;
        for t in self.stats.iter_mut() {
            if default_caseless_match_str(&t.prj, prj) && default_caseless_match_str(&t.ctx, ctx) {
                t.total += 1;
                if done {
                    t.done += 1;
                }
                if overdue {
                    t.overdue += 1;
                }
                t.spent += spent;
                found = true;
            }
        }

        if !found {
            let s = Stat {
                prj: prj.to_lowercase(),
                ctx: ctx.to_lowercase(),
                total: 1,
                done: usize::from(done),
                overdue: usize::from(overdue),
                spent,
            };
            self.stats.push(s);
        }
    }
    fn sort(&mut self) {
        self.stats.sort_by(|a, b| {
            if a.prj == TOTAL {
                return Ordering::Less;
            }
            if b.prj == TOTAL {
                return Ordering::Greater;
            }

            let c = a.prj.to_lowercase().cmp(&b.prj.to_lowercase());
            if c != Ordering::Equal {
                return c;
            }
            if a.ctx == TOTAL {
                return Ordering::Less;
            }
            if b.ctx == TOTAL {
                return Ordering::Greater;
            }
            a.ctx.to_lowercase().cmp(&b.ctx.to_lowercase())
        });
    }
}

fn show_full_stats(stdout: &mut StandardStream, tasks: &todo::TaskSlice) -> io::Result<()> {
    let mut st = Stats { stats: Vec::new() };
    let today = chrono::Utc::now().naive_utc().date();

    for t in tasks.iter() {
        let done = t.finished;
        let overdue = is_task_overdue(t, today);
        let spent = timer::spent_time(t);
        st.update_stat(TOTAL, TOTAL, done, overdue, spent);
        for p in t.projects.iter() {
            st.update_stat(p, TOTAL, done, overdue, spent);
            for c in t.contexts.iter() {
                st.update_stat(p, c, done, overdue, spent);
            }
        }
    }

    st.sort();
    let mut last_prj = "".to_string();
    let mut proj_total: usize = 0;

    let length = tasks.len();
    let header = format!(
        "{:pw$} {:cw$} {:nw$} {:nw$} {:nw$} {}",
        "Project",
        "Context",
        "Total",
        "Done",
        "Overdue",
        "Spent",
        pw = PRJ_WIDTH,
        cw = CTX_WIDTH,
        nw = NUM_WIDTH
    );
    writeln!(stdout, "{header}")?;
    let sep = "-".repeat(header.len());

    for s in st.stats.iter() {
        let prj = if s.prj == last_prj {
            String::new()
        } else {
            writeln!(stdout, "{sep}")?;
            last_prj.clone_from(&s.prj);
            proj_total = s.total;
            if s.prj.len() > PRJ_WIDTH {
                format!("{:.w$}", s.prj, w = PRJ_WIDTH)
            } else {
                s.prj.clone()
            }
        };
        write!(stdout, "{prj:PRJ_WIDTH$} ")?;

        let ctx = if s.ctx.len() > CTX_WIDTH { format!("{:.w$}", s.ctx, w = CTX_WIDTH) } else { s.ctx.clone() };
        write!(stdout, "{ctx:CTX_WIDTH$} ")?;

        let div_by = if s.ctx.is_empty() { length } else { proj_total };
        let sn = format!("{}({:>3}%)", s.total, s.total * 100 / div_by);
        write!(stdout, "{:>w$} ", &sn, w = NUM_WIDTH)?;

        let sn = if s.done == 0 { String::new() } else { format!("{}({:>3}%)", s.done, s.done * 100 / s.total) };
        write!(stdout, "{:>w$} ", &sn, w = NUM_WIDTH)?;

        let sn =
            if s.overdue == 0 { String::new() } else { format!("{}({:>3}%)", s.overdue, s.overdue * 100 / s.total) };
        write!(stdout, "{:>w$} ", &sn, w = NUM_WIDTH)?;

        if s.spent.num_seconds() == 0 {
            writeln!(stdout)?;
        } else {
            writeln!(stdout, "{}", &fmt::duration_str(s.spent))?;
        }
    }
    Ok(())
}
