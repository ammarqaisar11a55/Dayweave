mod common;

use common::App;
use serde_json::{json, Value};

fn daily(extra: Value) -> Value {
    let mut rule = json!({ "frequency": "daily" });
    common::merge(&mut rule, extra);
    rule
}

#[test]
fn completing_a_recurring_task_creates_the_next_occurrence() {
    let app = App::new();
    let task = app.task(
        "Water plants",
        json!({
            "dueDate": "2026-11-02",
            "dueTime": "09:00",
            "priority": "high",
            "tags": ["home"],
            "recurrence": { "frequency": "weekly" },
            "reminders": [{ "kind": "before", "offsetMinutes": 10 }],
            "subtasks": ["Front room"],
        }),
    );
    let subtask = app.call("tasks.get", json!({ "id": task["id"] }))["subtasks"][0]["id"].clone();
    app.call("subtasks.toggle", json!({ "id": subtask }));

    let result = app.call("tasks.complete", json!({ "id": task["id"] }));
    let next = &result["next"];
    assert_eq!(next["dueDate"], "2026-11-09");
    assert_eq!(next["dueTime"], "09:00");
    assert_eq!(next["status"], "todo");
    assert_eq!(next["priority"], "high");
    assert_eq!(next["tags"][0]["name"], "home");
    assert_eq!(next["reminderCount"], 1);
    assert_eq!(next["subtaskTotal"], 1);
    assert_eq!(next["subtaskDone"], 0, "the next occurrence starts unfinished");
    assert_eq!(next["recurrenceId"], task["recurrenceId"]);
}

#[test]
fn completed_occurrences_stay_as_history() {
    let app = App::new();
    let mut id =
        app.task("Daily", json!({ "dueDate": "2026-11-02", "recurrence": daily(json!({})) }))["id"].clone();
    for day in 2..=4 {
        app.set(&format!("2026-11-0{day}T10:00"));
        id = app.call("tasks.complete", json!({ "id": id }))["next"]["id"].clone();
    }
    let completed = app.call("tasks.list", json!({ "view": "completed", "sort": "deadline" }));
    let dates: Vec<_> = completed.as_array().unwrap().iter().map(|task| &task["completedOn"]).collect();
    assert_eq!(dates, ["2026-11-02", "2026-11-03", "2026-11-04"]);
    assert_eq!(app.list("all").len(), 4);
}

#[test]
fn reopening_and_completing_again_does_not_create_a_second_occurrence() {
    let app = App::new();
    let id =
        app.task("Once", json!({ "dueDate": "2026-11-02", "recurrence": daily(json!({})) }))["id"].clone();
    assert!(app.call("tasks.complete", json!({ "id": id }))["next"].is_object());
    app.call("tasks.reopen", json!({ "id": id }));
    assert!(app.call("tasks.complete", json!({ "id": id }))["next"].is_null());
    assert_eq!(app.list("all").len(), 2);
}

#[test]
fn completing_an_already_completed_task_changes_nothing() {
    let app = App::new();
    let id =
        app.task("Once", json!({ "dueDate": "2026-11-02", "recurrence": daily(json!({})) }))["id"].clone();
    app.call("tasks.complete", json!({ "id": id }));
    assert!(app.call("tasks.complete", json!({ "id": id }))["next"].is_null());
    assert_eq!(app.list("all").len(), 2);
}

#[test]
fn a_task_finished_late_skips_to_the_first_date_after_today() {
    let app = App::new();
    let id =
        app.task("Backlog", json!({ "dueDate": "2026-10-20", "recurrence": daily(json!({})) }))["id"].clone();
    let result = app.call("tasks.complete", json!({ "id": id }));
    assert_eq!(result["next"]["dueDate"], "2026-11-03");
    assert_eq!(app.list("all").len(), 2, "no backlog of missed occurrences is generated");
}

#[test]
fn a_monthly_task_on_the_31st_returns_to_it_after_a_short_month() {
    let app = App::new();
    app.set("2027-01-31T10:00");
    let mut id = app
        .task("Rent", json!({ "dueDate": "2027-01-31", "recurrence": { "frequency": "monthly" } }))["id"]
        .clone();
    let mut dates = Vec::new();
    for wall in ["2027-01-31T10:00", "2027-02-28T10:00", "2027-03-31T10:00"] {
        app.set(wall);
        let next = app.call("tasks.complete", json!({ "id": id }))["next"].clone();
        dates.push(next["dueDate"].as_str().unwrap().to_string());
        id = next["id"].clone();
    }
    assert_eq!(dates, ["2027-02-28", "2027-03-31", "2027-04-30"]);
}

#[test]
fn a_custom_rule_uses_its_interval_and_weekdays() {
    let app = App::new();
    let id = app.task(
        "Standup notes",
        json!({ "dueDate": "2026-11-02", "recurrence": { "frequency": "weekly", "interval": 2, "weekdays": [0, 4] } }),
    )["id"].clone();
    let next = app.call("tasks.complete", json!({ "id": id }))["next"].clone();
    assert_eq!(next["dueDate"], "2026-11-06");
    app.set("2026-11-06T10:00");
    let next = app.call("tasks.complete", json!({ "id": next["id"] }))["next"].clone();
    assert_eq!(next["dueDate"], "2026-11-16");
}

#[test]
fn a_series_ends_at_its_end_date_and_keeps_the_last_completion() {
    let app = App::new();
    let rule = daily(json!({ "endDate": "2026-11-03" }));
    let id = app.task("Short", json!({ "dueDate": "2026-11-02", "recurrence": rule }))["id"].clone();
    let next = app.call("tasks.complete", json!({ "id": id }))["next"].clone();
    assert_eq!(next["dueDate"], "2026-11-03");
    app.set("2026-11-03T10:00");
    assert!(app.call("tasks.complete", json!({ "id": next["id"] }))["next"].is_null());
    assert!(app.list("today").is_empty());
}

#[test]
fn a_series_ends_at_its_occurrence_limit() {
    let app = App::new();
    let rule = daily(json!({ "maxCount": 2 }));
    let id = app.task("Twice", json!({ "dueDate": "2026-11-02", "recurrence": rule }))["id"].clone();
    let next = app.call("tasks.complete", json!({ "id": id }))["next"].clone();
    assert!(next.is_object());
    app.set("2026-11-03T10:00");
    assert!(app.call("tasks.complete", json!({ "id": next["id"] }))["next"].is_null());
}

#[test]
fn a_start_date_keeps_its_distance_from_the_deadline() {
    let app = App::new();
    let id = app.task(
        "Report",
        json!({ "startDate": "2026-11-01", "dueDate": "2026-11-04", "recurrence": { "frequency": "weekly" } }),
    )["id"].clone();
    let next = app.call("tasks.complete", json!({ "id": id }))["next"].clone();
    assert_eq!(next["dueDate"], "2026-11-11");
    assert_eq!(next["startDate"], "2026-11-08");
}

#[test]
fn duplicating_a_recurring_task_makes_a_one_off() {
    let app = App::new();
    let id =
        app.task("Series", json!({ "dueDate": "2026-11-02", "recurrence": daily(json!({})) }))["id"].clone();
    let copy = app.call("tasks.duplicate", json!({ "id": id }));
    assert!(copy["recurrenceId"].is_null());
    assert!(app.call("tasks.complete", json!({ "id": copy["id"] }))["next"].is_null());
}

#[test]
fn removing_the_repeat_stops_the_series() {
    let app = App::new();
    let id =
        app.task("Series", json!({ "dueDate": "2026-11-02", "recurrence": daily(json!({})) }))["id"].clone();
    app.call("tasks.update", json!({ "id": id, "title": "Series", "dueDate": "2026-11-02" }));
    assert!(app.call("tasks.complete", json!({ "id": id }))["next"].is_null());
}

#[test]
fn editing_a_rule_applies_to_the_next_occurrence() {
    let app = App::new();
    let id =
        app.task("Series", json!({ "dueDate": "2026-11-02", "recurrence": daily(json!({})) }))["id"].clone();
    app.call(
        "tasks.update",
        json!({ "id": id, "title": "Series", "dueDate": "2026-11-02", "recurrence": { "frequency": "weekly" } }),
    );
    let next = app.call("tasks.complete", json!({ "id": id }))["next"].clone();
    assert_eq!(next["dueDate"], "2026-11-09");
    assert_eq!(app.call("tasks.get", json!({ "id": next["id"] }))["recurrence"]["frequency"], "weekly");
}
