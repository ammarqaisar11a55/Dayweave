//! Export, validation and restore. A restore never touches the live database until the incoming data
//! has been loaded, migrated and checked in a throwaway one. See ARCHITECTURE.md.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use crate::clock::{format_instant, Clock};
use crate::db::{self, placeholders, APPLICATION_ID, SCHEMA_VERSION, TABLES};
use crate::error::{Error, Result};
use crate::settings;

const FORMAT: &str = "dayweave-backup";
const FORMAT_VERSION: u32 = 1;
/// How many automatic safety copies to keep.
const SAFETY_COPIES: usize = 5;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub format: String,
    pub format_version: u32,
    pub app_version: String,
    pub schema_version: i64,
    pub exported_at: String,
    pub tables: BTreeMap<String, Vec<Map<String, Value>>>,
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0),
    )? > 0)
}

fn read_table(conn: &Connection, table: &str) -> Result<Vec<Map<String, Value>>> {
    let mut statement = conn.prepare(&format!("SELECT * FROM {table}"))?;
    let names: Vec<String> = statement.column_names().map(|name| name.to_string()).collect();
    let mut rows = statement.query([])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let mut object = Map::with_capacity(names.len());
        for (index, name) in names.iter().enumerate() {
            let value = match row.get_ref(index)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(number) => Value::from(number),
                ValueRef::Real(number) => Number::from_f64(number).map_or(Value::Null, Value::Number),
                ValueRef::Text(text) => Value::String(String::from_utf8_lossy(text).into_owned()),
                ValueRef::Blob(_) => return Err(Error::invalid("Backups do not support binary values")),
            };
            object.insert(name.clone(), value);
        }
        result.push(object);
    }
    Ok(result)
}

pub fn dump(conn: &Connection, clock: &dyn Clock) -> Result<Document> {
    let mut tables = BTreeMap::new();
    for table in TABLES {
        if table_exists(conn, table)? {
            tables.insert((*table).to_string(), read_table(conn, table)?);
        }
    }
    Ok(Document {
        format: FORMAT.into(),
        format_version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        schema_version: db::schema_version(conn)?,
        exported_at: format_instant(clock.now()),
        tables,
    })
}

fn to_sql(value: &Value) -> Result<SqlValue> {
    Ok(match value {
        Value::Null => SqlValue::Null,
        Value::Bool(flag) => SqlValue::Integer(i64::from(*flag)),
        Value::Number(number) => match number.as_i64() {
            Some(integer) => SqlValue::Integer(integer),
            None => SqlValue::Real(number.as_f64().unwrap_or_default()),
        },
        Value::String(text) => SqlValue::Text(text.clone()),
        _ => return Err(Error::invalid("This backup contains a value Dayweave doesn't recognise")),
    })
}

fn columns_of(conn: &Connection, table: &str) -> Result<Vec<String>> {
    Ok(conn
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// Replaces every table's contents. Column names come from the file, so each is checked against the
/// real table before it reaches any SQL.
fn replace_all(conn: &Connection, document: &Document) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    // Children may be inserted before the rows they point at, so checks wait until commit.
    tx.pragma_update(None, "defer_foreign_keys", true)?;
    for table in TABLES.iter().rev() {
        if table_exists(&tx, table)? {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
    }
    for table in document.tables.keys() {
        if !TABLES.contains(&table.as_str()) {
            return Err(Error::invalid(format!("This backup contains an unknown table: {table}")));
        }
    }
    for table in TABLES {
        let Some(rows) = document.tables.get(*table) else { continue };
        if !table_exists(&tx, table)? {
            return Err(Error::invalid(format!("This backup has data for a missing table: {table}")));
        }
        let known = columns_of(&tx, table)?;
        for row in rows {
            if let Some(unknown) = row.keys().find(|name| !known.contains(name)) {
                return Err(Error::invalid(format!("This backup has an unknown column: {table}.{unknown}")));
            }
            let names: Vec<&str> = row.keys().map(String::as_str).collect();
            let values = row.values().map(to_sql).collect::<Result<Vec<_>>>()?;
            let sql =
                format!("INSERT INTO {table} ({}) VALUES ({})", names.join(", "), placeholders(names.len()));
            tx.execute(&sql, params_from_iter(values.iter())).map_err(|error| {
                Error::invalid(format!("This backup has an invalid row in {table}: {error}"))
            })?;
        }
    }
    tx.commit()
        .map_err(|_| Error::invalid("This backup is inconsistent: some records point at missing ones"))?;
    Ok(())
}

fn check_integrity(conn: &Connection) -> Result<()> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result != "ok" {
        return Err(Error::invalid("This backup file is damaged"));
    }
    let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
    if statement.exists([])? {
        return Err(Error::invalid("This backup is inconsistent: some records point at missing ones"));
    }
    Ok(())
}

/// Loads a document into a throwaway database at its own schema, then migrates it to the current one.
fn validated(document: &Document) -> Result<Connection> {
    if document.format != FORMAT {
        return Err(Error::invalid("This file is not a Dayweave backup"));
    }
    if document.format_version > FORMAT_VERSION || document.schema_version > SCHEMA_VERSION {
        return Err(Error::invalid("This backup was made by a newer version of Dayweave"));
    }
    if document.schema_version < 1 {
        return Err(Error::invalid("This backup has no valid schema version"));
    }
    let conn = Connection::open_in_memory()?;
    db::configure(&conn)?;
    db::migrate_to(&conn, document.schema_version)?;
    replace_all(&conn, document)?;
    db::migrate(&conn)?;
    check_integrity(&conn)?;
    Ok(conn)
}

fn read_document(path: &Path, clock: &dyn Clock) -> Result<(Document, &'static str)> {
    let bytes = fs::read(path)?;
    if bytes.starts_with(b"SQLite format 3\0") {
        return Ok((read_sqlite(path, clock)?, "database"));
    }
    let document =
        serde_json::from_slice(&bytes).map_err(|_| Error::invalid("This file is not a Dayweave backup"))?;
    Ok((document, "json"))
}

fn read_sqlite(path: &Path, clock: &dyn Clock) -> Result<Document> {
    let conn = db::open_read_only(path).map_err(|_| Error::invalid("This backup file can't be opened"))?;
    let application: i32 = conn.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if application != APPLICATION_ID {
        return Err(Error::invalid("This database is not a Dayweave backup"));
    }
    if db::schema_version(&conn)? > SCHEMA_VERSION {
        return Err(Error::invalid("This backup was made by a newer version of Dayweave"));
    }
    check_integrity(&conn)?;
    dump(&conn, clock)
}

pub fn export_json(conn: &Connection, clock: &dyn Clock, path: &Path) -> Result<()> {
    let document = dump(conn, clock)?;
    write_atomic(path, serde_json::to_string_pretty(&document)?.as_bytes())
}

/// A full backup: a consistent copy of the database, including uncommitted write-ahead log contents.
pub fn export_database(conn: &Connection, path: &Path) -> Result<()> {
    let temporary = sibling(path, "tmp");
    let _ = fs::remove_file(&temporary);
    conn.execute("VACUUM INTO ?1", [temporary.to_string_lossy()])?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

/// Writes through a temporary file, so a failure can't leave a half-written backup at `path`.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = sibling(path, "tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn csv_row(fields: &[String]) -> String {
    let row: Vec<String> = fields.iter().map(|field| csv_field(field)).collect();
    row.join(",") + "\n"
}

pub fn export_tasks_csv(conn: &Connection, path: &Path) -> Result<()> {
    let mut output = csv_row(
        &[
            "Title",
            "Description",
            "Status",
            "Priority",
            "Category",
            "Project",
            "Tags",
            "Start date",
            "Due date",
            "Due time",
            "Created",
            "Completed",
        ]
        .map(String::from),
    );
    let mut statement = conn.prepare(
        "SELECT t.title, t.description, t.status, t.priority, coalesce(t.category, ''), coalesce(p.name, ''),
                coalesce((SELECT group_concat(tg.name, ' ') FROM task_tags tt JOIN tags tg ON tg.id = tt.tag_id
                          WHERE tt.task_id = t.id), ''),
                coalesce(t.start_date, ''), coalesce(t.due_date, ''), coalesce(t.due_time, ''), t.created_at,
                coalesce(t.completed_at, '')
         FROM tasks t LEFT JOIN projects p ON p.id = t.project_id ORDER BY t.id",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let fields = (0..12).map(|index| row.get(index)).collect::<rusqlite::Result<Vec<String>>>()?;
        output.push_str(&csv_row(&fields));
    }
    write_atomic(path, output.as_bytes())
}

pub fn export_habits_csv(conn: &Connection, path: &Path) -> Result<()> {
    let mut output = csv_row(&["Habit", "Date", "Status", "Value", "Unit", "Note"].map(String::from));
    let mut statement = conn.prepare(
        "SELECT h.name, c.date, c.status, coalesce(c.value, ''), h.unit, c.note
         FROM habit_completions c JOIN habits h ON h.id = c.habit_id ORDER BY h.name, c.date",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let value = match row.get_ref(3)? {
            ValueRef::Real(number) => number.to_string(),
            ValueRef::Integer(number) => number.to_string(),
            _ => String::new(),
        };
        output.push_str(&csv_row(&[row.get(0)?, row.get(1)?, row.get(2)?, value, row.get(4)?, row.get(5)?]));
    }
    write_atomic(path, output.as_bytes())
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub tasks: i64,
    pub habits: i64,
    pub habit_entries: i64,
    pub projects: i64,
    pub focus_sessions: i64,
}

fn counts(conn: &Connection) -> Result<Counts> {
    let count = |table: &str| -> Result<i64> {
        Ok(conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0))?)
    };
    Ok(Counts {
        tasks: count("tasks")?,
        habits: count("habits")?,
        habit_entries: count("habit_completions")?,
        projects: count("projects")?,
        focus_sessions: count("focus_sessions")?,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub kind: String,
    pub exported_at: String,
    pub app_version: String,
    pub schema_version: i64,
    pub incoming: Counts,
    pub current: Counts,
}

/// Validates a backup and describes it, without changing anything.
pub fn preview(conn: &Connection, clock: &dyn Clock, path: &Path) -> Result<Preview> {
    let (document, kind) = read_document(path, clock)?;
    let incoming = validated(&document)?;
    Ok(Preview {
        kind: kind.into(),
        exported_at: document.exported_at,
        app_version: document.app_version,
        schema_version: document.schema_version,
        incoming: counts(&incoming)?,
        current: counts(conn)?,
    })
}

/// Replaces all data with a backup's, after writing a safety copy of the current database.
pub fn restore(
    conn: &Connection,
    clock: &dyn Clock,
    data_dir: Option<&Path>,
    path: &Path,
) -> Result<Option<PathBuf>> {
    let (document, _) = read_document(path, clock)?;
    let incoming = validated(&document)?;
    let migrated = dump(&incoming, clock)?;

    let safety = data_dir.map(|dir| write_safety_copy(conn, clock, dir)).transpose()?;
    replace_all(conn, &migrated)?;
    // Reminders that fell due before the backup was made must not all fire at once.
    settings::set_internal(conn, settings::LAST_TICK, &format_instant(clock.now()))?;
    Ok(safety)
}

fn write_safety_copy(conn: &Connection, clock: &dyn Clock, data_dir: &Path) -> Result<PathBuf> {
    let directory = data_dir.join("backups");
    fs::create_dir_all(&directory)?;
    let stamp = clock.now().format("%Y%m%d-%H%M%S");
    let path = directory.join(format!("before-restore-{stamp}.db"));
    export_database(conn, &path)?;

    let mut copies: Vec<PathBuf> = fs::read_dir(&directory)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("before-restore-"))
        })
        .collect();
    copies.sort();
    let excess = copies.len().saturating_sub(SAFETY_COPIES);
    for old in copies.into_iter().take(excess) {
        let _ = fs::remove_file(old);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_fields_are_quoted_only_when_needed() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("two\nlines"), "\"two\nlines\"");
    }
}
