use std::collections::BTreeMap;

use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::clock::{
    format_date, format_datetime, format_time, parse_date, parse_datetime, parse_time, Clock,
};
use crate::error::{Error, Result};
use crate::habits;
use crate::reminders::fire_time;
use crate::settings;
use crate::streaks::DayState;
use crate::tasks;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// 'task' is a start date, 'deadline' a due date, and the rest are as named.
    pub kind: String,
    pub id: i64,
    pub title: String,
    pub time: Option<String>,
    pub priority: Option<String>,
    pub completed: bool,
    pub overdue: bool,
    pub seconds: Option<i64>,
    pub color: Option<String>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    pub date: String,
    pub items: Vec<Item>,
    pub habits_due: u32,
    pub habits_done: u32,
    pub focus_seconds: i64,
}

#[derive(Debug, Deserialize)]
pub struct RangeParams {
    pub from: String,
    pub to: String,
}

pub fn range(conn: &Connection, clock: &dyn Clock, params: RangeParams) -> Result<Vec<Day>> {
    let (from, to) = (parse_date(&params.from)?, parse_date(&params.to)?);
    if from > to || (to - from).num_days() > 62 {
        return Err(Error::invalid("The calendar shows up to two months at a time"));
    }
    let dates: Vec<NaiveDate> = from.iter_days().take_while(|date| *date <= to).collect();
    let mut days: BTreeMap<String, Day> = dates
        .iter()
        .map(|date| {
            let date = format_date(*date);
            (date.clone(), Day { date, ..Day::default() })
        })
        .collect();

    add_tasks(conn, clock, &params, &mut days)?;
    add_reminders(conn, &mut days)?;
    add_focus(conn, clock, &params, &mut days)?;

    for habit in habits::day_states(conn, clock, &dates)? {
        for (index, date) in dates.iter().enumerate() {
            let state = habit.states[index];
            let active = habit.due[index] || state == DayState::Done;
            if !active || state == DayState::NotScheduled {
                continue;
            }
            let Some(day) = days.get_mut(&format_date(*date)) else { continue };
            let completed = state == DayState::Done;
            day.habits_due += 1;
            day.habits_done += u32::from(completed);
            day.items.push(Item {
                kind: "habit".into(),
                id: habit.id,
                title: habit.name.clone(),
                time: None,
                priority: None,
                completed,
                overdue: state == DayState::Missed,
                seconds: None,
                color: Some(habit.color.clone()),
                due_date: None,
            });
        }
    }

    for day in days.values_mut() {
        day.items.sort_by(|a, b| {
            (a.time.is_none(), a.time.as_deref(), a.completed).cmp(&(
                b.time.is_none(),
                b.time.as_deref(),
                b.completed,
            ))
        });
    }
    Ok(days.into_values().collect())
}

fn add_tasks(
    conn: &Connection,
    clock: &dyn Clock,
    range: &RangeParams,
    days: &mut BTreeMap<String, Day>,
) -> Result<()> {
    let tail = "WHERE t.status != 'archived' AND ((t.due_date >= ? AND t.due_date <= ?)
                OR (t.start_date >= ? AND t.start_date <= ?))";
    let values = [
        range.from.clone().into(),
        range.to.clone().into(),
        range.from.clone().into(),
        range.to.clone().into(),
    ];
    for task in tasks::query(conn, clock, tail, &values)? {
        let item = |kind: &str, time: Option<String>| Item {
            kind: kind.into(),
            id: task.id,
            title: task.title.clone(),
            time,
            priority: Some(task.priority.clone()),
            completed: task.status == "completed",
            overdue: task.overdue,
            seconds: None,
            color: task.project_color.clone(),
            due_date: task.due_date.clone(),
        };
        if let Some(day) = task.due_date.as_ref().and_then(|date| days.get_mut(date)) {
            day.items.push(item("deadline", task.due_time.clone()));
        }
        if task.start_date != task.due_date {
            if let Some(day) = task.start_date.as_ref().and_then(|date| days.get_mut(date)) {
                day.items.push(item("task", None));
            }
        }
    }
    Ok(())
}

fn add_reminders(conn: &Connection, days: &mut BTreeMap<String, Day>) -> Result<()> {
    let default_time = parse_time(&settings::get_string(conn, "dateOnlyReminderTime")?)?;
    let mut statement = conn.prepare(
        "SELECT t.id, t.title, r.kind, r.offset_minutes, r.remind_at, t.due_date, t.due_time
         FROM task_reminders r JOIN tasks t ON t.id = r.task_id
         WHERE t.status IN ('todo', 'in_progress') AND r.snoozed = 0",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;
    for row in rows {
        let (id, title, kind, offset, remind_at, due_date, due_time) = row?;
        let wall = match (kind.as_str(), offset, remind_at, due_date.as_deref()) {
            ("before", Some(minutes), _, Some(date)) => {
                let time = due_time.as_deref().map(parse_time).transpose()?;
                fire_time(minutes, parse_date(date)?, time, default_time)
            }
            ("at", _, Some(text), _) => parse_datetime(&text)?,
            _ => continue,
        };
        let text = format_datetime(wall);
        if let Some(day) = days.get_mut(&text[..10]) {
            day.items.push(Item {
                kind: "reminder".into(),
                id,
                title,
                time: Some(format_time(wall.time())),
                priority: None,
                completed: false,
                overdue: false,
                seconds: None,
                color: None,
                due_date,
            });
        }
    }
    Ok(())
}

fn add_focus(
    conn: &Connection,
    clock: &dyn Clock,
    range: &RangeParams,
    days: &mut BTreeMap<String, Day>,
) -> Result<()> {
    let mut statement = conn.prepare(
        "SELECT f.id, f.local_date, f.started_at, f.elapsed_seconds, t.title, h.name
         FROM focus_sessions f
         LEFT JOIN tasks t ON t.id = f.task_id
         LEFT JOIN habits h ON h.id = f.habit_id
         WHERE f.kind = 'focus' AND f.status IN ('completed', 'stopped')
           AND f.local_date >= ?1 AND f.local_date <= ?2",
    )?;
    let rows = statement.query_map(params![range.from, range.to], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    for row in rows {
        let (id, date, started, seconds, task, habit) = row?;
        let Some(day) = days.get_mut(&date) else { continue };
        let started = crate::clock::parse_instant(&started)?;
        day.focus_seconds += seconds;
        day.items.push(Item {
            kind: "focus".into(),
            id,
            title: task.or(habit).unwrap_or_else(|| "Focus session".into()),
            time: Some(format_time(clock.to_local(started).time())),
            priority: None,
            completed: true,
            overdue: false,
            seconds: Some(seconds),
            color: None,
            due_date: None,
        });
    }
    Ok(())
}
