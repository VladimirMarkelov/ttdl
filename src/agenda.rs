use std::collections::HashMap;
use std::io::{self, Write};

use chrono::{Datelike, NaiveDate, Weekday};
use termcolor::{Color, StandardStream, WriteColor};
use todo_lib::{timer, todo, todotxt};

use crate::conf;

enum SlotKind {
    None,
    Start,
    StartEarly,
    Middle,
    Finish,
    FinishLate,
}

struct TaskSlot {
    id: usize,
    kind: SlotKind,
}

struct Slot {
    time: u32,
    tasks: Vec<TaskSlot>,
}

pub struct Agenda {
    time_start: u32,
    time_end: u32,
    date: NaiveDate,
    fields: Vec<String>,
    slot_size: u64,

    slots: Vec<Slot>,
}

impl Agenda {
    pub fn new(stdout: &mut StandardStream, dt: NaiveDate, conf: &conf::Conf) -> io::Result<Self> {
        let mut ag = Agenda {
            time_start: 8*60, // 8:00
            time_end: 20*60, // 20:00
            date: dt,
            fields: vec!["due".to_string()],
            slot_size: 30, // 30 minutes
            slots: Vec::new(),
        };
        Ok(ag)
    }
}
