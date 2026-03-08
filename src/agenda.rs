use chrono::NaiveDate;
use todo_lib::{todo, todotxt};

use crate::conf;
use crate::conv::{self, SEC_IN_MINUTE_U32};

// Use this field to decide to what time slot in an agenda a task beholds
const TIME_FIELD: &str = "time";
// A placeholder to mark time slots empty
const ID_RESERVED: usize = 999_999_999;

// Indexes in `agenda.marks` vector of various marks used when displaying an agenda.
// See enum SlotKind for detailed explanation.
const SLOT_START: usize = 0;
const SLOT_START_EARLY: usize = 3;
pub const SLOT_FINISH: usize = 2;
const SLOT_FINISH_LATE: usize = 4;
const SLOT_MIDDLE: usize = 1;
const SLOT_SINGLE: usize = 5;
const SLOT_UNLIMITED: usize = 6;
const SLOT_START_AFTER: usize = 7;
const SLOT_FINISH_BEFORE: usize = 8;
// It must the greatest index. So, adjust this constant if a new marker is added.
pub const SLOT_NONE: usize = 9;

// Agenda starts at 8:00
const DEFAULT_AGENDA_START: u32 = 8 * SEC_IN_MINUTE_U32;
// Agenda ends at 20:00
const DEFAULT_AGENDA_END: u32 = 20 * SEC_IN_MINUTE_U32;
// Default agenda slot time size - 30 minutes
const DEFAULT_SLOT_SIZE: u32 = 30;
// Minimal time slot size is 15 minutes
const MIN_SLOT_SIZE: u32 = 15;
// The end of the day 24:00 (or 0:00, or 12:00AM)
const DAY_END: u32 = 24 * SEC_IN_MINUTE_U32;
// Default set of characters to draw the outline. Charater indices:
//   0 - Task starts
//   1 - Task continues
//   2 - Task finishes
//   3 - Task has started before the agenda time range (e.g., task has `time:700-900` and you
//       display agenda for 800-1200)
//   4 - Task finishes after the agenda time range ends
//   5 - Task is shorter than the time slot. It stars and ends within a single time slot
//   6 - Task is "unlimited", i.e, it has start time but does not have end time (or end time is
//       earlier than the start time)
//   7 - Task starts after the start of its first time slot  and the task is longer than 1 time slot
//   8 - Task ends before the end of its last time slot  and the task is longer than 1 time slot
const DEFAULT_MARKS: &str = "┌│└╎╎─[┬┴";

// Covert time in format "{hour}{zero-padded-minutes}" into the number of minutes since midnight.
// E.g, "810" (that is 8:00) to 490 minutes.
fn time_to_minutes(t: u32) -> u32 {
    t / 100 * SEC_IN_MINUTE_U32 + t % 100
}

#[derive(PartialEq, Debug)]
pub enum SlotKind {
    // Empty slot
    None,
    // Task origin
    Start,
    // A task was started before the agenda time range starts, so the task origin is not displayed
    StartEarly,
    // A task goes on
    Middle,
    // Task finish
    Finish,
    // A task finishes aftet the agenda ends, so the end of the task is not displayed
    FinishLate,
    // A task that is shorter than a time slot
    Single,
    // Unlimited - a task that does not have the end time, e.g `time:1000`
    Unlimited,
    // A task starts after its time slot starts
    StartAfter,
    // A task end before its time slot ends
    FinishBefore,
}

impl SlotKind {
    // Returns a char index in agenda.marks vector for case when an agenda line is not duplicated.
    // Duplication happens when more than 1 task starts at the same time (or more than one task
    // starts before the agenda time range starts)
    pub fn to_char_index(&self) -> usize {
        match self {
            SlotKind::None => SLOT_NONE,
            SlotKind::Start => SLOT_START,
            SlotKind::StartEarly => SLOT_START_EARLY,
            SlotKind::Finish => SLOT_FINISH,
            SlotKind::FinishLate => SLOT_FINISH_LATE,
            SlotKind::Middle => SLOT_MIDDLE,
            SlotKind::Single => SLOT_SINGLE,
            SlotKind::Unlimited => SLOT_UNLIMITED,
            SlotKind::StartAfter => SLOT_START_AFTER,
            SlotKind::FinishBefore => SLOT_FINISH_BEFORE,
        }
    }
    // When lines are duplicated, to avoid confusion, it is critical to display easy to read
    // information. So, TTDL prints duplicated lines one by one, and each new line starts a new
    // tasks. The task displayed before the current task must get `before_index` to display the
    // correct symbol for the agenda outline.
    pub fn to_char_before_index(&self) -> usize {
        match self {
            SlotKind::None => SLOT_NONE,
            SlotKind::Start => SLOT_MIDDLE,
            SlotKind::StartEarly => SLOT_START_EARLY,
            SlotKind::Finish => SLOT_NONE,
            SlotKind::FinishLate => SLOT_FINISH_LATE,
            SlotKind::Middle => SLOT_MIDDLE,
            SlotKind::Single => SLOT_NONE,
            SlotKind::Unlimited => SLOT_NONE,
            SlotKind::StartAfter => SLOT_MIDDLE,
            SlotKind::FinishBefore => SLOT_MIDDLE,
        }
    }
    // When lines are duplicated, to avoid confusion, it is critical to display easy to read
    // information. So, TTDL prints duplicated lines one by one, and each new line starts a new
    // tasks. The task displayed after the current task must get `after_index` to display the
    // correct symbol for the agenda outline.
    pub fn to_char_after_index(&self) -> usize {
        match self {
            SlotKind::None => SLOT_NONE,
            SlotKind::Start => SLOT_NONE,
            SlotKind::StartEarly => SLOT_NONE,
            SlotKind::Finish => SLOT_MIDDLE,
            SlotKind::FinishLate => SLOT_MIDDLE,
            SlotKind::Middle => SLOT_MIDDLE,
            SlotKind::Single => SLOT_NONE,
            SlotKind::Unlimited => SLOT_NONE,
            SlotKind::StartAfter => SLOT_NONE,
            SlotKind::FinishBefore => SLOT_MIDDLE,
        }
    }
    // Returns true if the SlotKind means the start has just started (i.e, it is the first line
    // when the task appears in the outline)
    pub fn is_start(&self) -> bool {
        *self == SlotKind::Start
            || *self == SlotKind::StartEarly
            || *self == SlotKind::Single
            || *self == SlotKind::Unlimited
            || *self == SlotKind::StartAfter
    }
}

// A task slot
pub struct TaskSlot {
    // Unique task ID from the list of tasks
    pub id: usize,
    // What mark should be drawn in the agenda outline for this task at the current time slot
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

// An agenda slot
pub struct Slot {
    // The start time of the slot
    pub time: u32,
    // The list of tasks that starts, finishes or continues during this time slot
    pub tasks: Vec<TaskSlot>,
}

impl Slot {
    // Returns how many tasks start during this time slot.
    pub fn start_cnt(&self) -> usize {
        let mut cnt = 0;
        for ts in &self.tasks {
            if ts.kind.is_start() {
                cnt += 1;
            }
        }
        cnt
    }
    // Returns the index of the nth task that starts during this time slot
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

// Holds temporary information to draw the agenda
pub struct Agenda {
    // Agenda start
    pub time_start: u32,
    // Agenda end
    pub time_end: u32,
    // For what date show the agenda
    pub date: NaiveDate,
    // What tags to check for the date. Default is `due`
    pub fields: Vec<String>,
    // The slot size (a slot is a single line in the agenda output)
    pub slot_size: u32,

    // All, including empty ones, time slots for the agenda
    pub slots: Vec<Slot>,
    // Tasks that have 'fields' value equals selected day, but these tasks miss tag `time`, so they
    // are treated as if they last all the day. To avoid cluttering, the tasks are displayed at the
    // bottom after the agenda outline is printed.
    pub all_day: Vec<usize>,

    // What symbols to use when printing the agenda outline
    marks: Vec<char>,
}

impl Agenda {
    pub fn new(dt: NaiveDate, conf: &conf::Conf) -> Self {
        let mut ag = Agenda {
            time_start: DEFAULT_AGENDA_START,
            time_end: DEFAULT_AGENDA_END,
            date: dt,
            fields: Vec::new(),
            slot_size: DEFAULT_SLOT_SIZE,
            slots: Vec::new(),
            all_day: Vec::new(),
            marks: DEFAULT_MARKS.chars().collect(),
        };
        if let Some(s) = &conf.marks {
            ag.marks = s.chars().collect();
            if ag.marks.len() < SLOT_NONE {
                ag.marks.push('┬');
                ag.marks.push('┴');
            }
        }
        // Slot size must be parsed before the parsing time range as time range uses the slot size
        if let Some(d_str) = &conf.slot {
            if d_str.find(|c: char| !c.is_ascii_digit()).is_none() {
                match d_str.parse::<u32>() {
                    Err(_) => {
                        eprintln!("Slot must be a positive integer number: '{d_str}'");
                    }
                    Ok(n) => {
                        if !(MIN_SLOT_SIZE..DAY_END).contains(&n) {
                            eprintln!(
                                "Slot value must be between {MIN_SLOT_SIZE} minutes and 24 hours: '{d_str}'. Using default slot size"
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
                        if !(MIN_SLOT_SIZE..DAY_END).contains(&dur) {
                            eprintln!(
                                "Slot value must be between {MIN_SLOT_SIZE} minutes and 24 hours: '{d_str}'. Using default slot size"
                            );
                        } else {
                            ag.slot_size = dur;
                        }
                    }
                }
            }
        }
        // Time range must be parsed after parsing the slot size as time range uses the slot size
        if let Some(s) = &conf.time_range {
            match conv::str_to_time_interval(s) {
                conv::TimeInterval::Single(val) => {
                    if let Some(t) = val {
                        ag.time_start = time_to_minutes(t);
                    }
                }
                conv::TimeInterval::Range(valb, vale) => {
                    if let Some(st) = valb {
                        ag.time_start = time_to_minutes(st);
                    } else {
                        ag.time_start = 0;
                    }
                    if let Some(en) = vale {
                        ag.time_end = time_to_minutes(en);
                    } else {
                        ag.time_end = DAY_END;
                    }
                }
            }
            if ag.time_start >= ag.time_end {
                eprintln!("Agenda start time is greater than the agenda end time: {s}. Reset values to the defaults");
                ag.time_start = DEFAULT_AGENDA_START;
                ag.time_end = DEFAULT_AGENDA_END;
            } else if ag.time_end - ag.time_start < ag.slot_size {
                eprintln!(
                    "Too small difference between start and end time: {s}. It should be at least a slot size '{0}'. Update the end time",
                    ag.slot_size
                );
                ag.time_end = ag.time_start + ag.slot_size;
            }
        }
        // This must be done before parsing `conf.on`. See the next code block
        match &conf.on_fields {
            None => ag.fields = vec!["due".to_string()],
            Some(s) => {
                for f in s.split(',') {
                    ag.fields.push(f.to_string());
                }
            }
        };
        // Consuming `conf.on` must following consuming `conf.on_fields` because `on` must override
        // `on_fields`. Reason: `on` is passed in command-line, so it has higher priority.
        if let Some(on_date) = &conf.on {
            if let Some((s_fields, s_date)) = on_date.split_once('=') {
                // The value is a range (i.e, two values separated with '=')
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
                // The value is a single word that looks like date. E.g `2025-10-11` or `today`
                ag.date = dval;
            } else {
                // The value is a single and cannot be converted to a date. Consider it a list of
                // tag names separated with a comma
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

    // Determines task's date for the agenda.
    // If more than one field is defined for this purpose, the TTDL gets the first non-empty tag.
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

    // Determines task's time for the agenda.
    // - Time can be a single value: `1100`. In this case, the slot_size is used to calculate the
    //   end task time
    // - Time can be range, even with an open end: `1100-1300` or `-1400`(from 0:00 to 14:00)
    fn task_time(&self, task: &todotxt::Task) -> Option<(u32, u32)> {
        let range = task.tags.get(TIME_FIELD).map(|sval| conv::str_to_time_interval(sval));
        match range {
            Some(conv::TimeInterval::Single(s)) => s.map(|sval| {
                let st = time_to_minutes(sval);
                let en = if st > 0 { st - 1 } else { 0 };
                (st, en)
            }),
            Some(conv::TimeInterval::Range(sb, se)) => {
                let st = if let Some(v) = sb { time_to_minutes(v) } else { 0 };
                let en = if let Some(v) = se { time_to_minutes(v) } else { DAY_END };
                Some((st, en))
            }
            None => None,
        }
    }

    // Finds a vertical gap in the outline where all slots from slot_st to slot_en are empty.
    // Used to draw as compact outline as possible.
    // Returns ID_RESERVED if no unoccupied ranges are found. This means the caller must add a new
    // column.
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

    fn kind_for_first_slot(
        &self,
        slot_id: usize,
        slot_st: usize,
        slot_en: usize,
        time_st: u32,
        time_en: u32,
    ) -> SlotKind {
        let slot_time = self.slots[slot_id].time;
        if self.slots[slot_id].time > time_st {
            SlotKind::StartEarly
        } else if slot_st == slot_en && time_en > time_st {
            SlotKind::Single
        } else if slot_st == slot_en {
            SlotKind::Unlimited
        } else if slot_id == slot_st && slot_time == time_st {
            SlotKind::Start
        } else {
            SlotKind::StartAfter
        }
    }

    fn kind_for_middle_slot(
        &self,
        slot_id: usize,
        slot_st: usize,
        slot_en: usize,
        time_st: u32,
        time_en: u32,
    ) -> SlotKind {
        let slot_time = self.slots[slot_id].time;
        if slot_st == slot_en && time_en > time_st {
            SlotKind::Single
        } else if slot_st == slot_en {
            SlotKind::Unlimited
        } else if slot_id == slot_st && slot_time == time_st {
            SlotKind::Start
        } else if slot_id == slot_st {
            SlotKind::StartAfter
        } else if slot_id == slot_en && slot_time == time_en {
            SlotKind::Finish
        } else if slot_id == slot_en {
            SlotKind::FinishBefore
        } else {
            SlotKind::Middle
        }
    }

    fn kind_for_last_slot(
        &self,
        slot_id: usize,
        slot_st: usize,
        slot_en: usize,
        time_st: u32,
        time_en: u32,
    ) -> SlotKind {
        let slot_time = self.slots[slot_id].time;
        if slot_id == slot_st && slot_time == time_st {
            SlotKind::Start
        } else if slot_id == slot_st {
            SlotKind::StartAfter
        } else if self.slots[slot_id].time < time_en {
            SlotKind::FinishLate
        } else if slot_st == slot_en && time_en > time_st {
            SlotKind::Single
        } else if slot_st == slot_en {
            SlotKind::Unlimited
        } else if slot_time == time_en {
            SlotKind::Finish
        } else {
            SlotKind::FinishBefore
        }
    }

    // Fill the outline using the task data. The most important part of the function is to
    // determine what task state is in every slot.
    fn fill_column(&mut self, slot_st: usize, slot_en: usize, col: usize, task_id: usize, time_st: u32, time_en: u32) {
        for slot_id in slot_st..=slot_en {
            while self.slots[slot_id].tasks.len() < col {
                self.slots[slot_id].tasks.push(TaskSlot::empty_slot());
            }
            let new_slot = TaskSlot {
                kind: if slot_id == 0 {
                    self.kind_for_first_slot(slot_id, slot_st, slot_en, time_st, time_en)
                } else if slot_id == self.slots.len() - 1 {
                    self.kind_for_last_slot(slot_id, slot_st, slot_en, time_st, time_en)
                } else {
                    self.kind_for_middle_slot(slot_id, slot_st, slot_en, time_st, time_en)
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

    // Fill in the column of an agenda with the task.
    fn add_task_to_agenda(&mut self, time_start: u32, time_end: u32, id: usize) {
        if self.slots.is_empty() {
            self.all_day.push(id);
            return;
        }
        let time_end = if time_end < time_start { time_start } else { time_end };
        let start_diff = time_start.saturating_sub(self.time_start);
        let end_diff = if time_end < self.time_start { 0 } else { time_end - self.time_start };
        let slot_st = if time_start >= self.time_start { (start_diff / self.slot_size) as usize } else { 0 };
        let slot_start_at = slot_st as u32 * self.slot_size + self.time_start;
        let mut slot_en = {
            let en = (end_diff / self.slot_size) as usize;
            if en >= self.slots.len() { self.slots.len() - 1 } else { en }
        };
        // If a task spans over more than 1 time slot and its end time is in the middle of the last
        // time slot, increase the last occupied slot index.
        // Example: `time:1350-1410` with time slot `30` will be drawn in the outline slots:
        //      1330, 1400, and 1430.
        //      But `time:1400-1410` will be drawn in a single time slot 1400 with a special
        //      character.
        if (time_start < slot_start_at || time_end >= (slot_start_at + self.slot_size))
            && slot_en < self.slots.len() - 1
            && !end_diff.is_multiple_of(self.slot_size)
        {
            slot_en += 1;
        }
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

    // Add all tasks from `ids` to the agenda if there dates match the requried one.
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
                let unlimited = task_st > task_en;
                let before = !unlimited && (task_en < self.time_start);
                let after = task_st > self.time_end;
                if before || after {
                    continue;
                }
                self.add_task_to_agenda(task_st, task_en, *tid);
            } else {
                self.all_day.push(*tid);
            }
        }
    }

    // How many columns the agenda outline contains.
    pub fn max_columns(&self) -> usize {
        let mut max = 0;
        for slot in &self.slots {
            if slot.tasks.len() > max {
                max = slot.tasks.len();
            }
        }
        max
    }

    // Returns a character for a certain task slot kind.
    pub fn mark(&self, idx: usize) -> char {
        if idx >= SLOT_NONE { ' ' } else { self.marks[idx] }
    }
}
