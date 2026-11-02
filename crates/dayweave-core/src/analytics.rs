use std::collections::{BTreeMap, HashMap};

use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::clock::{format_date, parse_date, Clock};
use crate::error::{Error, Result};
use crate::habits::{self, HeatmapDay, HeatmapParams, ListParams};
use crate::overview::week_of;
use crate::settings;
use crate::streaks::StreakUnit;

#[derive(Debug, Deserialize)]
pub struct Params {
    pub days: i64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DayPoint {
    pub date: String,
    pub tasks_completed: i64,
    pub tasks_created: i64,
    pub habit_rate: Option<f64>,
    pub habits_done: u32,
    pub focus_seconds: i64,
    pub focus_sessions: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekPoint {
    pub week: String,
    pub tasks_completed: i64,
    pub habit_rate: Option<f64>,
    pub focus_seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriorityCount {
    pub priority: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCompletion {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub total: i64,
    pub done: i64,
    pub completion: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HabitSummary {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub current_streak: u32,
    pub best_streak: u32,
    pub streak_unit: StreakUnit,
    pub completion_rate: f64,
    pub weekly_consistency: f64,
    pub monthly_consistency: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    pub days: i64,
    pub tasks_completed: i64,
    pub tasks_overdue: i64,
    pub tasks_pending: i64,
    /// Finished tasks as a share of those finished in the period plus those still open and past due.
    pub task_completion_rate: f64,
    pub habit_completion_rate: f64,
    pub focus_seconds: i64,
    pub focus_sessions: i64,
    pub daily: Vec<DayPoint>,
    pub weekly: Vec<WeekPoint>,
    pub priorities: Vec<PriorityCount>,
    pub projects: Vec<ProjectCompletion>,
    pub habits: Vec<HabitSummary>,
    pub most_consistent: Vec<HabitSummary>,
    pub needs_improvement: Vec<HabitSummary>,
}

fn counts_by_date(conn: &Connection, sql: &str, from: &str, to: &str) -> Result<HashMap<String, (i64, i64)>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![from, to], |row| {
        Ok((row.get::<_, String>(0)?, (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get(conn: &Connection, clock: &dyn Clock, params: Params) -> Result<Analytics> {
    if !(1..=366).contains(&params.days) {
        return Err(Error::invalid("Choose a period of up to a year"));
    }
    let today = clock.today();
    let from = today - Duration::days(params.days - 1);
    let (from_text, to_text) = (format_date(from), format_date(today));

    let completed = counts_by_date(
        conn,
        "SELECT completed_on, count(*), 0 FROM tasks
         WHERE completed_on >= ?1 AND completed_on <= ?2 AND status != 'archived' GROUP BY completed_on",
        &from_text,
        &to_text,
    )?;
    // created_at is UTC, so the local day comes from converting each row.
    let mut created: HashMap<String, i64> = HashMap::new();
    let mut statement = conn.prepare("SELECT created_at FROM tasks WHERE created_at >= ?1")?;
    let lower =
        crate::clock::format_instant(clock.to_instant(from.and_hms_opt(0, 0, 0).expect("valid time")));
    for row in statement.query_map([lower], |row| row.get::<_, String>(0))? {
        let instant = crate::clock::parse_instant(&row?)?;
        *created.entry(format_date(clock.to_local(instant).date())).or_default() += 1;
    }
    let focus = counts_by_date(
        conn,
        "SELECT local_date, sum(elapsed_seconds), count(*) FILTER (WHERE status = 'completed')
         FROM focus_sessions
         WHERE kind = 'focus' AND status IN ('completed', 'stopped') AND local_date >= ?1 AND local_date <= ?2
         GROUP BY local_date",
        &from_text,
        &to_text,
    )?;
    let heat = habits::heatmap(
        conn,
        clock,
        HeatmapParams { id: None, from: from_text.clone(), to: to_text.clone() },
    )?;

    let daily: Vec<DayPoint> = heat
        .iter()
        .map(|day| {
            let (focus_seconds, focus_sessions) = focus.get(&day.date).copied().unwrap_or_default();
            DayPoint {
                date: day.date.clone(),
                tasks_completed: completed.get(&day.date).map_or(0, |value| value.0),
                tasks_created: created.get(&day.date).copied().unwrap_or(0),
                habit_rate: settled_rate(day),
                habits_done: day.done,
                focus_seconds,
                focus_sessions,
            }
        })
        .collect();

    let weekly = weekly_points(conn, &daily)?;
    let tasks_completed = daily.iter().map(|day| day.tasks_completed).sum();
    let focus_seconds = daily.iter().map(|day| day.focus_seconds).sum();
    let focus_sessions = daily.iter().map(|day| day.focus_sessions).sum();

    let counts = crate::tasks::counts(conn, clock)?;
    let tasks_pending: i64 =
        conn.query_row("SELECT count(*) FROM tasks WHERE status IN ('todo', 'in_progress')", [], |row| {
            row.get(0)
        })?;
    let settled = tasks_completed + counts.overdue;
    let task_completion_rate = if settled == 0 { 0.0 } else { tasks_completed as f64 / settled as f64 };

    let (habit_done, habit_due) =
        heat.iter().fold((0u32, 0u32), |(done, due), day| (done + day.done, due + day.due - day.pending));
    let habit_completion_rate =
        if habit_due == 0 { 0.0 } else { f64::from(habit_done) / f64::from(habit_due) };

    let habits = habit_summaries(conn, clock)?;
    let mut ranked: Vec<&HabitSummary> = habits.iter().collect();
    ranked.sort_by(|a, b| b.monthly_consistency.total_cmp(&a.monthly_consistency));
    let most_consistent = ranked.iter().take(3).map(|habit| (*habit).clone()).collect();
    let needs_improvement = ranked
        .iter()
        .rev()
        .filter(|habit| habit.monthly_consistency < 0.7)
        .take(3)
        .map(|habit| (*habit).clone())
        .collect();

    Ok(Analytics {
        days: params.days,
        tasks_completed,
        tasks_overdue: counts.overdue,
        tasks_pending,
        task_completion_rate,
        habit_completion_rate,
        focus_seconds,
        focus_sessions,
        daily,
        weekly,
        priorities: priorities(conn)?,
        projects: projects(conn)?,
        habits,
        most_consistent,
        needs_improvement,
    })
}

/// The share of habits met on a day, leaving out any still open today: an unfinished day hasn't
/// failed, the same rule the streak engine follows.
fn settled_rate(day: &HeatmapDay) -> Option<f64> {
    let settled = day.due - day.pending;
    (settled > 0).then(|| f64::from(day.done) / f64::from(settled))
}

fn weekly_points(conn: &Connection, daily: &[DayPoint]) -> Result<Vec<WeekPoint>> {
    let week_start = settings::week_start(conn)?;
    let mut weeks: BTreeMap<NaiveDate, (i64, f64, u32, i64)> = BTreeMap::new();
    for day in daily {
        let week = weeks.entry(week_of(parse_date(&day.date)?, week_start)).or_default();
        week.0 += day.tasks_completed;
        if let Some(rate) = day.habit_rate {
            week.1 += rate;
            week.2 += 1;
        }
        week.3 += day.focus_seconds;
    }
    Ok(weeks
        .into_iter()
        .map(|(week, (tasks, rate, days, focus))| WeekPoint {
            week: format_date(week),
            tasks_completed: tasks,
            habit_rate: (days > 0).then(|| rate / f64::from(days)),
            focus_seconds: focus,
        })
        .collect())
}

fn priorities(conn: &Connection) -> Result<Vec<PriorityCount>> {
    Ok(conn
        .prepare(
            "SELECT priority, count(*) FROM tasks WHERE status IN ('todo', 'in_progress') GROUP BY priority
             ORDER BY CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END",
        )?
        .query_map([], |row| Ok(PriorityCount { priority: row.get(0)?, count: row.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?)
}

fn projects(conn: &Connection) -> Result<Vec<ProjectCompletion>> {
    Ok(conn
        .prepare(
            "SELECT p.id, p.name, p.color, count(t.id) FILTER (WHERE t.status != 'archived'),
                    count(t.id) FILTER (WHERE t.status = 'completed')
             FROM projects p LEFT JOIN tasks t ON t.project_id = p.id
             WHERE p.archived_at IS NULL GROUP BY p.id ORDER BY p.name COLLATE NOCASE",
        )?
        .query_map([], |row| {
            let (total, done): (i64, i64) = (row.get(3)?, row.get(4)?);
            Ok(ProjectCompletion {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                total,
                done,
                completion: if total == 0 { 0.0 } else { done as f64 / total as f64 },
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

fn habit_summaries(conn: &Connection, clock: &dyn Clock) -> Result<Vec<HabitSummary>> {
    Ok(habits::list(conn, clock, ListParams::default())?
        .into_iter()
        .map(|habit| HabitSummary {
            id: habit.id,
            name: habit.name,
            color: habit.color,
            icon: habit.icon,
            current_streak: habit.stats.current_streak,
            best_streak: habit.stats.best_streak,
            streak_unit: habit.stats.streak_unit,
            completion_rate: habit.stats.completion_rate,
            weekly_consistency: habit.stats.weekly_consistency,
            monthly_consistency: habit.stats.monthly_consistency,
        })
        .collect())
}
