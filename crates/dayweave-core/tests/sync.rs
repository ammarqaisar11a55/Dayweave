//! Two devices syncing through an in-memory server that follows the real server's rules: the newer
//! change to a record wins, every accepted change gets the next number in the account's sequence, and
//! a device asks for everything after the last number it has seen.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::DateTime;
use chrono_tz::Asia::Karachi;
use common::{titles, App};
use dayweave_core::clock::ManualClock;
use dayweave_core::transport::{Transport, TransportError};
use dayweave_core::Core;
use serde_json::{json, Value};

#[derive(Clone)]
struct Stored {
    entity: String,
    id: String,
    updated_at: String,
    deleted: bool,
    data: Value,
    seq: i64,
    device: String,
}

#[derive(Default)]
struct Server {
    records: Mutex<Vec<Stored>>,
    offline: AtomicBool,
    revoked: AtomicBool,
}

fn newer(left: &str, right: &str) -> bool {
    DateTime::parse_from_rfc3339(left).unwrap() > DateTime::parse_from_rfc3339(right).unwrap()
}

impl Transport for Server {
    fn post(&self, url: &str, token: Option<&str>, body: &Value) -> Result<Value, TransportError> {
        if self.offline.load(Ordering::SeqCst) {
            return Err(TransportError::Unreachable("no network".into()));
        }
        let path = url.trim_start_matches("https://sync.test");
        match path {
            "/auth/signup" | "/auth/login" => {
                // The token names the device, so the server can leave out a device's own changes.
                let device = body["device"].as_str().unwrap_or("device").to_string();
                return Ok(json!({ "token": device, "email": body["email"] }));
            }
            "/auth/logout" => return Ok(json!({})),
            "/sync" => {}
            other => panic!("unexpected request to {other}"),
        }
        if self.revoked.load(Ordering::SeqCst) {
            return Err(TransportError::Rejected {
                status: 401,
                code: "unauthorized".into(),
                message: "Sign in".into(),
            });
        }
        let device = token.unwrap().to_string();
        let mut records = self.records.lock().unwrap();
        let mut next = records.iter().map(|record| record.seq).max().unwrap_or(0);
        let mut rejected: Vec<(String, String)> = Vec::new();
        for change in body["changes"].as_array().unwrap() {
            let entity = change["entity"].as_str().unwrap().to_string();
            let id = change["id"].as_str().unwrap().to_string();
            let updated_at = change["updatedAt"].as_str().unwrap().to_string();
            let existing = records.iter().position(|record| record.entity == entity && record.id == id);
            if let Some(index) = existing {
                if newer(&records[index].updated_at, &updated_at) {
                    rejected.push((entity, id));
                    continue;
                }
                records.remove(index);
            }
            next += 1;
            records.push(Stored {
                entity,
                id,
                updated_at,
                deleted: change["deleted"].as_bool().unwrap_or(false),
                data: change["data"].clone(),
                seq: next,
                device: device.clone(),
            });
        }
        let cursor = body["cursor"].as_i64().unwrap();
        let limit = body["limit"].as_u64().unwrap() as usize;
        let mut newer_records: Vec<&Stored> =
            records.iter().filter(|record| record.seq > cursor && record.device != device).collect();
        newer_records.sort_by_key(|record| record.seq);
        let more = newer_records.len() > limit;
        newer_records.truncate(limit);
        let last = if more {
            newer_records.last().map(|record| record.seq).unwrap_or(cursor)
        } else {
            next.max(cursor)
        };
        let mut changes: Vec<Value> = newer_records
            .iter()
            .map(|record| {
                json!({ "entity": record.entity, "id": record.id, "updatedAt": record.updated_at,
                        "deleted": record.deleted, "data": record.data })
            })
            .collect();
        for (entity, id) in rejected {
            let record = records.iter().find(|record| record.entity == entity && record.id == id).unwrap();
            changes.push(json!({ "entity": record.entity, "id": record.id, "updatedAt": record.updated_at,
                                 "deleted": record.deleted, "data": record.data }));
        }
        Ok(json!({ "cursor": last, "changes": changes, "more": more }))
    }
}

fn device(server: &Arc<Server>) -> App {
    let clock = Arc::new(ManualClock::new(Karachi, "2026-11-02T10:00"));
    let core = Core::in_memory(clock.clone()).unwrap().with_transport(server.clone());
    App { core, clock }
}

fn signed_in(server: &Arc<Server>, name: &str) -> App {
    let app = device(server);
    app.call(
        "sync.signUp",
        json!({ "server": "https://sync.test", "email": "Ammar@Example.com", "password": "correct horse", "device": name }),
    );
    app
}

fn sync(app: &App) -> Value {
    app.call("sync.run", json!({}))
}

/// SQLite records change times to the millisecond, so tests that order edits wait between them.
fn later() {
    std::thread::sleep(std::time::Duration::from_millis(15));
}

#[test]
fn a_task_with_everything_reaches_another_device() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let project = phone.call("projects.create", json!({ "name": "Thesis", "color": "blue" }));
    phone.task(
        "Write chapter 3",
        json!({
            "projectId": project["id"], "priority": "high", "dueDate": "2026-11-05", "dueTime": "17:00",
            "tags": ["writing", "university"], "subtasks": ["Outline", "Draft"],
            "reminders": [{ "kind": "before", "offsetMinutes": 30 }],
        }),
    );
    sync(&phone);

    let desktop = signed_in(&server, "desktop");
    let tasks = desktop.call("tasks.list", json!({ "view": "all" }));
    let task = &tasks[0];
    assert_eq!(task["title"], "Write chapter 3");
    assert_eq!(task["priority"], "high");
    assert_eq!(task["dueTime"], "17:00");
    let details = desktop.call("tasks.get", json!({ "id": task["id"] }));
    assert_eq!(details["projectName"], "Thesis", "the project link survives: {details}");
    let tags: Vec<&str> =
        details["tags"].as_array().unwrap().iter().map(|tag| tag["name"].as_str().unwrap()).collect();
    assert_eq!(tags, ["university", "writing"]);
    let subtasks: Vec<&str> = details["subtasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|subtask| subtask["title"].as_str().unwrap())
        .collect();
    assert_eq!(subtasks, ["Outline", "Draft"]);
    assert_eq!(details["reminders"][0]["offsetMinutes"], 30);

    // Writing what arrived didn't queue it to be sent back.
    assert_eq!(desktop.call("sync.status", json!({}))["pending"], 0);
    assert_eq!(phone.call("sync.status", json!({}))["pending"], 0);
}

#[test]
fn edits_travel_both_ways_and_the_newer_one_wins() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let task = phone.task("Draft", json!({}));
    sync(&phone);
    let desktop = signed_in(&server, "desktop");
    let desktop_id = desktop.call("tasks.list", json!({ "view": "all" }))[0]["id"].clone();

    // Both edit while offline. The desktop's edit is the later one.
    phone.call("tasks.update", json!({ "id": task["id"], "title": "Phone title" }));
    later();
    desktop.call("tasks.update", json!({ "id": desktop_id, "title": "Desktop title" }));

    sync(&phone);
    sync(&desktop);
    sync(&phone);
    assert_eq!(titles(&phone.call("tasks.list", json!({ "view": "all" }))), ["Desktop title"]);
    assert_eq!(titles(&desktop.call("tasks.list", json!({ "view": "all" }))), ["Desktop title"]);
}

#[test]
fn a_later_edit_made_before_syncing_is_kept_over_an_older_one_from_the_server() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let task = phone.task("Draft", json!({}));
    sync(&phone);
    let desktop = signed_in(&server, "desktop");
    let desktop_id = desktop.call("tasks.list", json!({ "view": "all" }))[0]["id"].clone();

    desktop.call("tasks.update", json!({ "id": desktop_id, "title": "Older" }));
    sync(&desktop);
    later();
    phone.call("tasks.update", json!({ "id": task["id"], "title": "Newer" }));
    // The phone sends its newer edit before it fetches, so the server already holds it.
    sync(&phone);
    assert_eq!(titles(&phone.call("tasks.list", json!({ "view": "all" }))), ["Newer"]);
    sync(&desktop);
    assert_eq!(titles(&desktop.call("tasks.list", json!({ "view": "all" }))), ["Newer"]);
}

#[test]
fn deletions_travel() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let task = phone.task("Temporary", json!({}));
    phone.task("Keep", json!({}));
    sync(&phone);
    let desktop = signed_in(&server, "desktop");
    assert_eq!(desktop.list("all").len(), 2);

    phone.call("tasks.delete", json!({ "id": task["id"] }));
    sync(&phone);
    sync(&desktop);
    assert_eq!(desktop.list("all"), ["Keep"]);
    assert_eq!(desktop.call("sync.status", json!({}))["pending"], 0);
}

#[test]
fn habits_and_their_check_ins_travel_with_checklist_items_intact() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let habit = phone.habit(
        "Morning routine",
        json!({ "kind": "checklist", "checklist": [{ "title": "Stretch" }, { "title": "Plan" }],
                "frequency": "weekdays", "weekdays": [0, 2, 4] }),
    );
    let first = habit["checklist"][0]["id"].clone();
    phone.call("habits.log", json!({ "id": habit["id"], "date": "2026-11-02", "checkedItems": [first] }));
    sync(&phone);

    let desktop = signed_in(&server, "desktop");
    let habits = desktop.call("habits.list", json!({}));
    let synced = &habits[0];
    assert_eq!(synced["name"], "Morning routine");
    assert_eq!(synced["weekdays"], json!([0, 2, 4]));
    let stretch = synced["checklist"][0]["id"].clone();
    assert_eq!(synced["checklist"][0]["title"], "Stretch");
    assert_eq!(
        synced["today"]["checkedItems"],
        json!([stretch]),
        "the checked item is the same item: {synced}"
    );
}

#[test]
fn the_same_habit_logged_on_two_devices_for_the_same_day_becomes_one_check_in() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let habit = phone.habit("Read", json!({ "kind": "quantity", "quantityTarget": 20, "unit": "pages" }));
    sync(&phone);
    let desktop = signed_in(&server, "desktop");
    let desktop_habit = desktop.call("habits.list", json!({}))[0]["id"].clone();

    phone.call("habits.log", json!({ "id": habit["id"], "date": "2026-11-02", "value": 5 }));
    later();
    desktop.call("habits.log", json!({ "id": desktop_habit, "date": "2026-11-02", "value": 12 }));
    sync(&phone);
    sync(&desktop);
    sync(&phone);

    for app in [&phone, &desktop] {
        let habit = &app.call("habits.list", json!({}))[0];
        assert_eq!(habit["today"]["value"], 12.0, "{habit}");
    }
}

#[test]
fn working_offline_keeps_every_change_until_the_server_is_reachable() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    server.offline.store(true, Ordering::SeqCst);

    phone.task("Written offline", json!({}));
    let error = phone.call_err("sync.run", json!({}));
    assert_eq!(error.code(), "offline");
    assert_eq!(phone.list("all"), ["Written offline"]);
    let status = phone.call("sync.status", json!({}));
    assert_eq!(status["pending"], 1);
    assert!(status["lastError"].as_str().unwrap().contains("online"));
    assert_eq!(status["signedIn"], true);

    server.offline.store(false, Ordering::SeqCst);
    sync(&phone);
    let status = phone.call("sync.status", json!({}));
    assert_eq!(status["pending"], 0);
    assert!(status["lastError"].is_null());
    assert!(status["lastSyncAt"].is_string());
    let desktop = signed_in(&server, "desktop");
    assert_eq!(desktop.list("all"), ["Written offline"]);
}

#[test]
fn local_only_use_never_needs_the_network() {
    let server = Arc::new(Server::default());
    server.offline.store(true, Ordering::SeqCst);
    let app = device(&server);
    app.task("Local", json!({}));
    let status = app.call("sync.status", json!({}));
    assert_eq!(status["signedIn"], false);
    assert_eq!(app.call_err("sync.run", json!({})).code(), "invalid");
}

#[test]
fn shared_preferences_travel_but_device_ones_stay() {
    let server = Arc::new(Server::default());
    let desktop = signed_in(&server, "desktop");
    desktop.call("settings.set", json!({ "key": "theme", "value": "dark" }));
    desktop.call("settings.set", json!({ "key": "startAtLogin", "value": true }));
    sync(&desktop);
    let phone = signed_in(&server, "phone");
    let settings = phone.call("settings.get", json!({}));
    assert_eq!(settings["theme"], "dark");
    assert_eq!(settings["startAtLogin"], false);
}

#[test]
fn completing_a_repeating_task_doesnt_create_the_next_one_twice() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    let task = phone
        .task("Water plants", json!({ "dueDate": "2026-11-02", "recurrence": { "frequency": "daily" } }));
    sync(&phone);
    let desktop = signed_in(&server, "desktop");
    phone.call("tasks.complete", json!({ "id": task["id"] }));
    sync(&phone);
    sync(&desktop);
    let all = desktop.call("tasks.list", json!({ "view": "all" }));
    assert_eq!(all.as_array().unwrap().len(), 2, "{all}");
    sync(&phone);
    assert_eq!(phone.list("all").len(), 2);
}

#[test]
fn only_finished_focus_sessions_travel() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    phone.call("focus.start", json!({ "kind": "focus", "minutes": 25 }));
    sync(&phone);
    let desktop = signed_in(&server, "desktop");
    assert!(desktop.call("focus.current", json!({})).is_null());

    phone.call("focus.complete", json!({}));
    sync(&phone);
    sync(&desktop);
    let history = desktop.call("focus.history", json!({}));
    assert_eq!(history.as_array().unwrap().len(), 1, "{history}");
}

#[test]
fn backups_never_contain_the_sign_in_and_restoring_keeps_this_device_signed_in() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    phone.task("Before", json!({}));
    let dir = tempfile::tempdir().unwrap();
    let json_file = dir.path().join("backup.json");
    let db_file = dir.path().join("backup.db");
    phone.call("backup.exportJson", json!({ "path": json_file }));
    phone.call("backup.exportDatabase", json!({ "path": db_file }));
    let text = std::fs::read_to_string(&json_file).unwrap();
    assert!(!text.contains("internal.sync"), "the JSON backup holds no sync sign-in");
    let bytes = std::fs::read(&db_file).unwrap();
    assert!(
        !bytes.windows(13).any(|window| window == b"internal.sync"),
        "the database copy holds no sync sign-in"
    );

    phone.call("backup.restore", json!({ "path": json_file }));
    assert_eq!(phone.call("sync.status", json!({}))["signedIn"], true);
    sync(&phone);
}

#[test]
fn signing_out_keeps_the_data_and_a_revoked_device_stops_trying() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    phone.task("Mine", json!({}));
    sync(&phone);
    phone.call("sync.signOut", json!({}));
    assert_eq!(phone.call("sync.status", json!({}))["signedIn"], false);
    assert_eq!(phone.list("all"), ["Mine"]);

    let desktop = signed_in(&server, "desktop");
    server.revoked.store(true, Ordering::SeqCst);
    let error = desktop.call_err("sync.run", json!({}));
    assert!(error.to_string().contains("Sign in again"), "{error}");
    assert_eq!(desktop.call("sync.status", json!({}))["signedIn"], false);
}

#[test]
fn the_server_address_must_be_secure_except_on_this_machine() {
    let server = Arc::new(Server::default());
    let app = device(&server);
    let attempt = |address: &str| {
        app.core.rpc(
            "sync.signIn",
            json!({ "server": address, "email": "a@b.c", "password": "long enough", "device": "test" }),
        )
    };
    assert!(attempt("http://dayweave.example.com").unwrap_err().to_string().contains("https://"));
    assert!(attempt("ftp://example.com").is_err());
}

#[test]
fn many_records_sync_in_rounds() {
    let server = Arc::new(Server::default());
    let phone = signed_in(&server, "phone");
    for index in 0..450 {
        phone.task(&format!("Task {index}"), json!({}));
    }
    let report = sync(&phone);
    assert_eq!(report["sent"], 450);
    let desktop = signed_in(&server, "desktop");
    assert_eq!(desktop.list("all").len(), 450);
}
