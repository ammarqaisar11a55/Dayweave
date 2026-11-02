use std::fs;
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::error::{Error, Result};

/// Marks a file as a Dayweave database, so a restore can reject any other SQLite file.
pub const APPLICATION_ID: i32 = 0x4457_4156;

pub const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_initial.sql")];

pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Every table, parents before children, which is the order a restore must insert them in.
pub const TABLES: &[&str] = &[
    "projects",
    "habits",
    "habit_schedules",
    "habit_checklist_items",
    "habit_completions",
    "task_recurrence",
    "tasks",
    "task_subtasks",
    "task_reminders",
    "tags",
    "task_tags",
    "focus_sessions",
    "app_settings",
    "notification_log",
];

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existed = path.exists();
    let conn = Connection::open(path)?;
    configure(&conn)?;
    let version = schema_version(&conn)?;
    if version > SCHEMA_VERSION {
        return Err(Error::DatabaseTooNew);
    }
    if existed && version > 0 && version < SCHEMA_VERSION {
        let copy = path.with_extension(format!("before-v{SCHEMA_VERSION}.db"));
        let _ = fs::remove_file(&copy);
        conn.execute("VACUUM INTO ?1", [copy.to_string_lossy()])?;
    }
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn open_read_only(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?)
}

pub fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    // Reports the mode rather than failing on an in-memory database.
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

pub fn migrate(conn: &Connection) -> Result<()> {
    migrate_to(conn, SCHEMA_VERSION)
}

/// Applies migrations one transaction at a time, so a failure leaves the last good version intact.
pub fn migrate_to(conn: &Connection, target: i64) -> Result<()> {
    let current = schema_version(conn)?;
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as i64 + 1;
        if version <= current || version > target {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.pragma_update(None, "application_id", APPLICATION_ID)?;
        tx.commit()?;
    }
    Ok(())
}

/// The `?, ?, ?` list for an `IN (...)` clause of `count` values.
pub fn placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_database_reaches_the_latest_schema() {
        let conn = open_in_memory().unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
        for table in TABLES {
            conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
        }
    }

    #[test]
    fn every_table_in_the_schema_is_part_of_a_backup() {
        let conn = open_in_memory().unwrap();
        let mut statement = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
            .unwrap();
        let mut found: Vec<String> =
            statement.query_map([], |row| row.get(0)).unwrap().map(|name| name.unwrap()).collect();
        found.sort();
        let mut expected: Vec<&str> = TABLES.to_vec();
        expected.sort();
        assert_eq!(found, expected);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let conn = open_in_memory().unwrap();
        let result = conn.execute("INSERT INTO task_subtasks (task_id, title) VALUES (999, 'Orphan')", []);
        assert!(result.is_err());
    }

    #[test]
    fn a_database_from_a_newer_version_is_refused_and_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dayweave.db");
        {
            let conn = open(&path).unwrap();
            conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1).unwrap();
        }
        assert!(matches!(open(&path), Err(Error::DatabaseTooNew)));
        let conn = open_read_only(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION + 1);
    }

    #[test]
    fn data_survives_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dayweave.db");
        {
            let conn = open(&path).unwrap();
            conn.execute("INSERT INTO app_settings (key, value) VALUES ('a', '1')", []).unwrap();
        }
        let conn = open(&path).unwrap();
        let value: String =
            conn.query_row("SELECT value FROM app_settings WHERE key = 'a'", [], |row| row.get(0)).unwrap();
        assert_eq!(value, "1");
    }
}
