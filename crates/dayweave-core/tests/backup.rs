mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono_tz::Asia::Karachi;
use common::App;
use dayweave_core::clock::ManualClock;
use dayweave_core::Core;
use serde_json::{json, Value};

fn path_param(path: &Path) -> Value {
    json!({ "path": path })
}

fn populate(app: &App) {
    let project = app.call("projects.create", json!({ "name": "Thesis", "color": "blue" }));
    let parent = app.task(
        "Write \"chapter, one\"",
        json!({
            "description": "Line one\nLine two",
            "projectId": project["id"],
            "dueDate": "2026-11-05",
            "dueTime": "09:00",
            "tags": ["writing"],
            "subtasks": ["Outline"],
            "reminders": [{ "kind": "before", "offsetMinutes": 30 }],
            "recurrence": { "frequency": "weekly", "weekdays": [0, 3] },
        }),
    );
    let subtask = app.call("tasks.get", json!({ "id": parent["id"] }))["subtasks"][0]["id"].clone();
    app.call("subtasks.add", json!({ "taskId": parent["id"], "parentId": subtask, "title": "Nested" }));
    let habit = app.habit("Read", json!({ "kind": "quantity", "quantityTarget": 20.5, "unit": "pages" }));
    app.call("habits.log", json!({ "id": habit["id"], "value": 21.5, "note": "Good, long chapter" }));
    let checklist = app.habit("Routine", json!({ "kind": "checklist", "checklist": [{ "title": "One" }] }));
    app.call("habits.log", json!({ "id": checklist["id"], "date": "2026-11-01", "checkedItems": [checklist["checklist"][0]["id"]] }));
    app.call("focus.start", json!({ "kind": "focus", "minutes": 5, "taskId": parent["id"] }));
    app.advance_minutes(5);
    app.tick();
    app.call("settings.set", json!({ "key": "theme", "value": "dark" }));
}

/// Everything a person can see, so a round trip can be compared as a whole.
fn snapshot(app: &App) -> Value {
    json!({
        "tasks": app.call("tasks.list", json!({})),
        "archived": app.call("tasks.list", json!({ "view": "archived" })),
        "detail": app.call("tasks.get", json!({ "id": 1 })),
        "habits": app.call("habits.list", json!({ "includeArchived": true })),
        "projects": app.call("projects.list", json!({ "includeArchived": true })),
        "focus": app.call("focus.history", json!({})),
        "settings": app.call("settings.get", json!({})),
        "tags": app.call("tasks.tags", json!({})),
    })
}

fn file(dir: &tempfile::TempDir, name: &str) -> PathBuf {
    dir.path().join(name)
}

#[test]
fn a_json_backup_restores_everything_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.json");
    let app = App::new();
    populate(&app);
    let before = snapshot(&app);
    app.call("backup.exportJson", path_param(&path));

    let fresh = App::new();
    fresh.task("Replaced by the restore", json!({}));
    fresh.call("backup.restore", path_param(&path));
    assert_eq!(snapshot(&fresh), before);
    assert!(fresh.list("all").iter().all(|title| title != "Replaced by the restore"));
}

#[test]
fn a_database_backup_restores_everything_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.db");
    let app = App::new();
    populate(&app);
    let before = snapshot(&app);
    app.call("backup.exportDatabase", path_param(&path));

    let fresh = App::new();
    fresh.call("backup.restore", path_param(&path));
    assert_eq!(snapshot(&fresh), before);
}

#[test]
fn exporting_over_an_existing_file_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.db");
    let app = App::new();
    app.call("backup.exportDatabase", path_param(&path));
    app.task("Added later", json!({}));
    app.call("backup.exportDatabase", path_param(&path));
    assert_eq!(app.call("backup.preview", path_param(&path))["incoming"]["tasks"], 1);
}

#[test]
fn exports_work_when_nothing_can_be_created_beside_the_chosen_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let app = App::new();
    populate(&app);
    let names = ["backup.json", "backup.db", "tasks.csv", "habits.csv"];
    for name in names {
        fs::write(file(&dir, name), "").unwrap();
    }
    // This is how a file granted by a desktop portal behaves: writable, in a closed directory.
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();

    let methods =
        ["backup.exportJson", "backup.exportDatabase", "backup.exportTasksCsv", "backup.exportHabitsCsv"];
    for (method, name) in methods.into_iter().zip(names) {
        app.call(method, path_param(&file(&dir, name)));
    }
    let sizes: Vec<u64> = names.map(|name| fs::metadata(file(&dir, name)).unwrap().len()).to_vec();
    let leftovers = fs::read_dir(dir.path()).unwrap().count();
    let fresh = App::new();
    let json = fresh.call("backup.preview", path_param(&file(&dir, "backup.json")));
    let database = fresh.call("backup.preview", path_param(&file(&dir, "backup.db")));
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();

    assert!(sizes.iter().all(|size| *size > 0), "{sizes:?}");
    assert_eq!(leftovers, names.len());
    assert_eq!(json["incoming"]["tasks"], 1);
    assert_eq!(database["incoming"]["tasks"], 1);
}

#[test]
fn a_preview_describes_the_file_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.json");
    let app = App::new();
    populate(&app);
    app.call("backup.exportJson", path_param(&path));

    let target = App::new();
    target.task("Keep me", json!({}));
    let preview = target.call("backup.preview", path_param(&path));
    assert_eq!(preview["kind"], "json");
    assert_eq!(preview["incoming"]["tasks"], 1);
    assert_eq!(preview["incoming"]["habits"], 2);
    assert_eq!(preview["incoming"]["habitEntries"], 2);
    assert_eq!(preview["incoming"]["projects"], 1);
    assert_eq!(preview["incoming"]["focusSessions"], 1);
    assert_eq!(preview["current"]["tasks"], 1);
    assert_eq!(target.list("all"), ["Keep me"]);
}

/// Exports a valid backup, breaks it with `change`, and checks the restore leaves live data alone.
fn assert_rejected(change: impl Fn(&mut Value), expected: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.json");
    let app = App::new();
    populate(&app);
    app.call("backup.exportJson", path_param(&path));
    let mut document: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    change(&mut document);
    fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();

    let target = App::new();
    target.task("Precious", json!({ "dueDate": "2026-11-05" }));
    let before = target.call("tasks.list", json!({}));
    for method in ["backup.preview", "backup.restore"] {
        let message = target.call_err(method, path_param(&path)).to_string();
        assert!(message.contains(expected), "{method}: {message}");
    }
    assert_eq!(target.call("tasks.list", json!({})), before);
}

#[test]
fn a_backup_with_a_broken_reference_is_rejected() {
    assert_rejected(|doc| doc["tables"]["tasks"][0]["project_id"] = json!(999), "inconsistent");
}

#[test]
fn a_backup_with_an_invalid_value_is_rejected() {
    assert_rejected(|doc| doc["tables"]["tasks"][0]["priority"] = json!("extreme"), "invalid row");
}

#[test]
fn a_backup_missing_a_required_value_is_rejected() {
    assert_rejected(
        |doc| {
            doc["tables"]["tasks"][0].as_object_mut().unwrap().remove("title");
        },
        "invalid row",
    );
}

#[test]
fn a_column_name_cannot_inject_sql() {
    assert_rejected(
        |doc| doc["tables"]["tags"][0]["name) VALUES ('x'); DROP TABLE tasks; --"] = json!("x"),
        "unknown column",
    );
}

#[test]
fn an_unknown_table_is_rejected() {
    assert_rejected(|doc| doc["tables"]["sqlite_master"] = json!([]), "unknown table");
}

#[test]
fn a_backup_from_a_newer_version_is_rejected() {
    assert_rejected(|doc| doc["schemaVersion"] = json!(9999), "newer version");
}

#[test]
fn a_file_with_the_wrong_format_marker_is_rejected() {
    assert_rejected(|doc| doc["format"] = json!("someone-elses-backup"), "not a Dayweave backup");
}

#[test]
fn a_non_backup_json_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "notes.json");
    fs::write(&path, r#"{ "hello": "world" }"#).unwrap();
    let app = App::new();
    assert!(app.call_err("backup.preview", path_param(&path)).to_string().contains("not a Dayweave backup"));
}

#[test]
fn garbage_and_empty_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::new();
    for (name, bytes) in [("garbage", &b"\x00\xff not json"[..]), ("empty", b"")] {
        let path = file(&dir, name);
        fs::write(&path, bytes).unwrap();
        assert!(app
            .call_err("backup.preview", path_param(&path))
            .to_string()
            .contains("not a Dayweave backup"));
    }
}

#[test]
fn a_missing_file_gives_a_file_error() {
    let app = App::new();
    assert_eq!(app.call_err("backup.preview", json!({ "path": "/nonexistent/backup.json" })).code(), "io");
}

#[test]
fn another_applications_sqlite_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "other.db");
    {
        let conn = rusqlite_connection(&path);
        conn.execute_batch("CREATE TABLE tasks (id INTEGER); INSERT INTO tasks VALUES (1);").unwrap();
    }
    let app = App::new();
    assert!(app.call_err("backup.preview", path_param(&path)).to_string().contains("not a Dayweave backup"));
}

fn rusqlite_connection(path: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).unwrap()
}

#[test]
fn a_damaged_database_file_is_rejected_and_live_data_survives() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.db");
    let app = App::new();
    populate(&app);
    app.call("backup.exportDatabase", path_param(&path));
    let mut bytes = fs::read(&path).unwrap();
    let length = bytes.len();
    for byte in &mut bytes[4096..length.min(16384)] {
        *byte = 0xAB;
    }
    fs::write(&path, &bytes).unwrap();

    let target = App::new();
    target.task("Precious", json!({}));
    assert!(target.core.rpc("backup.restore", path_param(&path)).is_err());
    assert_eq!(target.list("all"), ["Precious"]);
}

#[test]
fn a_safety_copy_is_written_before_a_restore_and_old_ones_are_pruned() {
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(ManualClock::new(Karachi, "2026-11-02T10:00"));
    let core = Core::open_with_clock(dir.path(), clock.clone()).unwrap();
    core.rpc("tasks.create", json!({ "title": "Before restore" })).unwrap();

    let backup = file(&dir, "backup.json");
    core.rpc("backup.exportJson", path_param(&backup)).unwrap();
    core.rpc("tasks.create", json!({ "title": "After export" })).unwrap();

    let mut safety = PathBuf::new();
    for second in 0..7 {
        clock.set(&format!("2026-11-02T10:0{second}"));
        let result = core.rpc("backup.restore", path_param(&backup)).unwrap();
        safety = PathBuf::from(result["safetyCopy"].as_str().unwrap());
    }
    let kept = fs::read_dir(dir.path().join("backups")).unwrap().count();
    assert_eq!(kept, 5);
    assert!(safety.exists());

    let preview = core.rpc("backup.preview", path_param(&safety)).unwrap();
    assert_eq!(preview["kind"], "database");
}

#[test]
fn a_restored_database_does_not_fire_old_reminders() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.json");
    let app = App::new();
    app.tick();
    app.task("Old", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [{ "kind": "before", "offsetMinutes": 0 }] }));
    app.call("backup.exportJson", path_param(&path));

    app.set("2026-11-03T10:00");
    app.call("backup.restore", path_param(&path));
    assert!(app.tick().is_empty());
}

#[test]
fn restored_data_keeps_working() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "backup.json");
    let app = App::new();
    populate(&app);
    app.call("backup.exportJson", path_param(&path));

    let fresh = App::new();
    fresh.call("backup.restore", path_param(&path));
    let result = fresh.call("tasks.complete", json!({ "id": 1 }));
    assert_eq!(result["next"]["dueDate"], "2026-11-09", "the repeat rule came back too");
    let created = fresh.task("New after restore", json!({}));
    assert!(created["id"].as_i64().unwrap() > 1);
}

#[test]
fn the_task_csv_quotes_awkward_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "tasks.csv");
    let app = App::new();
    populate(&app);
    app.call("backup.exportTasksCsv", path_param(&path));
    let text = fs::read_to_string(&path).unwrap();
    let mut lines = text.split('\n');
    assert_eq!(lines.next().unwrap(), "Title,Description,Status,Priority,Category,Project,Tags,Start date,Due date,Due time,Created,Completed");
    assert!(text.contains("\"Write \"\"chapter, one\"\"\",\"Line one\nLine two\",todo,medium,,Thesis,writing,,2026-11-05,09:00,"));
}

#[test]
fn the_habit_csv_lists_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = file(&dir, "habits.csv");
    let app = App::new();
    populate(&app);
    app.call("backup.exportHabitsCsv", path_param(&path));
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("Habit,Date,Status,Value,Unit,Note\n"));
    assert!(text.contains("Read,2026-11-02,done,21.5,pages,\"Good, long chapter\""));
    assert!(text.contains("Routine,2026-11-01,done,,,"));
}

#[test]
fn data_on_disk_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(ManualClock::new(Karachi, "2026-11-02T10:00"));
    {
        let core = Core::open_with_clock(dir.path(), clock.clone()).unwrap();
        core.rpc("tasks.create", json!({ "title": "Persisted", "subtasks": ["Child"] })).unwrap();
        core.rpc("habits.create", json!({ "name": "Kept" })).unwrap();
    }
    let core = Core::open_with_clock(dir.path(), clock).unwrap();
    assert_eq!(core.rpc("tasks.get", json!({ "id": 1 })).unwrap()["subtaskTotal"], 1);
    assert_eq!(core.rpc("habits.list", json!({})).unwrap()[0]["name"], "Kept");
}
