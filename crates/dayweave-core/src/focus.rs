use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::clock::{format_date, format_instant, parse_instant, Clock};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: i64,
    pub kind: String,
    pub status: String,
    pub planned_seconds: i64,
    pub elapsed_seconds: i64,
    pub remaining_seconds: i64,
    pub task_id: Option<i64>,
    pub task_title: Option<String>,
    pub habit_id: Option<i64>,
    pub habit_name: Option<String>,
    pub local_date: String,
    pub started_at: String,
    pub resumed_at: Option<String>,
    pub ended_at: Option<String>,
    /// When a running session will finish. The interface counts down to this.
    pub ends_at: Option<String>,
}

const SELECT: &str = "
    SELECT f.id, f.kind, f.status, f.planned_seconds, f.elapsed_seconds, f.task_id, t.title, f.habit_id,
           h.name, f.local_date, f.started_at, f.resumed_at, f.ended_at
    FROM focus_sessions f
    LEFT JOIN tasks t ON t.id = f.task_id
    LEFT JOIN habits h ON h.id = f.habit_id";

fn map_row(row: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        kind: row.get(1)?,
        status: row.get(2)?,
        planned_seconds: row.get(3)?,
        elapsed_seconds: row.get(4)?,
        remaining_seconds: 0,
        task_id: row.get(5)?,
        task_title: row.get(6)?,
        habit_id: row.get(7)?,
        habit_name: row.get(8)?,
        local_date: row.get(9)?,
        started_at: row.get(10)?,
        resumed_at: row.get(11)?,
        ended_at: row.get(12)?,
        ends_at: None,
    })
}

fn finish_row(mut session: Session, now: DateTime<Utc>) -> Result<Session> {
    if session.status == "running" {
        if let Some(resumed) = &session.resumed_at {
            let resumed = parse_instant(resumed)?;
            let before = session.elapsed_seconds;
            session.elapsed_seconds = before + (now - resumed).num_seconds().max(0);
            session.ends_at =
                Some(format_instant(resumed + Duration::seconds(session.planned_seconds - before)));
        }
    }
    session.remaining_seconds = (session.planned_seconds - session.elapsed_seconds).max(0);
    Ok(session)
}

pub fn current(conn: &Connection, clock: &dyn Clock) -> Result<Option<Session>> {
    let session = conn
        .query_row(&format!("{SELECT} WHERE f.status IN ('running', 'paused')"), [], map_row)
        .optional()?;
    session.map(|session| finish_row(session, clock.now())).transpose()
}

fn get(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Session> {
    let session = conn
        .query_row(&format!("{SELECT} WHERE f.id = ?1"), [id], map_row)
        .optional()?
        .ok_or(Error::NotFound("This focus session"))?;
    finish_row(session, clock.now())
}

fn active(conn: &Connection, clock: &dyn Clock) -> Result<Session> {
    current(conn, clock)?.ok_or_else(|| Error::invalid("There is no focus session in progress"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartParams {
    pub kind: String,
    pub minutes: i64,
    pub task_id: Option<i64>,
    pub habit_id: Option<i64>,
}

pub fn start(conn: &Connection, clock: &dyn Clock, params: StartParams) -> Result<Session> {
    if !matches!(params.kind.as_str(), "focus" | "short_break" | "long_break") {
        return Err(Error::invalid(format!("Unknown session type: {}", params.kind)));
    }
    if !(1..=480).contains(&params.minutes) {
        return Err(Error::invalid("Sessions can run from 1 to 480 minutes"));
    }
    if current(conn, clock)?.is_some() {
        return Err(Error::invalid("Finish or stop the current session first"));
    }
    let now = format_instant(clock.now());
    conn.execute(
        "INSERT INTO focus_sessions (kind, status, planned_seconds, task_id, habit_id, local_date,
                                     started_at, resumed_at)
         VALUES (?1, 'running', ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            params.kind,
            params.minutes * 60,
            params.task_id,
            params.habit_id,
            format_date(clock.today()),
            now
        ],
    )?;
    get(conn, clock, conn.last_insert_rowid())
}

pub fn pause(conn: &Connection, clock: &dyn Clock) -> Result<Session> {
    let session = active(conn, clock)?;
    if session.status == "running" {
        conn.execute(
            "UPDATE focus_sessions SET status = 'paused', elapsed_seconds = ?1, resumed_at = NULL
             WHERE id = ?2",
            params![session.elapsed_seconds.min(session.planned_seconds), session.id],
        )?;
    }
    get(conn, clock, session.id)
}

pub fn resume(conn: &Connection, clock: &dyn Clock) -> Result<Session> {
    let session = active(conn, clock)?;
    if session.status == "paused" {
        conn.execute(
            "UPDATE focus_sessions SET status = 'running', resumed_at = ?1 WHERE id = ?2",
            params![format_instant(clock.now()), session.id],
        )?;
    }
    get(conn, clock, session.id)
}

fn end(
    conn: &Connection,
    clock: &dyn Clock,
    session: &Session,
    status: &str,
    at: DateTime<Utc>,
) -> Result<()> {
    conn.execute(
        "UPDATE focus_sessions SET status = ?1, elapsed_seconds = ?2, resumed_at = NULL, ended_at = ?3
         WHERE id = ?4",
        params![
            status,
            session.elapsed_seconds.min(session.planned_seconds),
            format_instant(at.min(clock.now())),
            session.id
        ],
    )?;
    Ok(())
}

/// Ends a session early. Time already spent still counts.
pub fn stop(conn: &Connection, clock: &dyn Clock) -> Result<Session> {
    let session = active(conn, clock)?;
    end(conn, clock, &session, "stopped", clock.now())?;
    get(conn, clock, session.id)
}

/// Marks the session finished now, for when the work is done before the timer is.
pub fn complete(conn: &Connection, clock: &dyn Clock) -> Result<Session> {
    let session = active(conn, clock)?;
    end(conn, clock, &session, "completed", clock.now())?;
    get(conn, clock, session.id)
}

/// Finishes a running session whose time is up, on behalf of the scheduler. The end time is when the
/// timer ran out, which may be earlier than now if the application was closed.
pub fn finish_due(conn: &Connection, clock: &dyn Clock) -> Result<Option<Session>> {
    let Some(session) = current(conn, clock)? else { return Ok(None) };
    if session.status != "running" || session.remaining_seconds > 0 {
        return Ok(None);
    }
    let ended = match &session.ends_at {
        Some(text) => parse_instant(text)?,
        None => clock.now(),
    };
    end(conn, clock, &session, "completed", ended)?;
    get(conn, clock, session.id).map(Some)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct HistoryParams {
    pub limit: Option<i64>,
}

pub fn history(conn: &Connection, clock: &dyn Clock, params: HistoryParams) -> Result<Vec<Session>> {
    let sql = format!(
        "{SELECT} WHERE f.status IN ('completed', 'stopped')
         ORDER BY f.started_at DESC, f.id DESC LIMIT ?1"
    );
    let rows: Vec<Session> = conn
        .prepare(&sql)?
        .query_map([params.limit.unwrap_or(30).clamp(1, 500)], map_row)?
        .collect::<rusqlite::Result<_>>()?;
    rows.into_iter().map(|session| finish_row(session, clock.now())).collect()
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    if conn.execute("DELETE FROM focus_sessions WHERE id = ?1", [id])? == 0 {
        return Err(Error::NotFound("This focus session"));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub today_seconds: i64,
    pub week_seconds: i64,
    pub total_seconds: i64,
    pub today_sessions: i64,
    pub total_sessions: i64,
    /// Completed focus sessions since the last long break, used to suggest the next break.
    pub sessions_since_long_break: i64,
}

pub fn stats(conn: &Connection, clock: &dyn Clock) -> Result<Stats> {
    let today = clock.today();
    let week = format_date(today - Duration::days(6));
    let today = format_date(today);
    let (today_seconds, week_seconds, total_seconds, today_sessions, total_sessions) = conn.query_row(
        "SELECT
            coalesce(sum(elapsed_seconds) FILTER (WHERE local_date = ?1), 0),
            coalesce(sum(elapsed_seconds) FILTER (WHERE local_date >= ?2), 0),
            coalesce(sum(elapsed_seconds), 0),
            count(*) FILTER (WHERE local_date = ?1 AND status = 'completed'),
            count(*) FILTER (WHERE status = 'completed')
         FROM focus_sessions WHERE kind = 'focus' AND status IN ('completed', 'stopped')",
        params![today, week],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    )?;
    let sessions_since_long_break = conn.query_row(
        "SELECT count(*) FROM focus_sessions
         WHERE kind = 'focus' AND status = 'completed' AND local_date = ?1 AND id > coalesce(
             (SELECT max(id) FROM focus_sessions WHERE kind = 'long_break' AND status = 'completed'), 0)",
        [&today],
        |row| row.get(0),
    )?;
    Ok(Stats {
        today_seconds,
        week_seconds,
        total_seconds,
        today_sessions,
        total_sessions,
        sessions_since_long_break,
    })
}
