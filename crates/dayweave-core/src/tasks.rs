use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::clock::{
    format_date, format_datetime, format_instant, format_time, parse_date, parse_datetime, parse_time, Clock,
};
use crate::db::placeholders;
use crate::error::{Error, Result};
use crate::recurrence::{Frequency, Rule};

const STATUSES: &[&str] = &["todo", "in_progress", "completed", "archived"];
const PRIORITIES: &[&str] = &["low", "medium", "high", "urgent"];
const OPEN: &str = "t.status IN ('todo', 'in_progress')";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Subtask {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub kind: String,
    pub offset_minutes: Option<i64>,
    pub remind_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: i64,
    pub kind: String,
    pub offset_minutes: Option<i64>,
    pub remind_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceInput {
    pub frequency: String,
    #[serde(default = "one")]
    pub interval: i64,
    /// Selected weekdays for a weekly rule, where 0 is Monday.
    #[serde(default)]
    pub weekdays: Vec<u8>,
    pub end_date: Option<String>,
    pub max_count: Option<i64>,
}

fn one() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub category: Option<String>,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    pub project_color: Option<String>,
    pub habit_id: Option<i64>,
    pub habit_name: Option<String>,
    pub recurrence_id: Option<i64>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub completed_on: Option<String>,
    pub archived_at: Option<String>,
    pub overdue: bool,
    pub subtask_total: i64,
    pub subtask_done: i64,
    pub reminder_count: i64,
    pub tags: Vec<Tag>,
    /// Filled by `get`, and left empty in lists.
    pub subtasks: Vec<Subtask>,
    pub reminders: Vec<Reminder>,
    pub recurrence: Option<RecurrenceInput>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskInput {
    pub title: String,
    pub description: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub project_id: Option<i64>,
    pub habit_id: Option<i64>,
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub tags: Vec<String>,
    pub subtasks: Vec<String>,
    pub reminders: Vec<ReminderInput>,
    pub recurrence: Option<RecurrenceInput>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListParams {
    pub view: Option<String>,
    pub search: Option<String>,
    pub priorities: Vec<String>,
    pub statuses: Vec<String>,
    pub project_id: Option<i64>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub sort: Option<String>,
    pub descending: bool,
    pub limit: Option<i64>,
}

const SELECT: &str = "
    SELECT t.id, t.title, t.description, t.status, t.priority, t.category, t.project_id, p.name, p.color,
           t.habit_id, h.name, t.recurrence_id, t.start_date, t.due_date, t.due_time, t.estimated_minutes,
           t.created_at, t.updated_at, t.completed_at, t.completed_on, t.archived_at,
           (SELECT count(*) FROM task_subtasks s WHERE s.task_id = t.id),
           (SELECT count(*) FROM task_subtasks s WHERE s.task_id = t.id AND s.completed_at IS NOT NULL),
           (SELECT count(*) FROM task_reminders r WHERE r.task_id = t.id AND r.snoozed = 0)
    FROM tasks t
    LEFT JOIN projects p ON p.id = t.project_id
    LEFT JOIN habits h ON h.id = t.habit_id";

fn map_row(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        category: row.get(5)?,
        project_id: row.get(6)?,
        project_name: row.get(7)?,
        project_color: row.get(8)?,
        habit_id: row.get(9)?,
        habit_name: row.get(10)?,
        recurrence_id: row.get(11)?,
        start_date: row.get(12)?,
        due_date: row.get(13)?,
        due_time: row.get(14)?,
        estimated_minutes: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
        completed_at: row.get(18)?,
        completed_on: row.get(19)?,
        archived_at: row.get(20)?,
        subtask_total: row.get(21)?,
        subtask_done: row.get(22)?,
        reminder_count: row.get(23)?,
        overdue: false,
        tags: Vec::new(),
        subtasks: Vec::new(),
        reminders: Vec::new(),
        recurrence: None,
    })
}

/// A completed or archived task is never overdue, whatever its date says.
pub fn is_overdue(status: &str, due_date: Option<&str>, due_time: Option<&str>, clock: &dyn Clock) -> bool {
    if !matches!(status, "todo" | "in_progress") {
        return false;
    }
    let Some(due_date) = due_date else { return false };
    let now = clock.local_now();
    let today = format_date(now.date());
    match (due_date.cmp(today.as_str()), due_time) {
        (std::cmp::Ordering::Less, _) => true,
        (std::cmp::Ordering::Equal, Some(time)) => time < format_time(now.time()).as_str(),
        _ => false,
    }
}

/// Runs a task query and attaches tags and the overdue flag.
pub fn query(conn: &Connection, clock: &dyn Clock, tail: &str, values: &[SqlValue]) -> Result<Vec<Task>> {
    let sql = format!("{SELECT} {tail}");
    let mut statement = conn.prepare(&sql)?;
    let mut tasks = statement
        .query_map(params_from_iter(values.iter()), map_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    attach_tags(conn, &mut tasks)?;
    for task in &mut tasks {
        task.overdue = is_overdue(&task.status, task.due_date.as_deref(), task.due_time.as_deref(), clock);
    }
    Ok(tasks)
}

fn attach_tags(conn: &Connection, tasks: &mut [Task]) -> Result<()> {
    if tasks.is_empty() {
        return Ok(());
    }
    let mut by_task: HashMap<i64, Vec<Tag>> = HashMap::new();
    for chunk in tasks.chunks(500) {
        let ids: Vec<i64> = chunk.iter().map(|task| task.id).collect();
        let sql = format!(
            "SELECT tt.task_id, tg.id, tg.name FROM task_tags tt JOIN tags tg ON tg.id = tt.tag_id
             WHERE tt.task_id IN ({}) ORDER BY tg.name COLLATE NOCASE",
            placeholders(ids.len())
        );
        let mut statement = conn.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(ids.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, Tag { id: row.get(1)?, name: row.get(2)? }))
        })?;
        for row in rows {
            let (task, tag) = row?;
            by_task.entry(task).or_default().push(tag);
        }
    }
    for task in tasks {
        task.tags = by_task.remove(&task.id).unwrap_or_default();
    }
    Ok(())
}

/// Builds the WHERE and ORDER BY for a list request.
fn build_list(params: &ListParams, clock: &dyn Clock) -> Result<(String, Vec<SqlValue>)> {
    let now = clock.local_now();
    let today = format_date(now.date());
    let time = format_time(now.time());
    let mut clauses: Vec<String> = Vec::new();
    let mut values: Vec<SqlValue> = Vec::new();

    match params.view.as_deref().unwrap_or("all") {
        "all" => clauses.push("t.status != 'archived'".into()),
        "today" => {
            clauses.push(format!("{OPEN} AND (t.due_date = ? OR t.start_date = ?)"));
            values.push(today.clone().into());
            values.push(today.clone().into());
        }
        "upcoming" => {
            clauses.push(format!("{OPEN} AND t.due_date > ?"));
            values.push(today.clone().into());
        }
        "overdue" => {
            clauses.push(format!(
                "{OPEN} AND (t.due_date < ? OR (t.due_date = ? AND t.due_time IS NOT NULL AND t.due_time < ?))"
            ));
            values.push(today.clone().into());
            values.push(today.clone().into());
            values.push(time.into());
        }
        "completed" => clauses.push("t.status = 'completed'".into()),
        "archived" => clauses.push("t.status = 'archived'".into()),
        other => return Err(Error::invalid(format!("Unknown task view: {other}"))),
    }

    if let Some(search) = params.search.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
        clauses.push("(t.title LIKE ? ESCAPE '\\' OR t.description LIKE ? ESCAPE '\\')".into());
        let pattern = like_pattern(search);
        values.push(pattern.clone().into());
        values.push(pattern.into());
    }
    if !params.priorities.is_empty() {
        clauses.push(format!("t.priority IN ({})", placeholders(params.priorities.len())));
        values.extend(params.priorities.iter().map(|value| value.clone().into()));
    }
    if !params.statuses.is_empty() {
        clauses.push(format!("t.status IN ({})", placeholders(params.statuses.len())));
        values.extend(params.statuses.iter().map(|value| value.clone().into()));
    }
    if let Some(project) = params.project_id {
        clauses.push("t.project_id = ?".into());
        values.push(project.into());
    }
    if let Some(category) = &params.category {
        clauses.push("t.category = ?".into());
        values.push(category.clone().into());
    }
    if let Some(tag) = &params.tag {
        clauses.push(
            "EXISTS (SELECT 1 FROM task_tags tt JOIN tags tg ON tg.id = tt.tag_id
                     WHERE tt.task_id = t.id AND tg.name = ? COLLATE NOCASE)"
                .into(),
        );
        values.push(tag.clone().into());
    }
    if let Some(from) = &params.date_from {
        clauses.push("t.due_date >= ?".into());
        values.push(from.clone().into());
    }
    if let Some(to) = &params.date_to {
        clauses.push("t.due_date <= ?".into());
        values.push(to.clone().into());
    }

    let direction = if params.descending { "DESC" } else { "ASC" };
    let priority_rank =
        "CASE t.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 ELSE 3 END";
    let deadline =
        format!("t.due_date IS NULL, t.due_date {direction}, t.due_time IS NULL, t.due_time {direction}");
    let order = match params.sort.as_deref().unwrap_or("deadline") {
        "deadline" => format!("{deadline}, {priority_rank}"),
        "priority" => format!("{priority_rank} {direction}, {deadline}"),
        "created" => format!("t.created_at {direction}, t.id {direction}"),
        "status" => format!(
            "CASE t.status WHEN 'in_progress' THEN 0 WHEN 'todo' THEN 1 WHEN 'completed' THEN 2 ELSE 3 END
             {direction}, {deadline}"
        ),
        "title" => format!("t.title COLLATE NOCASE {direction}"),
        other => return Err(Error::invalid(format!("Unknown sort: {other}"))),
    };

    let mut tail = format!("WHERE {} ORDER BY {order}, t.id", clauses.join(" AND "));
    if let Some(limit) = params.limit {
        tail.push_str(&format!(" LIMIT {}", limit.max(0)));
    }
    Ok((tail, values))
}

pub fn like_pattern(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

pub fn list(conn: &Connection, clock: &dyn Clock, params: ListParams) -> Result<Vec<Task>> {
    let (tail, values) = build_list(&params, clock)?;
    query(conn, clock, &tail, &values)
}

/// Tasks grouped by view, for the counts beside the Tasks navigation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub all: i64,
    pub today: i64,
    pub upcoming: i64,
    pub overdue: i64,
    pub completed: i64,
    pub archived: i64,
}

pub fn counts(conn: &Connection, clock: &dyn Clock) -> Result<Counts> {
    let now = clock.local_now();
    let today = format_date(now.date());
    let time = format_time(now.time());
    Ok(conn.query_row(
        &format!(
            "SELECT
                count(*) FILTER (WHERE t.status != 'archived'),
                count(*) FILTER (WHERE {OPEN} AND (t.due_date = ?1 OR t.start_date = ?1)),
                count(*) FILTER (WHERE {OPEN} AND t.due_date > ?1),
                count(*) FILTER (WHERE {OPEN} AND (t.due_date < ?1
                    OR (t.due_date = ?1 AND t.due_time IS NOT NULL AND t.due_time < ?2))),
                count(*) FILTER (WHERE t.status = 'completed'),
                count(*) FILTER (WHERE t.status = 'archived')
             FROM tasks t"
        ),
        params![today, time],
        |row| {
            Ok(Counts {
                all: row.get(0)?,
                today: row.get(1)?,
                upcoming: row.get(2)?,
                overdue: row.get(3)?,
                completed: row.get(4)?,
                archived: row.get(5)?,
            })
        },
    )?)
}

pub fn get(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    let mut task =
        query(conn, clock, "WHERE t.id = ?", &[id.into()])?.pop().ok_or(Error::NotFound("This task"))?;

    let mut statement = conn.prepare(
        "SELECT id, parent_id, title, completed_at IS NOT NULL FROM task_subtasks
         WHERE task_id = ?1 ORDER BY position, id",
    )?;
    task.subtasks = statement
        .query_map([id], |row| {
            Ok(Subtask {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                title: row.get(2)?,
                completed: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut statement = conn.prepare(
        "SELECT id, kind, offset_minutes, remind_at FROM task_reminders
         WHERE task_id = ?1 AND snoozed = 0 ORDER BY offset_minutes, remind_at",
    )?;
    task.reminders = statement
        .query_map([id], |row| {
            Ok(Reminder {
                id: row.get(0)?,
                kind: row.get(1)?,
                offset_minutes: row.get(2)?,
                remind_at: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    if let Some(recurrence_id) = task.recurrence_id {
        task.recurrence = Some(load_rule_input(conn, recurrence_id)?);
    }
    Ok(task)
}

fn load_rule_input(conn: &Connection, id: i64) -> Result<RecurrenceInput> {
    Ok(conn.query_row(
        "SELECT frequency, interval, weekdays, end_date, max_count FROM task_recurrence WHERE id = ?1",
        [id],
        |row| {
            let mask: u8 = row.get(2)?;
            Ok(RecurrenceInput {
                frequency: row.get(0)?,
                interval: row.get(1)?,
                weekdays: (0..7).filter(|day| mask & (1 << day) != 0).collect(),
                end_date: row.get(3)?,
                max_count: row.get(4)?,
            })
        },
    )?)
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

fn validate(input: &mut TaskInput) -> Result<()> {
    input.title = input.title.trim().to_string();
    if input.title.is_empty() {
        return Err(Error::invalid("Give the task a title"));
    }
    if input.title.chars().count() > 500 {
        return Err(Error::invalid("Task titles can be up to 500 characters"));
    }
    if let Some(priority) = &input.priority {
        if !PRIORITIES.contains(&priority.as_str()) {
            return Err(Error::invalid(format!("Unknown priority: {priority}")));
        }
    }
    input.category = clean_optional(input.category.take());
    input.start_date = clean_optional(input.start_date.take());
    input.due_date = clean_optional(input.due_date.take());
    input.due_time = clean_optional(input.due_time.take());

    let due = input.due_date.as_deref().map(parse_date).transpose()?;
    let start = input.start_date.as_deref().map(parse_date).transpose()?;
    if let Some(time) = &input.due_time {
        parse_time(time)?;
        if due.is_none() {
            return Err(Error::invalid("Choose a due date before setting a time"));
        }
    }
    if let (Some(start), Some(due)) = (start, due) {
        if start > due {
            return Err(Error::invalid("The start date must be on or before the due date"));
        }
    }
    if input.estimated_minutes.is_some_and(|minutes| minutes < 0) {
        return Err(Error::invalid("The estimated duration cannot be negative"));
    }
    for reminder in &input.reminders {
        validate_reminder(reminder, due.is_some())?;
    }
    if let Some(recurrence) = &input.recurrence {
        if due.is_none() {
            return Err(Error::invalid("Choose a due date for a repeating task"));
        }
        validate_recurrence(recurrence)?;
    }
    Ok(())
}

fn validate_reminder(reminder: &ReminderInput, has_due_date: bool) -> Result<()> {
    match reminder.kind.as_str() {
        "before" => {
            if !has_due_date {
                return Err(Error::invalid("Choose a due date before adding a deadline reminder"));
            }
            match reminder.offset_minutes {
                Some(minutes) if minutes >= 0 => Ok(()),
                _ => Err(Error::invalid("Reminders must come before the deadline")),
            }
        }
        "at" => {
            let Some(remind_at) = &reminder.remind_at else {
                return Err(Error::invalid("Choose when to be reminded"));
            };
            parse_datetime(remind_at).map(|_| ())
        }
        other => Err(Error::invalid(format!("Unknown reminder type: {other}"))),
    }
}

fn validate_recurrence(recurrence: &RecurrenceInput) -> Result<()> {
    if Frequency::parse(&recurrence.frequency).is_none() {
        return Err(Error::invalid(format!("Unknown repeat: {}", recurrence.frequency)));
    }
    if !(1..=999).contains(&recurrence.interval) {
        return Err(Error::invalid("Repeat every 1 to 999 periods"));
    }
    if recurrence.weekdays.iter().any(|day| *day > 6) {
        return Err(Error::invalid("Weekdays run from 0 for Monday to 6 for Sunday"));
    }
    if let Some(end) = &recurrence.end_date {
        parse_date(end)?;
    }
    if recurrence.max_count.is_some_and(|count| count < 1) {
        return Err(Error::invalid("A repeating task needs at least one occurrence"));
    }
    Ok(())
}

fn weekday_mask(weekdays: &[u8]) -> u8 {
    weekdays.iter().fold(0, |mask, day| mask | (1 << day))
}

fn insert_recurrence(conn: &Connection, input: &RecurrenceInput, due: NaiveDate) -> Result<i64> {
    conn.execute(
        "INSERT INTO task_recurrence (frequency, interval, weekdays, anchor_day, end_date, max_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.frequency,
            input.interval,
            weekday_mask(&input.weekdays),
            due.day(),
            input.end_date,
            input.max_count
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

fn set_tags(conn: &Connection, task: i64, names: &[String]) -> Result<()> {
    conn.execute("DELETE FROM task_tags WHERE task_id = ?1", [task])?;
    for name in names {
        let name = name.trim().trim_start_matches('#').trim();
        if name.is_empty() {
            continue;
        }
        conn.execute("INSERT OR IGNORE INTO tags (name) VALUES (?1)", [name])?;
        let tag: i64 = conn.query_row("SELECT id FROM tags WHERE name = ?1", [name], |row| row.get(0))?;
        conn.execute("INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)", [task, tag])?;
    }
    conn.execute("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM task_tags)", [])?;
    Ok(())
}

fn set_reminders(conn: &Connection, task: i64, reminders: &[ReminderInput]) -> Result<()> {
    conn.execute("DELETE FROM task_reminders WHERE task_id = ?1 AND snoozed = 0", [task])?;
    let mut seen: Vec<&ReminderInput> = Vec::new();
    for reminder in reminders {
        if seen.contains(&reminder) {
            continue;
        }
        seen.push(reminder);
        conn.execute(
            "INSERT INTO task_reminders (task_id, kind, offset_minutes, remind_at) VALUES (?1, ?2, ?3, ?4)",
            params![task, reminder.kind, reminder.offset_minutes, reminder.remind_at],
        )?;
    }
    Ok(())
}

pub fn create(conn: &Connection, clock: &dyn Clock, mut input: TaskInput) -> Result<Task> {
    validate(&mut input)?;
    let status = input.status.as_deref().unwrap_or("todo");
    if !matches!(status, "todo" | "in_progress") {
        return Err(Error::invalid("New tasks start as To do or In progress"));
    }
    let tx = conn.unchecked_transaction()?;
    let recurrence = match (&input.recurrence, &input.due_date) {
        (Some(rule), Some(due)) => Some(insert_recurrence(&tx, rule, parse_date(due)?)?),
        _ => None,
    };
    let now = format_instant(clock.now());
    tx.execute(
        "INSERT INTO tasks (title, description, status, priority, category, project_id, habit_id,
                            recurrence_id, start_date, due_date, due_time, estimated_minutes,
                            created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
        params![
            input.title,
            input.description,
            status,
            input.priority.as_deref().unwrap_or("medium"),
            input.category,
            input.project_id,
            input.habit_id,
            recurrence,
            input.start_date,
            input.due_date,
            input.due_time,
            input.estimated_minutes,
            now
        ],
    )?;
    let id = tx.last_insert_rowid();
    set_tags(&tx, id, &input.tags)?;
    set_reminders(&tx, id, &input.reminders)?;
    for (position, title) in input.subtasks.iter().enumerate() {
        let title = title.trim();
        if !title.is_empty() {
            tx.execute(
                "INSERT INTO task_subtasks (task_id, title, position) VALUES (?1, ?2, ?3)",
                params![id, title, position as i64],
            )?;
        }
    }
    tx.commit()?;
    get(conn, clock, id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateParams {
    pub id: i64,
    #[serde(flatten)]
    pub input: TaskInput,
}

/// Replaces a task's fields, tags, reminders and repeat rule. Subtasks have their own operations, and
/// completing or archiving goes through `complete` and `archive` so their bookkeeping always runs.
pub fn update(conn: &Connection, clock: &dyn Clock, params: UpdateParams) -> Result<Task> {
    let UpdateParams { id, mut input } = params;
    validate(&mut input)?;
    let current = get(conn, clock, id)?;
    let status = input.status.clone().unwrap_or_else(|| current.status.clone());
    if !STATUSES.contains(&status.as_str()) {
        return Err(Error::invalid(format!("Unknown status: {status}")));
    }
    if status != current.status && !matches!(status.as_str(), "todo" | "in_progress") {
        return Err(Error::invalid("Use Complete or Archive to change this task's status"));
    }

    let tx = conn.unchecked_transaction()?;
    let recurrence = match (&input.recurrence, &input.due_date, current.recurrence_id) {
        (Some(rule), Some(due), Some(existing)) => {
            tx.execute(
                "UPDATE task_recurrence SET frequency = ?1, interval = ?2, weekdays = ?3, anchor_day = ?4,
                 end_date = ?5, max_count = ?6 WHERE id = ?7",
                params![
                    rule.frequency,
                    rule.interval,
                    weekday_mask(&rule.weekdays),
                    parse_date(due)?.day(),
                    rule.end_date,
                    rule.max_count,
                    existing
                ],
            )?;
            Some(existing)
        }
        (Some(rule), Some(due), None) => Some(insert_recurrence(&tx, rule, parse_date(due)?)?),
        _ => None,
    };
    tx.execute(
        "UPDATE tasks SET title = ?1, description = ?2, status = ?3, priority = ?4, category = ?5,
                project_id = ?6, habit_id = ?7, recurrence_id = ?8, start_date = ?9, due_date = ?10,
                due_time = ?11, estimated_minutes = ?12, updated_at = ?13 WHERE id = ?14",
        params![
            input.title,
            input.description,
            status,
            input.priority.as_deref().unwrap_or(&current.priority),
            input.category,
            input.project_id,
            input.habit_id,
            recurrence,
            input.start_date,
            input.due_date,
            input.due_time,
            input.estimated_minutes,
            format_instant(clock.now()),
            id
        ],
    )?;
    set_tags(&tx, id, &input.tags)?;
    set_reminders(&tx, id, &input.reminders)?;
    tx.commit()?;
    get(conn, clock, id)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completed {
    pub task: Task,
    /// The next occurrence, when completing this one generated it.
    pub next: Option<Task>,
}

pub fn complete(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Completed> {
    let task = get(conn, clock, id)?;
    if task.status == "completed" {
        return Ok(Completed { task, next: None });
    }
    if task.status == "archived" {
        return Err(Error::invalid("Restore this task before completing it"));
    }
    let now = clock.now();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE tasks SET status = 'completed', completed_at = ?1, completed_on = ?2, updated_at = ?1
         WHERE id = ?3",
        params![format_instant(now), format_date(clock.today()), id],
    )?;
    let next = spawn_next(&tx, clock, id)?;
    tx.commit()?;
    let next = next.map(|next_id| get(conn, clock, next_id)).transpose()?;
    Ok(Completed { task: get(conn, clock, id)?, next })
}

/// Generates the next occurrence of a recurring task once, however often this one is completed.
fn spawn_next(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Option<i64>> {
    let row = conn.query_row(
        "SELECT recurrence_id, spawned_next, due_date, start_date FROM tasks WHERE id = ?1",
        [id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    )?;
    let (Some(recurrence_id), false, Some(due_text), start_text) = row else { return Ok(None) };
    let due = parse_date(&due_text)?;

    let (rule, created) = conn.query_row(
        "SELECT frequency, interval, weekdays, anchor_day, end_date, max_count, created_count
         FROM task_recurrence WHERE id = ?1",
        [recurrence_id],
        |row| {
            Ok((
                Rule {
                    frequency: Frequency::parse(&row.get::<_, String>(0)?).unwrap_or(Frequency::Daily),
                    interval: row.get(1)?,
                    weekdays: row.get(2)?,
                    anchor_day: row.get(3)?,
                    end_date: row.get::<_, Option<String>>(4)?.and_then(|text| parse_date(&text).ok()),
                    max_count: row.get(5)?,
                },
                row.get::<_, u32>(6)?,
            ))
        },
    )?;
    let Some(next_due) = rule.next_occurrence(due, clock.today(), created) else { return Ok(None) };

    // A start date keeps its distance from the deadline.
    let next_start =
        start_text.as_deref().map(parse_date).transpose()?.map(|start| format_date(next_due - (due - start)));
    let now = format_instant(clock.now());
    conn.execute(
        "INSERT INTO tasks (title, description, priority, category, project_id, habit_id, recurrence_id,
                            start_date, due_date, due_time, estimated_minutes, created_at, updated_at)
         SELECT title, description, priority, category, project_id, habit_id, recurrence_id,
                ?1, ?2, due_time, estimated_minutes, ?3, ?3
         FROM tasks WHERE id = ?4",
        params![next_start, format_date(next_due), now, id],
    )?;
    let next = conn.last_insert_rowid();
    copy_children(conn, id, next)?;
    conn.execute("UPDATE tasks SET spawned_next = 1 WHERE id = ?1", [id])?;
    conn.execute(
        "UPDATE task_recurrence SET created_count = created_count + 1 WHERE id = ?1",
        [recurrence_id],
    )?;
    Ok(Some(next))
}

/// Copies tags, deadline reminders and unfinished copies of the subtasks from one task to another.
fn copy_children(conn: &Connection, from: i64, to: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO task_tags (task_id, tag_id) SELECT ?1, tag_id FROM task_tags WHERE task_id = ?2",
        [to, from],
    )?;
    conn.execute(
        "INSERT INTO task_reminders (task_id, kind, offset_minutes)
         SELECT ?1, kind, offset_minutes FROM task_reminders WHERE task_id = ?2 AND kind = 'before'",
        [to, from],
    )?;
    let rows: Vec<(i64, Option<i64>, String, i64)> = conn
        .prepare("SELECT id, parent_id, title, position FROM task_subtasks WHERE task_id = ?1 ORDER BY id")?
        .query_map([from], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut mapped: HashMap<i64, i64> = HashMap::new();
    for (old, parent, title, position) in rows {
        conn.execute(
            "INSERT INTO task_subtasks (task_id, parent_id, title, position) VALUES (?1, ?2, ?3, ?4)",
            params![to, parent.and_then(|id| mapped.get(&id)), title, position],
        )?;
        mapped.insert(old, conn.last_insert_rowid());
    }
    Ok(())
}

pub fn reopen(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    get(conn, clock, id)?;
    conn.execute(
        "UPDATE tasks SET status = 'todo', completed_at = NULL, completed_on = NULL, updated_at = ?1
         WHERE id = ?2 AND status = 'completed'",
        params![format_instant(clock.now()), id],
    )?;
    get(conn, clock, id)
}

pub fn archive(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    get(conn, clock, id)?;
    let now = format_instant(clock.now());
    conn.execute(
        "UPDATE tasks SET status_before_archive = status, status = 'archived', archived_at = ?1,
                updated_at = ?1 WHERE id = ?2 AND status != 'archived'",
        params![now, id],
    )?;
    get(conn, clock, id)
}

pub fn restore(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    get(conn, clock, id)?;
    conn.execute(
        "UPDATE tasks SET status = coalesce(status_before_archive, 'todo'), status_before_archive = NULL,
                archived_at = NULL, updated_at = ?1 WHERE id = ?2 AND status = 'archived'",
        params![format_instant(clock.now()), id],
    )?;
    get(conn, clock, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    if tx.execute("DELETE FROM tasks WHERE id = ?1", [id])? == 0 {
        return Err(Error::NotFound("This task"));
    }
    tx.execute("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM task_tags)", [])?;
    tx.execute("DELETE FROM task_recurrence WHERE id NOT IN (SELECT recurrence_id FROM tasks WHERE recurrence_id IS NOT NULL)", [])?;
    tx.commit()?;
    Ok(())
}

/// Copies a task as a new, unfinished one. The copy is a one-off: if it joined the series, two open
/// occurrences would each generate a next one.
pub fn duplicate(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    get(conn, clock, id)?;
    let tx = conn.unchecked_transaction()?;
    let now = format_instant(clock.now());
    tx.execute(
        "INSERT INTO tasks (title, description, priority, category, project_id, habit_id, start_date,
                            due_date, due_time, estimated_minutes, created_at, updated_at)
         SELECT title || ' (copy)', description, priority, category, project_id, habit_id, start_date,
                due_date, due_time, estimated_minutes, ?1, ?1
         FROM tasks WHERE id = ?2",
        params![now, id],
    )?;
    let copy = tx.last_insert_rowid();
    copy_children(&tx, id, copy)?;
    tx.commit()?;
    get(conn, clock, copy)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescheduleParams {
    pub id: i64,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
}

pub fn reschedule(conn: &Connection, clock: &dyn Clock, params: RescheduleParams) -> Result<Task> {
    let task = get(conn, clock, params.id)?;
    let due_date = clean_optional(params.due_date);
    let due_time = clean_optional(params.due_time);
    if let Some(date) = &due_date {
        parse_date(date)?;
    }
    if let Some(time) = &due_time {
        parse_time(time)?;
        if due_date.is_none() {
            return Err(Error::invalid("Choose a due date before setting a time"));
        }
    }
    if due_date.is_none() && (task.recurrence_id.is_some() || task.reminder_count > 0) {
        return Err(Error::invalid("Remove this task's reminders and repeat before clearing its date"));
    }
    // A start date after the new deadline would no longer make sense, so it moves with it.
    let start = match (&task.start_date, &due_date) {
        (Some(start), Some(due)) if start > due => Some(due.clone()),
        (start, _) => start.clone(),
    };
    conn.execute(
        "UPDATE tasks SET due_date = ?1, due_time = ?2, start_date = ?3, updated_at = ?4 WHERE id = ?5",
        params![due_date, due_time, start, format_instant(clock.now()), params.id],
    )?;
    get(conn, clock, params.id)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnoozeParams {
    pub id: i64,
    pub minutes: i64,
}

pub fn snooze(conn: &Connection, clock: &dyn Clock, params: SnoozeParams) -> Result<()> {
    get(conn, clock, params.id)?;
    if !(1..=60 * 24 * 7).contains(&params.minutes) {
        return Err(Error::invalid("Snooze for between a minute and a week"));
    }
    let at = clock.to_local(clock.now() + Duration::minutes(params.minutes));
    conn.execute(
        "INSERT INTO task_reminders (task_id, kind, remind_at, snoozed) VALUES (?1, 'at', ?2, 1)",
        params![params.id, format_datetime(at)],
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskAdd {
    pub task_id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
}

pub fn subtask_add(conn: &Connection, clock: &dyn Clock, params: SubtaskAdd) -> Result<Task> {
    get(conn, clock, params.task_id)?;
    let title = params.title.trim();
    if title.is_empty() {
        return Err(Error::invalid("Give the subtask a title"));
    }
    if let Some(parent) = params.parent_id {
        let owner: Option<i64> = conn
            .query_row("SELECT task_id FROM task_subtasks WHERE id = ?1", [parent], |row| row.get(0))
            .optional()?;
        if owner != Some(params.task_id) {
            return Err(Error::NotFound("This subtask"));
        }
    }
    conn.execute(
        "INSERT INTO task_subtasks (task_id, parent_id, title, position)
         SELECT ?1, ?2, ?3, coalesce(max(position), -1) + 1 FROM task_subtasks WHERE task_id = ?1",
        params![params.task_id, params.parent_id, title],
    )?;
    touch(conn, clock, params.task_id)?;
    get(conn, clock, params.task_id)
}

fn subtask_owner(conn: &Connection, id: i64) -> Result<i64> {
    conn.query_row("SELECT task_id FROM task_subtasks WHERE id = ?1", [id], |row| row.get(0))
        .optional()?
        .ok_or(Error::NotFound("This subtask"))
}

fn touch(conn: &Connection, clock: &dyn Clock, task: i64) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET updated_at = ?1 WHERE id = ?2",
        params![format_instant(clock.now()), task],
    )?;
    Ok(())
}

/// Toggles a subtask together with everything nested under it.
pub fn subtask_toggle(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    let task = subtask_owner(conn, id)?;
    let done: bool =
        conn.query_row("SELECT completed_at IS NOT NULL FROM task_subtasks WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    let value = (!done).then(|| format_instant(clock.now()));
    conn.execute(
        "WITH RECURSIVE tree(id) AS (
             SELECT ?1 UNION ALL
             SELECT s.id FROM task_subtasks s JOIN tree ON s.parent_id = tree.id
         )
         UPDATE task_subtasks SET completed_at = ?2 WHERE id IN (SELECT id FROM tree)",
        params![id, value],
    )?;
    touch(conn, clock, task)?;
    get(conn, clock, task)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskRename {
    pub id: i64,
    pub title: String,
}

pub fn subtask_rename(conn: &Connection, clock: &dyn Clock, params: SubtaskRename) -> Result<Task> {
    let task = subtask_owner(conn, params.id)?;
    let title = params.title.trim();
    if title.is_empty() {
        return Err(Error::invalid("Give the subtask a title"));
    }
    conn.execute("UPDATE task_subtasks SET title = ?1 WHERE id = ?2", params![title, params.id])?;
    touch(conn, clock, task)?;
    get(conn, clock, task)
}

pub fn subtask_delete(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Task> {
    let task = subtask_owner(conn, id)?;
    conn.execute("DELETE FROM task_subtasks WHERE id = ?1", [id])?;
    touch(conn, clock, task)?;
    get(conn, clock, task)
}

pub fn categories(conn: &Connection) -> Result<Vec<String>> {
    Ok(conn
        .prepare(
            "SELECT DISTINCT category FROM tasks WHERE category IS NOT NULL
             ORDER BY category COLLATE NOCASE",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

pub fn tags(conn: &Connection) -> Result<Vec<TagCount>> {
    Ok(conn
        .prepare(
            "SELECT tg.id, tg.name, count(tt.task_id) FROM tags tg
             LEFT JOIN task_tags tt ON tt.tag_id = tg.id
             GROUP BY tg.id ORDER BY tg.name COLLATE NOCASE",
        )?
        .query_map([], |row| Ok(TagCount { id: row.get(0)?, name: row.get(1)?, count: row.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?)
}
