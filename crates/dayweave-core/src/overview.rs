use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::clock::{format_date, Clock};
use crate::error::Result;
use crate::habits::{self, Habit};
use crate::settings;
use crate::streaks::DayState;
use crate::tasks::{self, Task};
use crate::{focus, habits::ListParams};

const OPEN: &str = "t.status IN ('todo', 'in_progress')";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekDay {
    pub date: String,
    pub tasks_done: i64,
    pub habits_done: u32,
    pub habits_due: u32,
    pub focus_seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub date: String,
    pub tasks: Vec<Task>,
    pub overdue: Vec<Task>,
    pub upcoming: Vec<Task>,
    pub habits: Vec<Habit>,
    pub tasks_done: u32,
    pub tasks_total: u32,
    pub habits_done: u32,
    pub habits_total: u32,
    pub progress: f64,
    pub focus_seconds: i64,
    pub focus_sessions: i64,
    pub week: Vec<WeekDay>,
}

/// A quit habit stays on track until a slip is logged, even though its day has not ended.
pub fn habit_met(habit: &Habit) -> bool {
    if habit.kind == "quit" {
        return !habit.today.slipped;
    }
    habit.today.state == DayState::Done
}

pub fn week_of(date: NaiveDate, week_start: chrono::Weekday) -> NaiveDate {
    date.week(week_start).first_day()
}

fn week(conn: &Connection, clock: &dyn Clock) -> Result<Vec<WeekDay>> {
    let start = week_of(clock.today(), settings::week_start(conn)?);
    let dates: Vec<NaiveDate> = (0..7).map(|offset| start + Duration::days(offset)).collect();
    let states = habits::day_states(conn, clock, &dates)?;
    dates
        .iter()
        .enumerate()
        .map(|(index, date)| {
            let text = format_date(*date);
            let tasks_done: i64 = conn.query_row(
                "SELECT count(*) FROM tasks WHERE completed_on = ?1 AND status != 'archived'",
                [&text],
                |row| row.get(0),
            )?;
            let focus_seconds: i64 = conn.query_row(
                "SELECT coalesce(sum(elapsed_seconds), 0) FROM focus_sessions
                 WHERE local_date = ?1 AND kind = 'focus' AND status IN ('completed', 'stopped')",
                [&text],
                |row| row.get(0),
            )?;
            let (mut habits_done, mut habits_due) = (0, 0);
            for habit in &states {
                let state = habit.states[index];
                if habit.due[index] && state != DayState::Inactive && state != DayState::NotScheduled {
                    habits_due += 1;
                    if state == DayState::Done {
                        habits_done += 1;
                    }
                }
            }
            Ok(WeekDay { date: text, tasks_done, habits_done, habits_due, focus_seconds })
        })
        .collect()
}

pub fn get(conn: &Connection, clock: &dyn Clock) -> Result<Overview> {
    let today = clock.today();
    let text = format_date(today);

    let order = "CASE WHEN t.status = 'completed' THEN 1 ELSE 0 END, t.due_time IS NULL, t.due_time,
                 CASE t.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END, t.id";
    let tail = format!(
        "WHERE ({OPEN} AND (t.due_date = ? OR t.start_date = ?)) OR t.completed_on = ? ORDER BY {order}"
    );
    let task_list =
        tasks::query(conn, clock, &tail, &[text.clone().into(), text.clone().into(), text.clone().into()])?;
    let overdue =
        tasks::list(conn, clock, tasks::ListParams { view: Some("overdue".into()), ..Default::default() })?;
    let horizon = format_date(today + Duration::days(14));
    let upcoming = tasks::query(
        conn,
        clock,
        &format!(
            "WHERE {OPEN} AND t.due_date > ? AND t.due_date <= ?
             ORDER BY t.due_date, t.due_time IS NULL, t.due_time LIMIT 8"
        ),
        &[text.clone().into(), horizon.into()],
    )?;
    let habit_list: Vec<Habit> = habits::list(conn, clock, ListParams::default())?
        .into_iter()
        .filter(|habit| habit.today.due)
        .collect();

    let tasks_total = task_list.len() as u32;
    let tasks_done = task_list.iter().filter(|task| task.status == "completed").count() as u32;
    let habits_total = habit_list.len() as u32;
    let habits_done = habit_list.iter().filter(|habit| habit_met(habit)).count() as u32;
    let total = tasks_total + habits_total;
    let progress = if total == 0 { 0.0 } else { f64::from(tasks_done + habits_done) / f64::from(total) };

    let (focus_seconds, focus_sessions) = conn.query_row(
        "SELECT coalesce(sum(elapsed_seconds), 0), count(*) FILTER (WHERE status = 'completed')
         FROM focus_sessions WHERE local_date = ?1 AND kind = 'focus' AND status IN ('completed', 'stopped')",
        params![text],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(Overview {
        date: text,
        tasks: task_list,
        overdue,
        upcoming,
        habits: habit_list,
        tasks_done,
        tasks_total,
        habits_done,
        habits_total,
        progress,
        focus_seconds,
        focus_sessions,
        week: week(conn, clock)?,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Badges {
    pub overdue: i64,
    pub today: i64,
    pub unread: i64,
    pub focus: Option<focus::Session>,
}

/// The few live numbers the application shell shows on every page.
pub fn badges(conn: &Connection, clock: &dyn Clock) -> Result<Badges> {
    let counts = tasks::counts(conn, clock)?;
    Ok(Badges {
        overdue: counts.overdue,
        today: counts.today,
        unread: crate::reminders::unread_count(conn)?,
        focus: focus::current(conn, clock)?,
    })
}
