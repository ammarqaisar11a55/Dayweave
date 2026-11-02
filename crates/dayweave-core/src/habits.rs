use std::collections::{BTreeMap, HashMap};

use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::clock::{format_date, format_instant, parse_date, parse_time, Clock};
use crate::error::{Error, Result};
use crate::settings;
use crate::streaks::{DayState, Engine, Entry, Kind, Schedule, Spec, Stats};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    #[serde(default)]
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayEntry {
    pub state: DayState,
    pub due: bool,
    pub value: Option<f64>,
    pub checked_items: Vec<i64>,
    pub note: String,
    pub slipped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Habit {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    pub kind: String,
    pub frequency: String,
    pub weekdays: Vec<u8>,
    pub weekly_target: Option<u32>,
    pub start_date: String,
    pub reminder_time: Option<String>,
    pub quantity_target: Option<f64>,
    pub unit: String,
    pub position: i64,
    pub archived_at: Option<String>,
    pub checklist: Vec<ChecklistItem>,
    pub today: TodayEntry,
    pub stats: Stats,
    /// The last seven days, oldest first, for the small history strip on a card.
    pub recent: Vec<RecentDay>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDay {
    pub date: String,
    pub state: DayState,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HabitInput {
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub kind: Option<String>,
    pub frequency: Option<String>,
    pub weekdays: Vec<u8>,
    pub weekly_target: Option<u32>,
    pub start_date: Option<String>,
    pub reminder_time: Option<String>,
    pub quantity_target: Option<f64>,
    pub unit: String,
    pub checklist: Vec<ChecklistItem>,
}

struct Base {
    id: i64,
    name: String,
    description: String,
    icon: String,
    color: String,
    kind: String,
    frequency: String,
    weekly_target: Option<u32>,
    start_date: String,
    reminder_time: Option<String>,
    quantity_target: Option<f64>,
    unit: String,
    position: i64,
    archived_at: Option<String>,
}

const SELECT: &str = "SELECT id, name, description, icon, color, kind, frequency, weekly_target,
    start_date, reminder_time, quantity_target, unit, position, archived_at FROM habits";

fn map_base(row: &Row) -> rusqlite::Result<Base> {
    Ok(Base {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        icon: row.get(3)?,
        color: row.get(4)?,
        kind: row.get(5)?,
        frequency: row.get(6)?,
        weekly_target: row.get(7)?,
        start_date: row.get(8)?,
        reminder_time: row.get(9)?,
        quantity_target: row.get(10)?,
        unit: row.get(11)?,
        position: row.get(12)?,
        archived_at: row.get(13)?,
    })
}

fn parse_kind(text: &str) -> Kind {
    match text {
        "quit" => Kind::Quit,
        "quantity" => Kind::Quantity,
        "checklist" => Kind::Checklist,
        _ => Kind::Build,
    }
}

struct LogRow {
    status: String,
    value: Option<f64>,
    checked_items: Vec<i64>,
    note: String,
}

fn load_log(conn: &Connection, id: i64) -> Result<BTreeMap<NaiveDate, LogRow>> {
    let mut statement = conn.prepare(
        "SELECT date, status, value, checked_items, note FROM habit_completions WHERE habit_id = ?1",
    )?;
    let rows = statement.query_map([id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            LogRow {
                status: row.get(1)?,
                value: row.get(2)?,
                checked_items: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                note: row.get(4)?,
            },
        ))
    })?;
    let mut log = BTreeMap::new();
    for row in rows {
        let (date, entry) = row?;
        log.insert(parse_date(&date)?, entry);
    }
    Ok(log)
}

fn entries_of(log: &BTreeMap<NaiveDate, LogRow>) -> BTreeMap<NaiveDate, Entry> {
    log.iter()
        .map(|(date, row)| {
            let entry = match row.status.as_str() {
                "done" => Entry::Done,
                "slip" => Entry::Slip,
                _ => Entry::Partial,
            };
            (*date, entry)
        })
        .collect()
}

fn load_weekdays(conn: &Connection, id: i64) -> Result<Vec<u8>> {
    Ok(conn
        .prepare("SELECT weekday FROM habit_schedules WHERE habit_id = ?1 ORDER BY weekday")?
        .query_map([id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

fn spec_of(conn: &Connection, base: &Base, weekdays: &[u8]) -> Result<Spec> {
    let schedule = match (base.frequency.as_str(), base.weekly_target) {
        ("weekly_target", Some(target)) => Schedule::WeeklyTarget(target),
        _ => {
            let mut days = [false; 7];
            for day in weekdays {
                days[*day as usize] = true;
            }
            Schedule::Days(days)
        }
    };
    Ok(Spec {
        kind: parse_kind(&base.kind),
        schedule,
        start: parse_date(&base.start_date)?,
        week_start: settings::week_start(conn)?,
    })
}

fn assemble(conn: &Connection, clock: &dyn Clock, base: Base) -> Result<Habit> {
    let today = clock.today();
    let weekdays = load_weekdays(conn, base.id)?;
    let spec = spec_of(conn, &base, &weekdays)?;
    let log = load_log(conn, base.id)?;
    let entries = entries_of(&log);
    let engine = Engine::new(&spec, &entries, today);

    let checklist = conn
        .prepare("SELECT id, title FROM habit_checklist_items WHERE habit_id = ?1 ORDER BY position, id")?
        .query_map([base.id], |row| Ok(ChecklistItem { id: row.get(0)?, title: row.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;

    let entry = log.get(&today);
    let state = engine.state(today);
    let recent = (0..7)
        .rev()
        .map(|offset| {
            let date = today - Duration::days(offset);
            RecentDay { date: format_date(date), state: engine.state(date) }
        })
        .collect();
    let stats = engine.stats();
    Ok(Habit {
        today: TodayEntry {
            state,
            due: state == DayState::Pending
                || (state != DayState::Inactive && state != DayState::NotScheduled),
            value: entry.and_then(|row| row.value),
            checked_items: entry.map(|row| row.checked_items.clone()).unwrap_or_default(),
            note: entry.map(|row| row.note.clone()).unwrap_or_default(),
            slipped: entry.is_some_and(|row| row.status == "slip"),
        },
        stats,
        recent,
        id: base.id,
        name: base.name,
        description: base.description,
        icon: base.icon,
        color: base.color,
        kind: base.kind,
        frequency: base.frequency,
        weekdays,
        weekly_target: base.weekly_target,
        start_date: base.start_date,
        reminder_time: base.reminder_time,
        quantity_target: base.quantity_target,
        unit: base.unit,
        position: base.position,
        archived_at: base.archived_at,
        checklist,
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListParams {
    pub include_archived: bool,
}

pub fn list(conn: &Connection, clock: &dyn Clock, params: ListParams) -> Result<Vec<Habit>> {
    let filter = if params.include_archived { "" } else { "WHERE archived_at IS NULL" };
    let sql = format!("{SELECT} {filter} ORDER BY archived_at IS NOT NULL, position, id");
    let bases: Vec<Base> = conn.prepare(&sql)?.query_map([], map_base)?.collect::<rusqlite::Result<_>>()?;
    bases.into_iter().map(|base| assemble(conn, clock, base)).collect()
}

pub fn get(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Habit> {
    let base =
        conn.query_row(&format!("{SELECT} WHERE id = ?1"), [id], map_base).map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound("This habit"),
            other => other.into(),
        })?;
    assemble(conn, clock, base)
}

fn validate(input: &mut HabitInput, today: NaiveDate) -> Result<()> {
    input.name = input.name.trim().to_string();
    if input.name.is_empty() {
        return Err(Error::invalid("Give the habit a name"));
    }
    let kind = input.kind.get_or_insert_with(|| "build".into()).clone();
    if !matches!(kind.as_str(), "build" | "quit" | "quantity" | "checklist") {
        return Err(Error::invalid(format!("Unknown habit type: {kind}")));
    }
    let frequency = input.frequency.get_or_insert_with(|| "daily".into()).clone();
    match frequency.as_str() {
        "daily" => {
            input.weekdays = (0..7).collect();
            input.weekly_target = None;
        }
        "weekdays" => {
            input.weekdays.sort_unstable();
            input.weekdays.dedup();
            if input.weekdays.is_empty() || input.weekdays.iter().any(|day| *day > 6) {
                return Err(Error::invalid("Choose at least one day of the week"));
            }
            input.weekly_target = None;
        }
        "weekly_target" => {
            if kind == "quit" {
                return Err(Error::invalid(
                    "A habit you're quitting applies every day, not a set number of times",
                ));
            }
            input.weekdays.clear();
            if !input.weekly_target.is_some_and(|target| (1..=7).contains(&target)) {
                return Err(Error::invalid("Set a weekly target from 1 to 7"));
            }
        }
        other => return Err(Error::invalid(format!("Unknown frequency: {other}"))),
    }
    if kind == "quantity" {
        if !input.quantity_target.is_some_and(|target| target > 0.0 && target.is_finite()) {
            return Err(Error::invalid("Set a target above zero"));
        }
    } else {
        input.quantity_target = None;
    }
    input.checklist.retain(|item| !item.title.trim().is_empty());
    if kind == "checklist" && input.checklist.is_empty() {
        return Err(Error::invalid("Add at least one checklist item"));
    }
    if kind != "checklist" {
        input.checklist.clear();
    }
    input.reminder_time = input.reminder_time.take().filter(|text| !text.is_empty());
    if let Some(time) = &input.reminder_time {
        parse_time(time)?;
    }
    match input.start_date.take().filter(|text| !text.is_empty()) {
        Some(text) => {
            parse_date(&text)?;
            input.start_date = Some(text);
        }
        None => input.start_date = Some(format_date(today)),
    }
    Ok(())
}

fn set_schedule(conn: &Connection, id: i64, weekdays: &[u8]) -> Result<()> {
    conn.execute("DELETE FROM habit_schedules WHERE habit_id = ?1", [id])?;
    for day in weekdays {
        conn.execute("INSERT INTO habit_schedules (habit_id, weekday) VALUES (?1, ?2)", params![id, day])?;
    }
    Ok(())
}

/// Updates checklist items in place, so history keeps pointing at items that still exist.
fn set_checklist(conn: &Connection, id: i64, items: &[ChecklistItem]) -> Result<()> {
    let keep: Vec<i64> = items.iter().map(|item| item.id).filter(|id| *id > 0).collect();
    let mut existing = conn.prepare("SELECT id FROM habit_checklist_items WHERE habit_id = ?1")?;
    let current: Vec<i64> = existing.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    for item in current.iter().filter(|item| !keep.contains(item)) {
        conn.execute("DELETE FROM habit_checklist_items WHERE id = ?1", [item])?;
    }
    for (position, item) in items.iter().enumerate() {
        if item.id > 0 && current.contains(&item.id) {
            conn.execute(
                "UPDATE habit_checklist_items SET title = ?1, position = ?2 WHERE id = ?3",
                params![item.title.trim(), position as i64, item.id],
            )?;
        } else {
            conn.execute(
                "INSERT INTO habit_checklist_items (habit_id, title, position) VALUES (?1, ?2, ?3)",
                params![id, item.title.trim(), position as i64],
            )?;
        }
    }
    Ok(())
}

pub fn create(conn: &Connection, clock: &dyn Clock, mut input: HabitInput) -> Result<Habit> {
    validate(&mut input, clock.today())?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO habits (name, description, icon, color, kind, frequency, weekly_target, start_date,
                             reminder_time, quantity_target, unit, position, created_at, updated_at)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, coalesce(max(position), -1) + 1, ?12, ?12
         FROM habits",
        params![
            input.name,
            input.description,
            input.icon.as_deref().unwrap_or("sparkles"),
            input.color.as_deref().unwrap_or("orange"),
            input.kind,
            input.frequency,
            input.weekly_target,
            input.start_date,
            input.reminder_time,
            input.quantity_target,
            input.unit.trim(),
            format_instant(clock.now())
        ],
    )?;
    let id = tx.last_insert_rowid();
    set_schedule(&tx, id, &input.weekdays)?;
    set_checklist(&tx, id, &input.checklist)?;
    tx.commit()?;
    get(conn, clock, id)
}

#[derive(Debug, Deserialize)]
pub struct UpdateParams {
    pub id: i64,
    #[serde(flatten)]
    pub input: HabitInput,
}

pub fn update(conn: &Connection, clock: &dyn Clock, params: UpdateParams) -> Result<Habit> {
    let UpdateParams { id, mut input } = params;
    let current = get(conn, clock, id)?;
    input.start_date.get_or_insert(current.start_date.clone());
    validate(&mut input, clock.today())?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE habits SET name = ?1, description = ?2, icon = ?3, color = ?4, kind = ?5, frequency = ?6,
                weekly_target = ?7, start_date = ?8, reminder_time = ?9, quantity_target = ?10, unit = ?11,
                updated_at = ?12 WHERE id = ?13",
        params![
            input.name,
            input.description,
            input.icon.as_deref().unwrap_or(&current.icon),
            input.color.as_deref().unwrap_or(&current.color),
            input.kind,
            input.frequency,
            input.weekly_target,
            input.start_date,
            input.reminder_time,
            input.quantity_target,
            input.unit.trim(),
            format_instant(clock.now()),
            id
        ],
    )?;
    set_schedule(&tx, id, &input.weekdays)?;
    set_checklist(&tx, id, &input.checklist)?;
    tx.commit()?;
    get(conn, clock, id)
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LogParams {
    pub id: i64,
    pub date: Option<String>,
    pub value: Option<f64>,
    pub checked_items: Option<Vec<i64>>,
    pub slip: bool,
    pub note: Option<String>,
}

/// Records a day's result. The stored status comes from the habit's type, so a quantity counts as done
/// only once it meets the target and a checklist only once every item is ticked.
pub fn log(conn: &Connection, clock: &dyn Clock, params: LogParams) -> Result<Habit> {
    let habit = get(conn, clock, params.id)?;
    let today = clock.today();
    let date = params.date.as_deref().map(parse_date).transpose()?.unwrap_or(today);
    if date > today {
        return Err(Error::invalid("Habits can't be logged for a future date"));
    }
    if date < parse_date(&habit.start_date)? {
        return Err(Error::invalid("This date is before the habit started"));
    }

    let previous = load_log(conn, habit.id)?.remove(&date);
    let value = params.value.or(previous.as_ref().and_then(|row| row.value));
    if value.is_some_and(|number| number < 0.0 || !number.is_finite()) {
        return Err(Error::invalid("Enter an amount of zero or more"));
    }
    let checked =
        params.checked_items.or(previous.as_ref().map(|row| row.checked_items.clone())).unwrap_or_default();
    let note = params.note.or(previous.as_ref().map(|row| row.note.clone())).unwrap_or_default();

    let status = match habit.kind.as_str() {
        "quit" if params.slip => "slip",
        "quit" => "partial",
        "quantity" => {
            let target = habit.quantity_target.unwrap_or(1.0);
            if value.unwrap_or(0.0) >= target {
                "done"
            } else {
                "partial"
            }
        }
        "checklist" => {
            let all = habit.checklist.iter().all(|item| checked.contains(&item.id));
            if all {
                "done"
            } else {
                "partial"
            }
        }
        _ => "done",
    };
    let now = format_instant(clock.now());
    conn.execute(
        "INSERT INTO habit_completions (habit_id, date, status, value, checked_items, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
         ON CONFLICT (habit_id, date) DO UPDATE SET status = excluded.status, value = excluded.value,
             checked_items = excluded.checked_items, note = excluded.note, updated_at = excluded.updated_at",
        params![habit.id, format_date(date), status, value, serde_json::to_string(&checked)?, note, now],
    )?;
    get(conn, clock, habit.id)
}

#[derive(Debug, Deserialize)]
pub struct UndoParams {
    pub id: i64,
    pub date: Option<String>,
}

pub fn undo(conn: &Connection, clock: &dyn Clock, params: UndoParams) -> Result<Habit> {
    let date = params.date.unwrap_or_else(|| format_date(clock.today()));
    conn.execute(
        "DELETE FROM habit_completions WHERE habit_id = ?1 AND date = ?2",
        params![params.id, date],
    )?;
    get(conn, clock, params.id)
}

pub fn set_archived(conn: &Connection, clock: &dyn Clock, id: i64, archived: bool) -> Result<Habit> {
    get(conn, clock, id)?;
    let now = format_instant(clock.now());
    conn.execute(
        "UPDATE habits SET archived_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![archived.then_some(&now), now, id],
    )?;
    get(conn, clock, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    if conn.execute("DELETE FROM habits WHERE id = ?1", [id])? == 0 {
        return Err(Error::NotFound("This habit"));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ReorderParams {
    pub ids: Vec<i64>,
}

pub fn reorder(conn: &Connection, params: ReorderParams) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (position, id) in params.ids.iter().enumerate() {
        tx.execute("UPDATE habits SET position = ?1 WHERE id = ?2", params![position as i64, id])?;
    }
    tx.commit()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    pub id: i64,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDay {
    pub date: String,
    pub state: DayState,
    pub value: Option<f64>,
    pub note: String,
}

/// One habit's days in a range, for the month calendar.
pub fn history(conn: &Connection, clock: &dyn Clock, params: HistoryParams) -> Result<Vec<HistoryDay>> {
    let habit = get(conn, clock, params.id)?;
    let (from, to) = parse_range(&params.from, &params.to)?;
    let base = conn.query_row(&format!("{SELECT} WHERE id = ?1"), [habit.id], map_base)?;
    let spec = spec_of(conn, &base, &habit.weekdays)?;
    let log = load_log(conn, habit.id)?;
    let entries = entries_of(&log);
    let engine = Engine::new(&spec, &entries, clock.today());
    Ok(from
        .iter_days()
        .take_while(|date| *date <= to)
        .map(|date| HistoryDay {
            date: format_date(date),
            state: engine.state(date),
            value: log.get(&date).and_then(|row| row.value),
            note: log.get(&date).map(|row| row.note.clone()).unwrap_or_default(),
        })
        .collect())
}

fn parse_range(from: &str, to: &str) -> Result<(NaiveDate, NaiveDate)> {
    let (from, to) = (parse_date(from)?, parse_date(to)?);
    if from > to || (to - from).num_days() > 800 {
        return Err(Error::invalid("Choose a range of up to two years"));
    }
    Ok((from, to))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapParams {
    /// One habit, or every active habit when absent.
    pub id: Option<i64>,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDay {
    pub date: String,
    /// 0 is nothing done, and 1 to 4 are rising intensity.
    pub level: u8,
    pub done: u32,
    pub due: u32,
    /// Due today and still open. These are undecided, so rates leave them out.
    pub pending: u32,
    pub missed: bool,
}

#[derive(Clone, Copy, Default)]
struct DayTotal {
    score: f64,
    done: u32,
    due: u32,
    pending: u32,
    missed: u32,
}

fn level_for(ratio: f64) -> u8 {
    match ratio {
        r if r <= 0.0 => 0,
        r if r < 0.34 => 1,
        r if r < 0.67 => 2,
        r if r < 1.0 => 3,
        _ => 4,
    }
}

/// Per-day totals across one or all habits.
pub fn heatmap(conn: &Connection, clock: &dyn Clock, params: HeatmapParams) -> Result<Vec<HeatmapDay>> {
    let (from, to) = parse_range(&params.from, &params.to)?;
    let today = clock.today();
    let sql = match params.id {
        Some(_) => format!("{SELECT} WHERE id = ?1"),
        None => format!("{SELECT} WHERE archived_at IS NULL AND ?1 IS NULL"),
    };
    let bases: Vec<Base> =
        conn.prepare(&sql)?.query_map([params.id], map_base)?.collect::<rusqlite::Result<_>>()?;

    let mut totals: HashMap<NaiveDate, DayTotal> = HashMap::new();
    for base in &bases {
        let weekdays = load_weekdays(conn, base.id)?;
        let spec = spec_of(conn, base, &weekdays)?;
        let log = load_log(conn, base.id)?;
        let entries = entries_of(&log);
        let engine = Engine::new(&spec, &entries, today);
        for date in from.iter_days().take_while(|date| *date <= to) {
            let total = totals.entry(date).or_default();
            match engine.state(date) {
                DayState::Done => {
                    total.score += 1.0;
                    total.done += 1;
                    total.due += 1;
                }
                DayState::Partial => {
                    total.score += progress(base, log.get(&date));
                    total.due += 1;
                }
                DayState::Pending => {
                    total.score += log.get(&date).map_or(0.0, |row| progress(base, Some(row)));
                    total.due += 1;
                    total.pending += 1;
                }
                DayState::Missed => {
                    total.due += 1;
                    total.missed += 1;
                }
                DayState::Inactive | DayState::NotScheduled => {}
            }
        }
    }

    Ok(from
        .iter_days()
        .take_while(|date| *date <= to)
        .map(|date| {
            let total = totals.get(&date).copied().unwrap_or_default();
            HeatmapDay {
                date: format_date(date),
                level: level_for(if total.due == 0 { 0.0 } else { total.score / f64::from(total.due) }),
                done: total.done,
                due: total.due,
                pending: total.pending,
                missed: total.missed > 0 && total.score <= 0.0,
            }
        })
        .collect())
}

fn progress(base: &Base, row: Option<&LogRow>) -> f64 {
    let Some(row) = row else { return 0.0 };
    match (base.quantity_target, row.value) {
        (Some(target), Some(value)) => (value / target).clamp(0.0, 1.0),
        _ if !row.checked_items.is_empty() => 0.5,
        _ => 0.0,
    }
}

/// One active habit's state across a set of dates. The calendar and overview build on this.
pub struct HabitDays {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub states: Vec<DayState>,
    pub due: Vec<bool>,
}

pub fn day_states(conn: &Connection, clock: &dyn Clock, dates: &[NaiveDate]) -> Result<Vec<HabitDays>> {
    let today = clock.today();
    let bases: Vec<Base> = conn
        .prepare(&format!("{SELECT} WHERE archived_at IS NULL ORDER BY position, id"))?
        .query_map([], map_base)?
        .collect::<rusqlite::Result<_>>()?;
    let mut result = Vec::with_capacity(bases.len());
    for base in bases {
        let weekdays = load_weekdays(conn, base.id)?;
        let spec = spec_of(conn, &base, &weekdays)?;
        let entries = entries_of(&load_log(conn, base.id)?);
        let engine = Engine::new(&spec, &entries, today);
        result.push(HabitDays {
            id: base.id,
            name: base.name,
            color: base.color,
            states: dates.iter().map(|date| engine.state(*date)).collect(),
            // A weekly target can fall on any day, so no particular future date is due.
            due: dates
                .iter()
                .map(|date| {
                    let fixed = matches!(spec.schedule, Schedule::Days(_));
                    *date >= spec.start && engine.is_due(*date) && (fixed || *date <= today)
                })
                .collect(),
        });
    }
    Ok(result)
}

/// Habits with a reminder at any time, for the scheduler.
pub struct ReminderHabit {
    pub id: i64,
    pub name: String,
    pub time: String,
    pub state: DayState,
}

pub fn with_reminders(conn: &Connection, clock: &dyn Clock) -> Result<Vec<ReminderHabit>> {
    let today = clock.today();
    let bases: Vec<Base> = conn
        .prepare(&format!("{SELECT} WHERE archived_at IS NULL AND reminder_time IS NOT NULL"))?
        .query_map([], map_base)?
        .collect::<rusqlite::Result<_>>()?;
    let mut result = Vec::with_capacity(bases.len());
    for base in bases {
        let weekdays = load_weekdays(conn, base.id)?;
        let spec = spec_of(conn, &base, &weekdays)?;
        let entries = entries_of(&load_log(conn, base.id)?);
        let state = Engine::new(&spec, &entries, today).state(today);
        result.push(ReminderHabit {
            id: base.id,
            name: base.name,
            time: base.reminder_time.unwrap_or_default(),
            state,
        });
    }
    Ok(result)
}
