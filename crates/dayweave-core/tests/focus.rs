mod common;

use std::sync::Arc;

use chrono_tz::Asia::Karachi;
use common::App;
use dayweave_core::clock::ManualClock;
use dayweave_core::Core;
use serde_json::{json, Value};

fn start(app: &App, minutes: i64) -> Value {
    app.call("focus.start", json!({ "kind": "focus", "minutes": minutes }))
}

#[test]
fn a_session_counts_down_and_completes_on_time_without_the_interface() {
    let app = App::new();
    app.tick();
    let session = start(&app, 25);
    assert_eq!(session["status"], "running");
    assert_eq!(session["remainingSeconds"], 1500);

    app.advance_minutes(10);
    assert_eq!(app.call("focus.current", json!({}))["remainingSeconds"], 900);

    app.advance_minutes(14);
    assert!(app.tick().is_empty());
    app.advance_minutes(1);
    assert_eq!(app.tick(), ["Focus session complete"]);
    assert!(app.call("focus.current", json!({})).is_null());

    let history = app.call("focus.history", json!({}));
    assert_eq!(history[0]["status"], "completed");
    assert_eq!(history[0]["elapsedSeconds"], 1500);
}

#[test]
fn paused_time_does_not_count() {
    let app = App::new();
    start(&app, 25);
    app.advance_minutes(10);
    let paused = app.call("focus.pause", json!({}));
    assert_eq!(paused["status"], "paused");
    assert_eq!(paused["elapsedSeconds"], 600);

    app.advance_minutes(60);
    assert_eq!(app.call("focus.current", json!({}))["elapsedSeconds"], 600);

    let resumed = app.call("focus.resume", json!({}));
    assert_eq!(resumed["status"], "running");
    assert_eq!(resumed["remainingSeconds"], 900);
    app.advance_minutes(5);
    assert_eq!(app.call("focus.current", json!({}))["elapsedSeconds"], 900);
}

#[test]
fn a_paused_session_never_finishes_by_itself() {
    let app = App::new();
    app.tick();
    start(&app, 5);
    app.call("focus.pause", json!({}));
    app.advance_minutes(120);
    assert!(app.tick().is_empty());
    assert_eq!(app.call("focus.current", json!({}))["status"], "paused");
}

#[test]
fn pausing_twice_does_not_lose_time() {
    let app = App::new();
    start(&app, 25);
    app.advance_minutes(3);
    app.call("focus.pause", json!({}));
    app.advance_minutes(3);
    assert_eq!(app.call("focus.pause", json!({}))["elapsedSeconds"], 180);
    assert_eq!(app.call("focus.resume", json!({}))["elapsedSeconds"], 180);
    assert_eq!(app.call("focus.resume", json!({}))["elapsedSeconds"], 180);
}

#[test]
fn only_one_session_runs_at_a_time() {
    let app = App::new();
    start(&app, 25);
    let error = app.call_err("focus.start", json!({ "kind": "focus", "minutes": 10 }));
    assert!(error.to_string().contains("current session"));
}

#[test]
fn stopping_early_keeps_the_time_spent_but_is_not_a_completed_session() {
    let app = App::new();
    let task = app.task("Write report", json!({}));
    app.call("focus.start", json!({ "kind": "focus", "minutes": 25, "taskId": task["id"] }));
    app.advance_minutes(12);
    let stopped = app.call("focus.stop", json!({}));
    assert_eq!(stopped["status"], "stopped");
    assert_eq!(stopped["taskTitle"], "Write report");

    let stats = app.call("focus.stats", json!({}));
    assert_eq!(stats["todaySeconds"], 720);
    assert_eq!(stats["todaySessions"], 0);
}

#[test]
fn finishing_early_counts_as_completed() {
    let app = App::new();
    start(&app, 25);
    app.advance_minutes(20);
    let session = app.call("focus.complete", json!({}));
    assert_eq!(session["status"], "completed");
    assert_eq!(session["elapsedSeconds"], 1200);
    assert_eq!(app.call("focus.stats", json!({}))["todaySessions"], 1);
}

#[test]
fn a_session_can_be_linked_to_a_task_or_a_habit_and_survives_deleting_them() {
    let app = App::new();
    let habit = app.habit("Study DSA", json!({}));
    app.call("focus.start", json!({ "kind": "focus", "minutes": 25, "habitId": habit["id"] }));
    app.advance_minutes(25);
    app.tick();
    assert_eq!(app.call("focus.history", json!({}))[0]["habitName"], "Study DSA");
    app.call("habits.delete", json!({ "id": habit["id"] }));
    let history = app.call("focus.history", json!({}));
    assert!(history[0]["habitId"].is_null());
    assert_eq!(history[0]["elapsedSeconds"], 1500);
}

#[test]
fn breaks_are_kept_out_of_focus_totals() {
    let app = App::new();
    app.call("focus.start", json!({ "kind": "short_break", "minutes": 5 }));
    app.advance_minutes(5);
    assert_eq!(app.tick(), ["Break is over"]);
    let stats = app.call("focus.stats", json!({}));
    assert_eq!(stats["todaySeconds"], 0);
    assert_eq!(stats["totalSessions"], 0);
}

#[test]
fn long_breaks_reset_the_count_towards_the_next_one() {
    let app = App::new();
    let finish = |kind: &str, minutes: i64| {
        app.call("focus.start", json!({ "kind": kind, "minutes": minutes }));
        app.advance_minutes(minutes);
        app.tick();
    };
    finish("focus", 25);
    finish("focus", 25);
    assert_eq!(app.call("focus.stats", json!({}))["sessionsSinceLongBreak"], 2);
    finish("long_break", 15);
    assert_eq!(app.call("focus.stats", json!({}))["sessionsSinceLongBreak"], 0);
}

#[test]
fn statistics_cover_today_the_week_and_all_time() {
    let app = App::new();
    let finish = |app: &App, minutes: i64| {
        app.call("focus.start", json!({ "kind": "focus", "minutes": minutes }));
        app.advance_minutes(minutes);
        app.tick();
    };
    app.set("2026-10-01T10:00");
    finish(&app, 50);
    app.set("2026-10-30T10:00");
    finish(&app, 25);
    app.set("2026-11-02T10:00");
    finish(&app, 25);
    let stats = app.call("focus.stats", json!({}));
    assert_eq!(stats["todaySeconds"], 1500);
    assert_eq!(stats["weekSeconds"], 3000);
    assert_eq!(stats["totalSeconds"], 6000);
    assert_eq!(stats["totalSessions"], 3);
}

#[test]
fn a_session_interrupted_by_a_restart_finishes_when_its_time_ran_out() {
    let dir = tempfile::tempdir().unwrap();
    let clock = Arc::new(ManualClock::new(Karachi, "2026-11-02T10:00"));
    {
        let core = Core::open_with_clock(dir.path(), clock.clone()).unwrap();
        core.tick().unwrap();
        core.rpc("focus.start", json!({ "kind": "focus", "minutes": 25 })).unwrap();
    }
    clock.set("2026-11-02T15:00");
    let core = Core::open_with_clock(dir.path(), clock).unwrap();
    assert_eq!(core.tick().unwrap().len(), 1);
    let history = core.rpc("focus.history", json!({})).unwrap();
    assert_eq!(history[0]["status"], "completed");
    assert_eq!(history[0]["elapsedSeconds"], 1500);
    assert_eq!(history[0]["endedAt"], "2026-11-02T05:25:00Z", "it ended when the timer did, not at launch");
}

#[test]
fn invalid_sessions_are_rejected() {
    let app = App::new();
    assert!(app
        .call_err("focus.start", json!({ "kind": "nap", "minutes": 5 }))
        .to_string()
        .contains("Unknown"));
    assert!(app
        .call_err("focus.start", json!({ "kind": "focus", "minutes": 0 }))
        .to_string()
        .contains("minutes"));
    assert!(app.call_err("focus.pause", json!({})).to_string().contains("no focus session"));
}
