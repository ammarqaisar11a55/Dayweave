//! Sync between devices through a Dayweave server. The database stays the source of truth on each
//! device and everything works offline: changes wait in an outbox until a sync can reach the server.
//!
//! Data travels as records, each with a stable ID shared by every device: a task with its subtasks,
//! reminders and tags; a habit with its schedule and checklist; a check-in; a project; a repeat rule;
//! a finished focus session; a preference. Triggers (migration 0002) mark a record dirty whenever any
//! of its rows change, and record deletions. When two devices change the same record, the newer change
//! wins, by the time it was made.
//!
//! This module only reads and writes the database. The network round trip is in [`crate::Core`].

use std::cmp::Ordering;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::{Error, Result};
use crate::settings;

/// One record as it travels. `data` is empty for a deletion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub entity: String,
    pub id: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub data: Value,
}

/// What applying a batch from the server did.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub applied: usize,
    /// Older than this device's own unsent change, so this device's version was kept.
    pub kept_local: usize,
    /// Couldn't be written, usually because it came from a newer version of Dayweave.
    pub skipped: usize,
}

/// Parents before the records that refer to them.
const ORDER: &[&str] = &["setting", "project", "habit", "recurrence", "task", "completion", "focus"];

struct Spec {
    table: &'static str,
    columns: &'static [&'static str],
    /// Columns holding another record's local ID, which travel as that record's sync ID.
    references: &'static [(&'static str, &'static str)],
}

const PROJECT: Spec = Spec {
    table: "projects",
    columns: &[
        "name",
        "description",
        "color",
        "icon",
        "start_date",
        "deadline",
        "archived_at",
        "created_at",
        "updated_at",
    ],
    references: &[],
};

const HABIT: Spec = Spec {
    table: "habits",
    columns: &[
        "name",
        "description",
        "icon",
        "color",
        "kind",
        "frequency",
        "weekly_target",
        "start_date",
        "reminder_time",
        "quantity_target",
        "unit",
        "position",
        "archived_at",
        "created_at",
        "updated_at",
    ],
    references: &[],
};

const COMPLETION: Spec = Spec {
    table: "habit_completions",
    columns: &["habit_id", "date", "status", "value", "note", "created_at", "updated_at"],
    references: &[("habit_id", "habit")],
};

const RECURRENCE: Spec = Spec {
    table: "task_recurrence",
    columns: &["frequency", "interval", "weekdays", "anchor_day", "end_date", "max_count", "created_count"],
    references: &[],
};

const TASK: Spec = Spec {
    table: "tasks",
    columns: &[
        "title",
        "description",
        "status",
        "status_before_archive",
        "priority",
        "category",
        "project_id",
        "habit_id",
        "recurrence_id",
        "spawned_next",
        "start_date",
        "due_date",
        "due_time",
        "estimated_minutes",
        "created_at",
        "updated_at",
        "completed_at",
        "completed_on",
        "archived_at",
    ],
    references: &[("project_id", "project"), ("habit_id", "habit"), ("recurrence_id", "recurrence")],
};

const FOCUS: Spec = Spec {
    table: "focus_sessions",
    columns: &[
        "kind",
        "status",
        "planned_seconds",
        "elapsed_seconds",
        "task_id",
        "habit_id",
        "local_date",
        "started_at",
        "resumed_at",
        "ended_at",
    ],
    references: &[("task_id", "task"), ("habit_id", "habit")],
};

fn spec(entity: &str) -> Option<&'static Spec> {
    match entity {
        "project" => Some(&PROJECT),
        "habit" => Some(&HABIT),
        "completion" => Some(&COMPLETION),
        "recurrence" => Some(&RECURRENCE),
        "task" => Some(&TASK),
        "focus" => Some(&FOCUS),
        _ => None,
    }
}

// Mapping between local rows and sync IDs

fn sync_id_of(conn: &Connection, entity: &str, local_id: i64) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT sync_id FROM sync_records WHERE entity = ?1 AND local_id = ?2",
            params![entity, local_id.to_string()],
            |row| row.get(0),
        )
        .optional()?)
}

fn local_id_of(conn: &Connection, entity: &str, sync_id: &str) -> Result<Option<i64>> {
    let local: Option<String> = conn
        .query_row(
            "SELECT local_id FROM sync_records WHERE entity = ?1 AND sync_id = ?2",
            params![entity, sync_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(local.and_then(|text| text.parse().ok()))
}

struct Local {
    local_id: String,
    dirty: bool,
    modified_at: String,
}

fn local_record(conn: &Connection, entity: &str, sync_id: &str) -> Result<Option<Local>> {
    Ok(conn
        .query_row(
            "SELECT local_id, dirty, modified_at FROM sync_records WHERE entity = ?1 AND sync_id = ?2",
            params![entity, sync_id],
            |row| {
                Ok(Local {
                    local_id: row.get(0)?,
                    dirty: row.get::<_, i64>(1)? == 1,
                    modified_at: row.get(2)?,
                })
            },
        )
        .optional()?)
}

fn remember(conn: &Connection, entity: &str, local_id: &str, sync_id: &str, modified_at: &str) -> Result<()> {
    // A row can only have one sync ID, and a sync ID one row.
    conn.execute("DELETE FROM sync_records WHERE entity = ?1 AND sync_id = ?2", params![entity, sync_id])?;
    conn.execute(
        "INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty) VALUES (?1, ?2, ?3, ?4, 0)
         ON CONFLICT (entity, local_id) DO UPDATE SET sync_id = excluded.sync_id,
             modified_at = excluded.modified_at, dirty = 0",
        params![entity, local_id, sync_id, modified_at],
    )?;
    Ok(())
}

/// Compares two instants by time rather than as text, since '10:00:00Z' and '10:00:00.250Z' differ in
/// format. An unreadable time counts as the oldest possible.
fn compare_times(left: &str, right: &str) -> Ordering {
    let parse = |text: &str| DateTime::parse_from_rfc3339(text).map(|time| time.with_timezone(&Utc)).ok();
    parse(left).cmp(&parse(right))
}

// Reading records

fn column_value(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => json!(number),
        ValueRef::Real(number) => json!(number),
        ValueRef::Text(text) => Value::String(String::from_utf8_lossy(text).into_owned()),
        ValueRef::Blob(_) => Value::Null,
    }
}

fn read_row(conn: &Connection, spec: &Spec, local_id: i64) -> Result<Option<Map<String, Value>>> {
    let sql = format!("SELECT {} FROM {} WHERE id = ?1", spec.columns.join(", "), spec.table);
    let row = conn
        .query_row(&sql, [local_id], |row| {
            let mut map = Map::new();
            for (index, column) in spec.columns.iter().enumerate() {
                map.insert((*column).to_string(), column_value(row.get_ref(index)?));
            }
            Ok(map)
        })
        .optional()?;
    let Some(mut map) = row else { return Ok(None) };
    for (column, entity) in spec.references {
        let reference = match map.get(*column).and_then(Value::as_i64) {
            Some(id) => sync_id_of(conn, entity, id)?.map(Value::String).unwrap_or(Value::Null),
            None => Value::Null,
        };
        map.insert((*column).to_string(), reference);
    }
    Ok(Some(map))
}

fn export(conn: &Connection, entity: &str, local_id: &str) -> Result<Option<Value>> {
    if entity == "setting" {
        let value: Option<String> = conn
            .query_row("SELECT value FROM app_settings WHERE key = ?1", [local_id], |row| row.get(0))
            .optional()?;
        return Ok(
            value.map(|text| json!({ "value": serde_json::from_str::<Value>(&text).unwrap_or(Value::Null) }))
        );
    }
    let Some(spec) = spec(entity) else { return Ok(None) };
    let Ok(id) = local_id.parse::<i64>() else { return Ok(None) };
    let Some(mut data) = read_row(conn, spec, id)? else { return Ok(None) };

    match entity {
        "habit" => {
            let mut statement =
                conn.prepare("SELECT weekday FROM habit_schedules WHERE habit_id = ?1 ORDER BY weekday")?;
            let weekdays: Vec<i64> =
                statement.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            data.insert("weekdays".into(), json!(weekdays));
            let mut statement = conn.prepare(
                "SELECT i.title, i.position, s.sync_id FROM habit_checklist_items i
                 LEFT JOIN sync_records s ON s.entity = 'checklist_item' AND s.local_id = CAST(i.id AS TEXT)
                 WHERE i.habit_id = ?1 ORDER BY i.position, i.id",
            )?;
            let checklist: Vec<Value> = statement
                .query_map([id], |row| {
                    Ok(json!({ "title": row.get::<_, String>(0)?, "position": row.get::<_, i64>(1)?,
                               "id": row.get::<_, Option<String>>(2)? }))
                })?
                .collect::<rusqlite::Result<_>>()?;
            data.insert("checklist".into(), Value::Array(checklist));
        }
        "completion" => {
            let checked: String =
                conn.query_row("SELECT checked_items FROM habit_completions WHERE id = ?1", [id], |row| {
                    row.get(0)
                })?;
            let local: Vec<i64> = serde_json::from_str(&checked).unwrap_or_default();
            let mut ids = Vec::new();
            for item in local {
                if let Some(sync_id) = sync_id_of(conn, "checklist_item", item)? {
                    ids.push(sync_id);
                }
            }
            data.insert("checked_items".into(), json!(ids));
        }
        "task" => {
            let mut statement = conn
                .prepare("SELECT id, parent_id, title, position, completed_at FROM task_subtasks WHERE task_id = ?1 ORDER BY id")?;
            let subtasks: Vec<Value> = statement
                .query_map([id], |row| {
                    Ok(json!({ "key": row.get::<_, i64>(0)?, "parent": row.get::<_, Option<i64>>(1)?,
                               "title": row.get::<_, String>(2)?, "position": row.get::<_, i64>(3)?,
                               "completed_at": row.get::<_, Option<String>>(4)? }))
                })?
                .collect::<rusqlite::Result<_>>()?;
            data.insert("subtasks".into(), Value::Array(subtasks));
            let mut statement = conn.prepare(
                "SELECT kind, offset_minutes, remind_at FROM task_reminders WHERE task_id = ?1 AND snoozed = 0 ORDER BY id",
            )?;
            let reminders: Vec<Value> = statement
                .query_map([id], |row| {
                    Ok(json!({ "kind": row.get::<_, String>(0)?, "offset_minutes": row.get::<_, Option<i64>>(1)?,
                               "remind_at": row.get::<_, Option<String>>(2)? }))
                })?
                .collect::<rusqlite::Result<_>>()?;
            data.insert("reminders".into(), Value::Array(reminders));
            let mut statement = conn.prepare(
                "SELECT t.name FROM task_tags tt JOIN tags t ON t.id = tt.tag_id WHERE tt.task_id = ?1 ORDER BY t.name",
            )?;
            let tags: Vec<String> =
                statement.query_map([id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            data.insert("tags".into(), json!(tags));
        }
        _ => {}
    }
    Ok(Some(Value::Object(data)))
}

/// Up to `limit` records waiting to be sent: changed records first, then deletions.
pub fn pending(conn: &Connection, limit: usize) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    let mut statement = conn.prepare(
        "SELECT entity, local_id, sync_id, modified_at FROM sync_records
         WHERE dirty = 1 AND entity != 'checklist_item' ORDER BY modified_at LIMIT ?1",
    )?;
    let rows: Vec<(String, String, String, String)> = statement
        .query_map([limit as i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (entity, local_id, sync_id, modified_at) in rows {
        match export(conn, &entity, &local_id)? {
            Some(data) => {
                changes.push(Change { entity, id: sync_id, updated_at: modified_at, deleted: false, data })
            }
            // The row is gone without a deletion being recorded, so its mapping is stale.
            None => {
                conn.execute(
                    "DELETE FROM sync_records WHERE entity = ?1 AND local_id = ?2",
                    params![entity, local_id],
                )?;
            }
        }
    }
    let remaining = limit.saturating_sub(changes.len());
    let mut statement =
        conn.prepare("SELECT entity, sync_id, deleted_at FROM sync_tombstones ORDER BY deleted_at LIMIT ?1")?;
    let deletions = statement.query_map([remaining as i64], |row| {
        Ok(Change {
            entity: row.get(0)?,
            id: row.get(1)?,
            updated_at: row.get(2)?,
            deleted: true,
            data: Value::Null,
        })
    })?;
    for deletion in deletions {
        changes.push(deletion?);
    }
    Ok(changes)
}

pub fn pending_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT (SELECT count(*) FROM sync_records WHERE dirty = 1 AND entity != 'checklist_item')
              + (SELECT count(*) FROM sync_tombstones)",
        [],
        |row| row.get(0),
    )?)
}

/// Clears what the server has accepted. A record changed again while the request was in flight keeps
/// its newer change waiting, because its time no longer matches the one that was sent.
pub fn mark_sent(conn: &Connection, sent: &[Change]) -> Result<()> {
    for change in sent {
        if change.deleted {
            conn.execute(
                "DELETE FROM sync_tombstones WHERE entity = ?1 AND sync_id = ?2 AND deleted_at = ?3",
                params![change.entity, change.id, change.updated_at],
            )?;
        } else {
            conn.execute(
                "UPDATE sync_records SET dirty = 0 WHERE entity = ?1 AND sync_id = ?2 AND modified_at = ?3",
                params![change.entity, change.id, change.updated_at],
            )?;
        }
    }
    Ok(())
}

/// After signing in, everything here is sent to the account, including what an earlier account had.
pub fn mark_all_dirty(conn: &Connection) -> Result<()> {
    conn.execute("UPDATE sync_records SET dirty = 1 WHERE entity != 'checklist_item'", [])?;
    Ok(())
}

// Writing records from the server

fn sql_value(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(flag) => SqlValue::Integer(i64::from(*flag)),
        Value::Number(number) => match number.as_i64() {
            Some(integer) => SqlValue::Integer(integer),
            None => SqlValue::Real(number.as_f64().unwrap_or_default()),
        },
        Value::String(text) => SqlValue::Text(text.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// Writes a record's own row and returns its local ID. Columns missing from the data, as from an
/// older version of Dayweave, keep their current values or their defaults.
fn upsert_row(
    conn: &Connection,
    spec: &Spec,
    data: &Map<String, Value>,
    existing: Option<i64>,
) -> Result<i64> {
    let mut columns = Vec::new();
    let mut values = Vec::new();
    for column in spec.columns {
        let Some(value) = data.get(*column) else { continue };
        let value = match spec.references.iter().find(|(name, _)| name == column) {
            Some((_, entity)) => match value.as_str() {
                Some(sync_id) => {
                    local_id_of(conn, entity, sync_id)?.map(SqlValue::Integer).unwrap_or(SqlValue::Null)
                }
                None => SqlValue::Null,
            },
            None => sql_value(value),
        };
        columns.push(*column);
        values.push(value);
    }
    match existing {
        Some(id) => {
            if !columns.is_empty() {
                let assignments: Vec<String> = columns
                    .iter()
                    .enumerate()
                    .map(|(index, column)| format!("{column} = ?{}", index + 1))
                    .collect();
                let sql = format!(
                    "UPDATE {} SET {} WHERE id = ?{}",
                    spec.table,
                    assignments.join(", "),
                    columns.len() + 1
                );
                values.push(SqlValue::Integer(id));
                conn.execute(&sql, rusqlite::params_from_iter(values))?;
            }
            Ok(id)
        }
        None => {
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                spec.table,
                columns.join(", "),
                crate::db::placeholders(columns.len())
            );
            conn.execute(&sql, rusqlite::params_from_iter(values))?;
            Ok(conn.last_insert_rowid())
        }
    }
}

fn write_habit_parts(conn: &Connection, habit_id: i64, data: &Map<String, Value>) -> Result<()> {
    if let Some(weekdays) = data.get("weekdays").and_then(Value::as_array) {
        conn.execute("DELETE FROM habit_schedules WHERE habit_id = ?1", [habit_id])?;
        for day in weekdays.iter().filter_map(Value::as_i64) {
            conn.execute(
                "INSERT OR IGNORE INTO habit_schedules (habit_id, weekday) VALUES (?1, ?2)",
                params![habit_id, day],
            )?;
        }
    }
    let Some(checklist) = data.get("checklist").and_then(Value::as_array) else { return Ok(()) };
    // Items keep their IDs, because check-ins refer to them.
    let mut kept = Vec::new();
    for item in checklist {
        let title = item.get("title").and_then(Value::as_str).unwrap_or_default();
        let position = item.get("position").and_then(Value::as_i64).unwrap_or_default();
        let sync_id = item.get("id").and_then(Value::as_str);
        let existing = match sync_id {
            Some(sync_id) => local_id_of(conn, "checklist_item", sync_id)?,
            None => None,
        };
        let local = match existing {
            Some(id) => {
                conn.execute(
                    "UPDATE habit_checklist_items SET title = ?2, position = ?3, habit_id = ?4 WHERE id = ?1",
                    params![id, title, position, habit_id],
                )?;
                id
            }
            None => {
                conn.execute(
                    "INSERT INTO habit_checklist_items (habit_id, title, position) VALUES (?1, ?2, ?3)",
                    params![habit_id, title, position],
                )?;
                conn.last_insert_rowid()
            }
        };
        if let Some(sync_id) = sync_id {
            remember(conn, "checklist_item", &local.to_string(), sync_id, "1970-01-01T00:00:00Z")?;
        }
        kept.push(local);
    }
    let mut statement = conn.prepare("SELECT id FROM habit_checklist_items WHERE habit_id = ?1")?;
    let current: Vec<i64> =
        statement.query_map([habit_id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    for id in current.into_iter().filter(|id| !kept.contains(id)) {
        conn.execute("DELETE FROM habit_checklist_items WHERE id = ?1", [id])?;
        conn.execute(
            "DELETE FROM sync_records WHERE entity = 'checklist_item' AND local_id = ?1",
            [id.to_string()],
        )?;
    }
    Ok(())
}

fn write_task_parts(conn: &Connection, task_id: i64, data: &Map<String, Value>) -> Result<()> {
    if let Some(subtasks) = data.get("subtasks").and_then(Value::as_array) {
        conn.execute("DELETE FROM task_subtasks WHERE task_id = ?1", [task_id])?;
        // A subtask can nest under another, so parents are written first, a level at a time.
        let mut created: HashMap<i64, i64> = HashMap::new();
        let mut remaining: Vec<&Value> = subtasks.iter().collect();
        while !remaining.is_empty() {
            let before = remaining.len();
            let mut next = Vec::new();
            for subtask in remaining {
                let parent = subtask.get("parent").and_then(Value::as_i64);
                let parent_id = match parent {
                    Some(key) => match created.get(&key) {
                        Some(id) => Some(*id),
                        None if subtasks
                            .iter()
                            .any(|other| other.get("key").and_then(Value::as_i64) == Some(key)) =>
                        {
                            next.push(subtask);
                            continue;
                        }
                        None => None,
                    },
                    None => None,
                };
                conn.execute(
                    "INSERT INTO task_subtasks (task_id, parent_id, title, position, completed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        task_id,
                        parent_id,
                        subtask.get("title").and_then(Value::as_str).unwrap_or_default(),
                        subtask.get("position").and_then(Value::as_i64).unwrap_or_default(),
                        subtask.get("completed_at").and_then(Value::as_str),
                    ],
                )?;
                if let Some(key) = subtask.get("key").and_then(Value::as_i64) {
                    created.insert(key, conn.last_insert_rowid());
                }
            }
            if next.len() == before {
                break; // A cycle in the data. What remains can't be placed.
            }
            remaining = next;
        }
    }
    if let Some(reminders) = data.get("reminders").and_then(Value::as_array) {
        conn.execute("DELETE FROM task_reminders WHERE task_id = ?1 AND snoozed = 0", [task_id])?;
        for reminder in reminders {
            conn.execute(
                "INSERT INTO task_reminders (task_id, kind, offset_minutes, remind_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    task_id,
                    reminder.get("kind").and_then(Value::as_str).unwrap_or_default(),
                    reminder.get("offset_minutes").and_then(Value::as_i64),
                    reminder.get("remind_at").and_then(Value::as_str),
                ],
            )?;
        }
    }
    if let Some(tags) = data.get("tags").and_then(Value::as_array) {
        conn.execute("DELETE FROM task_tags WHERE task_id = ?1", [task_id])?;
        for name in tags.iter().filter_map(Value::as_str).map(str::trim).filter(|name| !name.is_empty()) {
            conn.execute("INSERT INTO tags (name) VALUES (?1) ON CONFLICT (name) DO NOTHING", [name])?;
            let tag: i64 = conn.query_row("SELECT id FROM tags WHERE name = ?1", [name], |row| row.get(0))?;
            conn.execute(
                "INSERT OR IGNORE INTO task_tags (task_id, tag_id) VALUES (?1, ?2)",
                params![task_id, tag],
            )?;
        }
    }
    Ok(())
}

fn apply_one(conn: &Connection, change: &Change) -> Result<()> {
    let local = local_record(conn, &change.entity, &change.id)?;

    if change.deleted {
        if let Some(local) = local {
            if let Some(spec) = spec(&change.entity) {
                conn.execute(&format!("DELETE FROM {} WHERE id = ?1", spec.table), [&local.local_id])?;
            }
            conn.execute(
                "DELETE FROM sync_records WHERE entity = ?1 AND sync_id = ?2",
                params![change.entity, change.id],
            )?;
        }
        return Ok(());
    }

    let data = change.data.as_object().ok_or_else(|| Error::invalid("A synced record has no data"))?;
    if change.entity == "setting" {
        let value = data.get("value").cloned().unwrap_or(Value::Null);
        settings::set(conn, settings::SetParams { key: change.id.clone(), value })?;
        return remember(conn, "setting", &change.id, &change.id, &change.updated_at);
    }

    let spec = spec(&change.entity)
        .ok_or_else(|| Error::invalid(format!("Unknown record type {}", change.entity)))?;
    let mut existing = local.as_ref().and_then(|local| local.local_id.parse::<i64>().ok());
    if change.entity == "completion" && existing.is_none() {
        // The same habit and day, logged here before this device knew the server's record.
        let habit = data
            .get("habit_id")
            .and_then(Value::as_str)
            .map(|id| local_id_of(conn, "habit", id))
            .transpose()?;
        let Some(Some(habit)) = habit else { return Ok(()) }; // Its habit is gone.
        existing = conn
            .query_row(
                "SELECT id FROM habit_completions WHERE habit_id = ?1 AND date = ?2",
                params![habit, data.get("date").and_then(Value::as_str)],
                |row| row.get(0),
            )
            .optional()?;
    }
    let id = upsert_row(conn, spec, data, existing)?;

    match change.entity.as_str() {
        "habit" => write_habit_parts(conn, id, data)?,
        "task" => write_task_parts(conn, id, data)?,
        "completion" => {
            if let Some(items) = data.get("checked_items").and_then(Value::as_array) {
                let mut local_items = Vec::new();
                for item in items.iter().filter_map(Value::as_str) {
                    if let Some(local) = local_id_of(conn, "checklist_item", item)? {
                        local_items.push(local);
                    }
                }
                conn.execute(
                    "UPDATE habit_completions SET checked_items = ?2 WHERE id = ?1",
                    params![id, serde_json::to_string(&local_items)?],
                )?;
            }
        }
        _ => {}
    }
    remember(conn, &change.entity, &id.to_string(), &change.id, &change.updated_at)
}

/// Writes changes from the server. A change older than this device's own unsent change to the same
/// record is ignored, and this device's version is sent instead. A record that can't be written is
/// skipped without affecting the rest.
pub fn apply(conn: &Connection, changes: &[Change]) -> Result<Applied> {
    let mut sorted: Vec<&Change> = changes.iter().collect();
    sorted.sort_by_key(|change| {
        ORDER.iter().position(|entity| *entity == change.entity).unwrap_or(ORDER.len())
    });

    let mut summary = Applied::default();
    let tx = conn.unchecked_transaction()?;
    tx.execute("UPDATE sync_guard SET applying = 1", [])?;
    for change in sorted {
        if let Some(local) = local_record(&tx, &change.entity, &change.id)? {
            if local.dirty && compare_times(&local.modified_at, &change.updated_at) == Ordering::Greater {
                summary.kept_local += 1;
                continue;
            }
        }
        let deleted_here: Option<String> = tx
            .query_row(
                "SELECT deleted_at FROM sync_tombstones WHERE entity = ?1 AND sync_id = ?2",
                params![change.entity, change.id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(deleted_at) = deleted_here {
            if compare_times(&deleted_at, &change.updated_at) == Ordering::Greater {
                summary.kept_local += 1;
                continue;
            }
            tx.execute(
                "DELETE FROM sync_tombstones WHERE entity = ?1 AND sync_id = ?2",
                params![change.entity, change.id],
            )?;
        }

        tx.execute("SAVEPOINT sync_record", [])?;
        match apply_one(&tx, change) {
            Ok(()) => {
                tx.execute("RELEASE sync_record", [])?;
                summary.applied += 1;
            }
            Err(error) => {
                tx.execute("ROLLBACK TO sync_record", [])?;
                tx.execute("RELEASE sync_record", [])?;
                eprintln!("skipped a synced {} record: {error}", change.entity);
                summary.skipped += 1;
            }
        }
    }
    // Rows removed along with their parents leave mappings behind.
    tx.execute(
        "DELETE FROM sync_records WHERE entity = 'completion'
           AND CAST(local_id AS INTEGER) NOT IN (SELECT id FROM habit_completions)",
        [],
    )?;
    tx.execute(
        "DELETE FROM sync_records WHERE entity = 'checklist_item'
           AND CAST(local_id AS INTEGER) NOT IN (SELECT id FROM habit_checklist_items)",
        [],
    )?;
    tx.execute("UPDATE sync_guard SET applying = 0", [])?;
    tx.commit()?;
    Ok(summary)
}

// Account state, kept in internal settings so it never appears among preferences

pub const SERVER: &str = "internal.sync.server";
pub const TOKEN: &str = "internal.sync.token";
pub const EMAIL: &str = "internal.sync.email";
pub const CURSOR: &str = "internal.sync.cursor";
pub const LAST_SYNC: &str = "internal.sync.lastSyncAt";
pub const LAST_ERROR: &str = "internal.sync.lastError";

pub struct Account {
    pub server: String,
    pub token: String,
}

pub fn account(conn: &Connection) -> Result<Option<Account>> {
    let server = settings::get_internal(conn, SERVER)?;
    let token = settings::get_internal(conn, TOKEN)?;
    Ok(match (server, token) {
        (Some(server), Some(token)) if !token.is_empty() => Some(Account { server, token }),
        _ => None,
    })
}

pub fn cursor(conn: &Connection) -> Result<i64> {
    Ok(settings::get_internal(conn, CURSOR)?.and_then(|text| text.parse().ok()).unwrap_or(0))
}

pub fn forget_account(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM app_settings WHERE key IN (?1, ?2, ?3, ?4, ?5)",
        params![TOKEN, EMAIL, CURSOR, LAST_SYNC, LAST_ERROR],
    )?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub signed_in: bool,
    pub server: Option<String>,
    pub email: Option<String>,
    pub last_sync_at: Option<String>,
    pub last_error: Option<String>,
    pub pending: i64,
}

pub fn status(conn: &Connection) -> Result<Status> {
    let signed_in = account(conn)?.is_some();
    Ok(Status {
        signed_in,
        server: settings::get_internal(conn, SERVER)?,
        email: if signed_in { settings::get_internal(conn, EMAIL)? } else { None },
        last_sync_at: settings::get_internal(conn, LAST_SYNC)?,
        last_error: settings::get_internal(conn, LAST_ERROR)?,
        pending: pending_count(conn)?,
    })
}

/// The server's address, cleaned up. Plain HTTP is accepted only for a server on this machine or the
/// Android emulator's host, so a password is never sent unencrypted over a network.
pub fn normalize_server(address: &str) -> Result<String> {
    let trimmed = address.trim().trim_end_matches('/');
    let with_scheme =
        if trimmed.contains("://") { trimmed.to_string() } else { format!("https://{trimmed}") };
    let rest = with_scheme
        .strip_prefix("https://")
        .map(|rest| (true, rest))
        .or_else(|| with_scheme.strip_prefix("http://").map(|rest| (false, rest)));
    let Some((secure, rest)) = rest else {
        return Err(Error::invalid("The server address must start with https://"));
    };
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    if host.is_empty() {
        return Err(Error::invalid("Enter the server's address, like https://dayweave.example.com"));
    }
    if !secure && !matches!(host, "localhost" | "127.0.0.1" | "10.0.2.2") {
        return Err(Error::invalid("The server address must start with https://"));
    }
    Ok(with_scheme)
}
