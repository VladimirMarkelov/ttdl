use chrono::NaiveDate;
use todo_lib::{todo, todotxt};

use crate::conf;
use crate::conv::{self, SEC_IN_MINUTE_U32};

const TIME_FIELD: &str = "time";
const ID_RESERVED: usize = 999_999_999;
const SLOT_NONE: char = ' ';
const SLOT_START: char = '┌';
const SLOT_STARTEARLY: char = '╎';
const SLOT_FINISH: char = '└';
const SLOT_FINISHLATE: char = '╎';
const SLOT_MIDDLE: char = '│';
const SLOT_SINGLE: char = '─';

#[derive(PartialEq)]
pub enum SlotKind {
    None,
    Start,
    StartEarly,
    Middle,
    Finish,
    FinishLate,
    Single,
}

impl SlotKind {
    pub fn to_char(&self) -> char {
        match self {
            SlotKind::None => SLOT_NONE,
            SlotKind::Start => SLOT_START,
            SlotKind::StartEarly => SLOT_STARTEARLY,
            SlotKind::Finish => SLOT_FINISH,
            SlotKind::FinishLate => SLOT_FINISHLATE,
            SlotKind::Middle => SLOT_MIDDLE,
            SlotKind::Single => SLOT_SINGLE,
        }
    }
    pub fn to_char_before(&self) -> char {
        match self {
            SlotKind::None => SLOT_NONE,
            SlotKind::Start => SLOT_MIDDLE,
            SlotKind::StartEarly => SLOT_STARTEARLY,
            SlotKind::Finish => SLOT_NONE,
            SlotKind::FinishLate => SLOT_FINISHLATE,
            SlotKind::Middle => SLOT_MIDDLE,
            SlotKind::Single => SLOT_NONE,
        }
    }
    pub fn to_char_after(&self) -> char {
        match self {
            SlotKind::None => SLOT_NONE,
            SlotKind::Start => SLOT_NONE,
            SlotKind::StartEarly => SLOT_NONE,
            SlotKind::Finish => SLOT_MIDDLE,
            SlotKind::FinishLate => SLOT_MIDDLE,
            SlotKind::Middle => SLOT_MIDDLE,
            SlotKind::Single => SLOT_NONE,
        }
    }
    pub fn is_start(&self) -> bool {
        *self == SlotKind::Start || *self == SlotKind::StartEarly || *self == SlotKind::Single
    }
}

pub struct TaskSlot {
    pub id: usize,
    pub kind: SlotKind,
}

impl TaskSlot {
    fn empty_slot() -> Self {
        TaskSlot { id: ID_RESERVED, kind: SlotKind::None }
    }
    pub fn is_empty(&self) -> bool {
        self.id == ID_RESERVED || self.kind == SlotKind::None
    }
}

pub struct Slot {
    pub time: u32,
    pub tasks: Vec<TaskSlot>,
}

impl Slot {
    pub fn start_cnt(&self) -> usize {
        let mut cnt = 0;
        for ts in &self.tasks {
            if ts.kind.is_start() {
                cnt += 1;
            }
        }
        cnt
    }
    pub fn nth_start(&self, n: usize) -> Option<usize> {
        let mut found = 0;
        for (idx, ts) in self.tasks.iter().enumerate() {
            if ts.kind.is_start() {
                if found == n {
                    return Some(idx);
                } else {
                    found += 1;
                }
            }
        }
        None
    }
}

pub struct Agenda {
    pub time_start: u32,
    pub time_end: u32,
    pub date: NaiveDate,
    pub fields: Vec<String>,
    pub slot_size: u32,

    pub slots: Vec<Slot>,
    pub all_day: Vec<usize>,
}

impl Agenda {
    pub fn new(dt: NaiveDate, conf: &conf::Conf) -> Self {
        let mut ag = Agenda {
            time_start: 8 * SEC_IN_MINUTE_U32, // 8:00
            time_end: 20 * SEC_IN_MINUTE_U32,  // 20:00
            date: dt,
            fields: vec!["due".to_string()],
            slot_size: 30, // 30 minutes
            slots: Vec::new(),
            all_day: Vec::new(),
        };
        if let Some(s) = &conf.time_range {
            match conv::str_to_time_interval(s) {
                conv::TimeInterval::Single(val) => {
                    if let Some(t) = val {
                        ag.time_start = t / 100 * SEC_IN_MINUTE_U32 + t % 100;
                    }
                }
                conv::TimeInterval::Range(valb, vale) => {
                    if let Some(st) = valb {
                        ag.time_start = st / 100 * SEC_IN_MINUTE_U32 + st % 100;
                    } else {
                        ag.time_start = 0;
                    }
                    if let Some(en) = vale {
                        ag.time_end = en / 100 * SEC_IN_MINUTE_U32 + en % 100;
                    } else {
                        ag.time_end = 24 * SEC_IN_MINUTE_U32;
                    }
                }
            }
        }
        if let Some(d_str) = &conf.slot {
            if d_str.find(|c: char| !c.is_ascii_digit()).is_none() {
                match d_str.parse::<u32>() {
                    Err(_) => {
                        eprintln!("Slot must be a positive integer number: '{d_str}'");
                    }
                    Ok(n) => {
                        if !(15..24 * SEC_IN_MINUTE_U32).contains(&n) {
                            eprintln!(
                                "Slot value must be between 15 minutes and 24 hours: '{d_str}'. Using default slot size"
                            );
                        } else {
                            ag.slot_size = n;
                        }
                    }
                }
            } else {
                match conv::str_to_duration(d_str) {
                    None => {
                        eprintln!("Failed to parse duration '{d_str}'");
                    }
                    Some(dur) => {
                        let dur = (dur as u32) / SEC_IN_MINUTE_U32;
                        if !(15..24 * SEC_IN_MINUTE_U32).contains(&dur) {
                            eprintln!(
                                "Slot value must be between 15 minutes and 24 hours: '{d_str}'. Using default slot size"
                            );
                        } else {
                            ag.slot_size = dur;
                        }
                    }
                }
            }
        }
        if let Some(on_date) = &conf.on {
            if let Some((s_fields, s_date)) = on_date.split_once('=') {
                if !s_date.is_empty() {
                    match conv::str_to_date(s_date, dt) {
                        Some(d) => ag.date = d,
                        None => eprintln!("Failed to parse date: {s_date}"),
                    }
                }
                if !s_fields.is_empty() {
                    ag.fields.clear();
                    for f in s_fields.split(',') {
                        ag.fields.push(f.to_string());
                    }
                }
            } else if let Some(dval) = conv::str_to_date(on_date, dt) {
                ag.date = dval;
            } else {
                ag.fields.clear();
                for f in on_date.split(',') {
                    ag.fields.push(f.to_string());
                }
            };
        }
        let mut c_time = ag.time_start;
        while c_time <= ag.time_end {
            let slot = Slot { time: c_time, tasks: Vec::new() };
            ag.slots.push(slot);
            c_time += ag.slot_size;
        }
        ag
    }

    fn task_date(&self, task: &todotxt::Task, today: NaiveDate) -> Option<NaiveDate> {
        for field in &self.fields {
            let val = match field.as_str() {
                "due" => task.due_date,
                "created" => task.create_date,
                "threshold" | "thr" => task.threshold_date,
                fname => task.tags.get(fname).and_then(|fval| conv::str_to_date(fval, today)),
            };
            if val.is_some() {
                return val;
            }
        }
        None
    }

    fn task_time(&self, task: &todotxt::Task) -> Option<(u32, u32)> {
        let range = task.tags.get(TIME_FIELD).map(|sval| conv::str_to_time_interval(sval));
        match range {
            Some(conv::TimeInterval::Single(s)) => s.map(|sval| {
                let st = sval / 100 * SEC_IN_MINUTE_U32 + sval % 100;
                let en = st + self.slot_size;
                (st, en)
            }),
            Some(conv::TimeInterval::Range(sb, se)) => {
                let st = if let Some(v) = sb { v / 100 * SEC_IN_MINUTE_U32 + v % 100 } else { 0 };
                let en = if let Some(v) = se { v / 100 * SEC_IN_MINUTE_U32 + v % 100 } else { 24 * SEC_IN_MINUTE_U32 };
                Some((st, en))
            }
            None => None,
        }
    }

    fn unoccupied_column(&self, slot_st: usize, slot_en: usize, max_col: usize) -> usize {
        for i in 0..max_col {
            let mut occupied = false;
            for idx in slot_st..=slot_en {
                if self.slots[idx].tasks.len() > i && !self.slots[idx].tasks[i].is_empty() {
                    occupied = true;
                    break;
                }
            }
            if !occupied {
                return i;
            }
        }
        ID_RESERVED
    }

    fn fill_column(&mut self, slot_st: usize, slot_en: usize, col: usize, task_id: usize, time_st: u32, time_en: u32) {
        for slot_id in slot_st..=slot_en {
            while self.slots[slot_id].tasks.len() < col {
                self.slots[slot_id].tasks.push(TaskSlot::empty_slot());
            }
            let new_slot = TaskSlot {
                kind: if slot_id == 0 {
                    if self.slots[slot_id].time > time_st {
                        SlotKind::StartEarly
                    } else if slot_st == slot_en {
                        SlotKind::Single
                    } else {
                        SlotKind::Start
                    }
                } else if slot_id == self.slots.len() - 1 {
                    if slot_id == slot_st {
                        SlotKind::Start
                    } else if self.slots[slot_id].time + self.slot_size < time_en {
                        SlotKind::FinishLate
                    } else if slot_st == slot_en {
                        SlotKind::Single
                    } else {
                        SlotKind::Finish
                    }
                } else if slot_st == slot_en {
                    SlotKind::Single
                } else if slot_id == slot_st {
                    SlotKind::Start
                } else if slot_id == slot_en {
                    SlotKind::Finish
                } else {
                    SlotKind::Middle
                },
                id: task_id,
            };
            if self.slots[slot_id].tasks.len() == col {
                self.slots[slot_id].tasks.push(new_slot);
            } else {
                self.slots[slot_id].tasks[col] = new_slot;
            }
        }
    }

    fn add_task_to_agenda(&mut self, time_start: u32, time_end: u32, id: usize) {
        if self.slots.is_empty() {
            self.all_day.push(id);
            return;
        }
        let slot_st =
            if time_start >= self.time_start { ((time_start - self.time_start) / self.slot_size) as usize } else { 0 };
        let slot_en = {
            let en = ((time_end - self.time_start) / self.slot_size) as usize;
            if en >= self.slots.len() { self.slots.len() - 1 } else { en }
        };
        let mut max_cols = 0usize;
        let mut min_cols = ID_RESERVED;
        for slot in &self.slots {
            let l = slot.tasks.len();
            if l < min_cols {
                min_cols = l;
            }
            if l > max_cols {
                max_cols = l;
            }
        }
        let mut col = self.unoccupied_column(slot_st, slot_en, max_cols);
        if col == ID_RESERVED {
            col = max_cols;
        }
        self.fill_column(slot_st, slot_en, col, id, time_start, time_end);
    }

    pub fn fill_agenda(&mut self, tasks: &todo::TaskSlice, ids: &todo::IDSlice, today: NaiveDate) {
        for tid in ids {
            let task_date = self.task_date(&tasks[*tid], today);
            if let Some(dt) = task_date {
                if dt != self.date {
                    continue;
                }
            } else {
                continue;
            }
            if let Some((task_st, task_en)) = self.task_time(&tasks[*tid]) {
                if task_st > self.time_end || task_en < self.time_start {
                    continue;
                }
                self.add_task_to_agenda(task_st, task_en, *tid);
            } else {
                self.all_day.push(*tid);
            }
        }
    }

    pub fn max_columns(&self) -> usize {
        let mut max = 0;
        for slot in &self.slots {
            if slot.tasks.len() > max {
                max = slot.tasks.len();
            }
        }
        max
    }
}
