//! Decides what to notify about. The rules are described in ARCHITECTURE.md: nothing is scheduled ahead
//! of time, and the notification log's primary key is what prevents duplicates.

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::clock::{
    format_date, format_datetime, format_instant, parse_date, parse_datetime, parse_instant, parse_time,
    Clock,
};
use crate::error::Result;
use crate::focus;
use crate::settings;
use crate::streaks::DayState;

/// Reminders older than this are dropped rather than delivered late.
const MAX_CATCH_UP_DAYS: i64 = 7;
/// A notice later than this counts as missed, and several of them are grouped into one.
const LATE_AFTER_MINUTES: i64 = 5;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub key: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub task_id: Option<i64>,
    pub habit_id: Option<i64>,
}

/// The wall-clock time a reminder fires for a deadline. A date-only deadline has no time of its own,
/// so `default_time` stands in for it.
pub fn fire_time(
    offset_minutes: i64,
    due_date: NaiveDate,
    due_time: Option<NaiveTime>,
    default_time: NaiveTime,
) -> NaiveDateTime {
    due_date.and_time(due_time.unwrap_or(default_time)) - Duration::minutes(offset_minutes)
}

fn describe_deadline(
    due_date: NaiveDate,
    due_time: Option<&str>,
    today: NaiveDate,
    now: NaiveDateTime,
) -> String {
    let days = (due_date - today).num_days();
    let Some(time) = due_time else {
        return match days {
            ..=-2 => format!("Overdue by {} days", -days),
            -1 => "Overdue by 1 day".into(),
            0 => "Due today".into(),
            1 => "Due tomorrow".into(),
            _ => format!("Due in {days} days"),
        };
    };
    let Ok(parsed) = parse_time(time) else { return String::new() };
    let minutes = (due_date.and_time(parsed) - now).num_minutes();
    match minutes {
        m if m < 0 => format!("Was due at {time}"),
        0 => "Due now".into(),
        1 => "Due in 1 minute".into(),
        2..=59 => format!("Due in {minutes} minutes"),
        60..=119 => "Due in 1 hour".into(),
        120..=1439 if days == 0 => format!("Due in {} hours", minutes / 60),
        _ if days == 1 => format!("Due tomorrow at {time}"),
        _ => format!("Due in {days} days"),
    }
}

struct Due {
    fire: DateTime<Utc>,
    notice: Notice,
    snoozed_id: Option<i64>,
}

fn task_reminders(
    conn: &Connection,
    clock: &dyn Clock,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Vec<Due>> {
    let default_time = parse_time(&settings::get_string(conn, "dateOnlyReminderTime")?)?;
    let today = clock.today();
    let local_now = clock.local_now();
    let mut statement = conn.prepare(
        "SELECT r.id, r.kind, r.offset_minutes, r.remind_at, r.snoozed, t.id, t.title, t.due_date, t.due_time
         FROM task_reminders r JOIN tasks t ON t.id = r.task_id
         WHERE t.status IN ('todo', 'in_progress')",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<i64>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, bool>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
        ))
    })?;

    let mut due = Vec::new();
    for row in rows {
        let (id, kind, offset, remind_at, snoozed, task, title, due_date, due_time) = row?;
        let parsed_due = due_date.as_deref().map(parse_date).transpose()?;
        let parsed_time = due_time.as_deref().map(parse_time).transpose()?;
        let wall = match (kind.as_str(), offset, &remind_at, parsed_due) {
            ("before", Some(minutes), _, Some(date)) => fire_time(minutes, date, parsed_time, default_time),
            ("at", _, Some(text), _) => parse_datetime(text)?,
            _ => continue,
        };
        let fire = clock.to_instant(wall);
        if fire <= since || fire > now {
            continue;
        }
        let body = parsed_due
            .map(|date| describe_deadline(date, due_time.as_deref(), today, local_now))
            .unwrap_or_else(|| "Reminder".into());
        // The deadline is part of the key: rescheduling makes this a new reminder.
        let deadline =
            format!("{}T{}", due_date.as_deref().unwrap_or("none"), due_time.as_deref().unwrap_or(""));
        due.push(Due {
            fire,
            notice: Notice {
                key: format!("task:{task}:reminder:{id}:{deadline}:{}", format_datetime(wall)),
                kind: "task".into(),
                title,
                body,
                task_id: Some(task),
                habit_id: None,
            },
            snoozed_id: snoozed.then_some(id),
        });
    }
    Ok(due)
}

fn habit_reminders(
    conn: &Connection,
    clock: &dyn Clock,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Vec<Due>> {
    let today = clock.today();
    let mut due = Vec::new();
    for habit in crate::habits::with_reminders(conn, clock)? {
        if habit.state != DayState::Pending {
            continue;
        }
        let fire = clock.to_instant(today.and_time(parse_time(&habit.time)?));
        if fire <= since || fire > now {
            continue;
        }
        due.push(Due {
            fire,
            notice: Notice {
                key: format!("habit:{}:{}", habit.id, format_date(today)),
                kind: "habit".into(),
                title: habit.name,
                body: "Time for this habit".into(),
                task_id: None,
                habit_id: Some(habit.id),
            },
            snoozed_id: None,
        });
    }
    Ok(due)
}

fn focus_notice(session: &focus::Session) -> Notice {
    let (title, body) = match session.kind.as_str() {
        "focus" => (
            "Focus session complete".to_string(),
            session.task_title.clone().unwrap_or_else(|| "Time for a break".into()),
        ),
        _ => ("Break is over".to_string(), "Ready for the next session?".to_string()),
    };
    Notice {
        key: format!("focus:{}", session.id),
        kind: "focus".into(),
        title,
        body,
        task_id: session.task_id,
        habit_id: session.habit_id,
    }
}

/// Records a notice. Returns false if this exact notice was already recorded.
fn record(conn: &Connection, notice: &Notice, at: DateTime<Utc>) -> Result<bool> {
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO notification_log (key, kind, title, body, task_id, habit_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            notice.key,
            notice.kind,
            notice.title,
            notice.body,
            notice.task_id,
            notice.habit_id,
            format_instant(at)
        ],
    )?;
    Ok(inserted == 1)
}

/// One pass of the scheduler. Returns what should be shown as desktop notifications now.
pub fn tick(conn: &Connection, clock: &dyn Clock) -> Result<Vec<Notice>> {
    let now = clock.now();
    let tx = conn.unchecked_transaction()?;
    let earliest = now - Duration::days(MAX_CATCH_UP_DAYS);
    let since = match settings::get_internal(&tx, settings::LAST_TICK)? {
        Some(text) => parse_instant(&text)?.max(earliest),
        // On the very first run there is no history to catch up on.
        None => now,
    };

    let mut due = task_reminders(&tx, clock, since, now)?;
    due.extend(habit_reminders(&tx, clock, since, now)?);
    due.sort_by_key(|item| item.fire);

    let tasks_on = settings::get_bool(&tx, "taskNotifications")?;
    let habits_on = settings::get_bool(&tx, "habitNotifications")?;
    let focus_on = settings::get_bool(&tx, "focusNotifications")?;
    let enabled = settings::get_bool(&tx, "notificationsEnabled")?;

    let mut on_time = Vec::new();
    let mut late = Vec::new();
    for item in due {
        if let Some(id) = item.snoozed_id {
            tx.execute("DELETE FROM task_reminders WHERE id = ?1", [id])?;
        }
        if !record(&tx, &item.notice, item.fire)? {
            continue;
        }
        let wanted = match item.notice.kind.as_str() {
            "habit" => habits_on,
            _ => tasks_on,
        };
        if !wanted {
            continue;
        }
        if now - item.fire > Duration::minutes(LATE_AFTER_MINUTES) {
            late.push(item.notice);
        } else {
            on_time.push(item.notice);
        }
    }

    if let Some(session) = focus::finish_due(&tx, clock)? {
        let notice = focus_notice(&session);
        if record(&tx, &notice, now)? && focus_on {
            on_time.push(notice);
        }
    }

    settings::set_internal(&tx, settings::LAST_TICK, &format_instant(now))?;
    tx.commit()?;

    if !enabled {
        return Ok(Vec::new());
    }
    let mut notices = group_late(late, now);
    notices.extend(on_time);
    Ok(notices)
}

fn group_late(mut late: Vec<Notice>, now: DateTime<Utc>) -> Vec<Notice> {
    if late.len() < 2 {
        return late;
    }
    let count = late.len();
    let names: Vec<String> = late.drain(..).take(3).map(|notice| notice.title).collect();
    let more = count.saturating_sub(names.len());
    let body = if more > 0 { format!("{} and {more} more", names.join(", ")) } else { names.join(", ") };
    vec![Notice {
        key: format!("summary:{}", format_instant(now)),
        kind: "summary".into(),
        title: format!("{count} reminders while Dayweave was closed"),
        body,
        task_id: None,
        habit_id: None,
    }]
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub key: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub task_id: Option<i64>,
    pub habit_id: Option<i64>,
    pub created_at: String,
    pub read: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LogParams {
    pub limit: Option<i64>,
}

pub fn log(conn: &Connection, params: LogParams) -> Result<Vec<LogEntry>> {
    Ok(conn
        .prepare(
            "SELECT key, kind, title, body, task_id, habit_id, created_at, read_at IS NOT NULL
             FROM notification_log WHERE cleared_at IS NULL ORDER BY created_at DESC LIMIT ?1",
        )?
        .query_map([params.limit.unwrap_or(50).clamp(1, 200)], |row| {
            Ok(LogEntry {
                key: row.get(0)?,
                kind: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                task_id: row.get(4)?,
                habit_id: row.get(5)?,
                created_at: row.get(6)?,
                read: row.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn unread_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM notification_log WHERE read_at IS NULL AND cleared_at IS NULL",
        [],
        |row| row.get(0),
    )?)
}

pub fn mark_all_read(conn: &Connection, clock: &dyn Clock) -> Result<()> {
    conn.execute(
        "UPDATE notification_log SET read_at = ?1 WHERE read_at IS NULL",
        [format_instant(clock.now())],
    )?;
    Ok(())
}

/// Empties the visible list. Recent rows stay hidden rather than deleted, because their keys are what
/// stop those notices firing a second time. Rows past the catch-up window can no longer repeat.
pub fn clear(conn: &Connection, clock: &dyn Clock) -> Result<()> {
    let now = clock.now();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE notification_log SET cleared_at = ?1, read_at = coalesce(read_at, ?1) WHERE cleared_at IS NULL",
        [format_instant(now)],
    )?;
    tx.execute(
        "DELETE FROM notification_log WHERE created_at < ?1",
        [format_instant(now - Duration::days(MAX_CATCH_UP_DAYS + 1))],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        parse_date(text).unwrap()
    }

    fn wall(text: &str) -> NaiveDateTime {
        parse_datetime(text).unwrap()
    }

    #[test]
    fn a_timed_deadline_fires_its_offset_ahead() {
        let time = parse_time("17:00").unwrap();
        let default = parse_time("09:00").unwrap();
        let fire = fire_time(30, date("2026-11-06"), Some(time), default);
        assert_eq!(fire, wall("2026-11-06T16:30"));
    }

    #[test]
    fn a_date_only_deadline_uses_the_default_time_of_day() {
        let default = parse_time("09:00").unwrap();
        assert_eq!(fire_time(0, date("2026-11-06"), None, default), wall("2026-11-06T09:00"));
        assert_eq!(fire_time(1440, date("2026-11-06"), None, default), wall("2026-11-05T09:00"));
    }

    #[test]
    fn an_offset_can_cross_midnight() {
        let time = parse_time("00:10").unwrap();
        let fire = fire_time(30, date("2026-11-06"), Some(time), parse_time("09:00").unwrap());
        assert_eq!(fire, wall("2026-11-05T23:40"));
    }

    #[test]
    fn descriptions_of_date_only_deadlines_never_mention_hours() {
        let today = date("2026-11-02");
        let now = wall("2026-11-02T23:30");
        assert_eq!(describe_deadline(today, None, today, now), "Due today");
        assert_eq!(describe_deadline(date("2026-11-03"), None, today, now), "Due tomorrow");
        assert_eq!(describe_deadline(date("2026-11-05"), None, today, now), "Due in 3 days");
        assert_eq!(describe_deadline(date("2026-11-01"), None, today, now), "Overdue by 1 day");
        assert_eq!(describe_deadline(date("2026-10-30"), None, today, now), "Overdue by 3 days");
    }

    #[test]
    fn descriptions_of_timed_deadlines_use_the_closest_unit() {
        let today = date("2026-11-02");
        let now = wall("2026-11-02T14:00");
        assert_eq!(describe_deadline(today, Some("14:10"), today, now), "Due in 10 minutes");
        assert_eq!(describe_deadline(today, Some("15:00"), today, now), "Due in 1 hour");
        assert_eq!(describe_deadline(today, Some("18:00"), today, now), "Due in 4 hours");
        assert_eq!(describe_deadline(today, Some("14:00"), today, now), "Due now");
        assert_eq!(describe_deadline(today, Some("13:00"), today, now), "Was due at 13:00");
        assert_eq!(describe_deadline(date("2026-11-03"), Some("09:00"), today, now), "Due tomorrow at 09:00");
        assert_eq!(describe_deadline(date("2026-11-06"), Some("09:00"), today, now), "Due in 4 days");
    }

    #[test]
    fn several_late_notices_become_one_summary() {
        let notice = |title: &str| Notice {
            key: title.into(),
            kind: "task".into(),
            title: title.into(),
            body: String::new(),
            task_id: None,
            habit_id: None,
        };
        let now = Utc::now();
        assert_eq!(group_late(vec![notice("One")], now).len(), 1);
        let grouped = group_late(vec![notice("A"), notice("B"), notice("C"), notice("D")], now);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].title, "4 reminders while Dayweave was closed");
        assert_eq!(grouped[0].body, "A, B, C and 1 more");
    }
}
