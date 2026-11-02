use chrono::Weekday;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::clock::parse_time;
use crate::error::{Error, Result};

/// Keys starting with this prefix are internal state rather than preferences.
const INTERNAL_PREFIX: &str = "internal.";
pub const LAST_TICK: &str = "internal.lastTick";

fn defaults() -> Map<String, Value> {
    let value = json!({
        "theme": "system",
        "weekStart": "monday",
        "timeFormat": "24h",
        "taskDensity": "comfortable",
        "sidebarCollapsed": false,
        "reduceMotion": false,
        "defaultPriority": "medium",
        "defaultReminderMinutes": 30,
        "confirmDelete": true,
        "closeToBackground": true,
        "startAtLogin": false,
        "notificationsEnabled": true,
        "taskNotifications": true,
        "habitNotifications": true,
        "focusNotifications": true,
        "dateOnlyReminderTime": "09:00",
        "focusMinutes": 25,
        "shortBreakMinutes": 5,
        "longBreakMinutes": 15,
        "sessionsBeforeLongBreak": 4,
    });
    match value {
        Value::Object(map) => map,
        _ => unreachable!("defaults are an object"),
    }
}

fn validate(key: &str, value: &Value) -> Result<()> {
    let one_of = |allowed: &[&str]| value.as_str().is_some_and(|text| allowed.contains(&text));
    let minutes = |low: u64, high: u64| value.as_u64().is_some_and(|number| (low..=high).contains(&number));
    let valid = match key {
        "theme" => one_of(&["system", "light", "dark"]),
        "weekStart" => one_of(&["monday", "sunday", "saturday"]),
        "timeFormat" => one_of(&["12h", "24h"]),
        "taskDensity" => one_of(&["comfortable", "compact"]),
        "defaultPriority" => one_of(&["low", "medium", "high", "urgent"]),
        "defaultReminderMinutes" => value.is_null() || minutes(0, 60 * 24 * 30),
        "dateOnlyReminderTime" => value.as_str().is_some_and(|text| parse_time(text).is_ok()),
        "focusMinutes" => minutes(1, 240),
        "shortBreakMinutes" => minutes(1, 60),
        "longBreakMinutes" => minutes(1, 120),
        "sessionsBeforeLongBreak" => minutes(1, 12),
        _ => value.is_boolean(),
    };
    if valid {
        Ok(())
    } else {
        Err(Error::invalid(format!("{value} is not a valid value for {key}")))
    }
}

pub fn all(conn: &Connection) -> Result<Value> {
    let mut merged = defaults();
    let mut statement = conn.prepare("SELECT key, value FROM app_settings")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (key, value) = row?;
        if merged.contains_key(&key) {
            merged.insert(key, serde_json::from_str(&value)?);
        }
    }
    Ok(Value::Object(merged))
}

#[derive(Deserialize)]
pub struct SetParams {
    pub key: String,
    pub value: Value,
}

pub fn set(conn: &Connection, params: SetParams) -> Result<Value> {
    if !defaults().contains_key(&params.key) {
        return Err(Error::invalid(format!("Unknown setting: {}", params.key)));
    }
    validate(&params.key, &params.value)?;
    write(conn, &params.key, &params.value)?;
    all(conn)
}

fn write(conn: &Connection, key: &str, value: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value.to_string()],
    )?;
    Ok(())
}

pub fn get_bool(conn: &Connection, key: &str) -> Result<bool> {
    Ok(all(conn)?[key].as_bool().unwrap_or(false))
}

pub fn get_string(conn: &Connection, key: &str) -> Result<String> {
    Ok(all(conn)?[key].as_str().unwrap_or_default().to_string())
}

pub fn week_start(conn: &Connection) -> Result<Weekday> {
    Ok(match get_string(conn, "weekStart")?.as_str() {
        "sunday" => Weekday::Sun,
        "saturday" => Weekday::Sat,
        _ => Weekday::Mon,
    })
}

pub fn get_internal(conn: &Connection, key: &str) -> Result<Option<String>> {
    debug_assert!(key.starts_with(INTERNAL_PREFIX));
    let value = conn
        .query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |row| row.get::<_, String>(0))
        .optional()?;
    Ok(value.and_then(|text| serde_json::from_str(&text).ok()))
}

pub fn set_internal(conn: &Connection, key: &str, value: &str) -> Result<()> {
    debug_assert!(key.starts_with(INTERNAL_PREFIX));
    write(conn, key, &Value::String(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn set_value(conn: &Connection, key: &str, value: Value) -> Result<Value> {
        set(conn, SetParams { key: key.into(), value })
    }

    #[test]
    fn defaults_apply_until_a_setting_is_changed() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(all(&conn).unwrap()["theme"], "system");
        set_value(&conn, "theme", json!("dark")).unwrap();
        assert_eq!(all(&conn).unwrap()["theme"], "dark");
    }

    #[test]
    fn invalid_values_and_unknown_keys_are_rejected() {
        let conn = db::open_in_memory().unwrap();
        assert!(set_value(&conn, "theme", json!("blue")).is_err());
        assert!(set_value(&conn, "focusMinutes", json!(0)).is_err());
        assert!(set_value(&conn, "startAtLogin", json!("yes")).is_err());
        assert!(set_value(&conn, "nonsense", json!(true)).is_err());
    }

    #[test]
    fn internal_state_never_appears_among_preferences() {
        let conn = db::open_in_memory().unwrap();
        set_internal(&conn, LAST_TICK, "2026-11-02T10:00:00Z").unwrap();
        assert_eq!(get_internal(&conn, LAST_TICK).unwrap().unwrap(), "2026-11-02T10:00:00Z");
        assert!(all(&conn).unwrap().get(LAST_TICK).is_none());
    }
}
