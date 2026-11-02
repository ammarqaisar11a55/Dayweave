mod common;

use chrono_tz::America::New_York;
use common::App;
use serde_json::{json, Value};

fn before(minutes: i64) -> Value {
    json!({ "kind": "before", "offsetMinutes": minutes })
}

/// Runs the scheduler once so later changes are measured from now.
fn start(app: &App) {
    assert!(app.tick().is_empty());
}

#[test]
fn a_reminder_fires_once_at_its_offset() {
    let app = App::new();
    start(&app);
    app.task("Submit", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(30)] }));

    app.set("2026-11-02T11:29");
    assert!(app.tick().is_empty());
    app.set("2026-11-02T11:30");
    assert_eq!(app.tick(), ["Submit"]);
    app.set("2026-11-02T11:31");
    assert!(app.tick().is_empty(), "a reminder never repeats");
}

#[test]
fn a_task_can_have_several_reminders() {
    let app = App::new();
    start(&app);
    app.task("Exam", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(60), before(10), before(0)] }));
    let mut fired = Vec::new();
    for wall in ["2026-11-02T11:00", "2026-11-02T11:50", "2026-11-02T12:00", "2026-11-02T12:30"] {
        app.set(wall);
        fired.push(app.tick().len());
    }
    assert_eq!(fired, [1, 1, 1, 0]);
}

#[test]
fn identical_reminders_are_stored_once() {
    let app = App::new();
    let task = app.task("Exam", json!({ "dueDate": "2026-11-02", "reminders": [before(30), before(30)] }));
    assert_eq!(task["reminderCount"], 1);
}

#[test]
fn a_date_only_deadline_uses_the_configured_time_of_day() {
    let app = App::new();
    start(&app);
    app.call("settings.set", json!({ "key": "dateOnlyReminderTime", "value": "15:00" }));
    app.task("Pay bill", json!({ "dueDate": "2026-11-02", "reminders": [before(0)] }));
    app.set("2026-11-02T14:59");
    assert!(app.tick().is_empty());
    app.set("2026-11-02T15:00");
    assert_eq!(app.tick(), ["Pay bill"]);
}

#[test]
fn completing_a_task_cancels_its_reminders() {
    let app = App::new();
    start(&app);
    let task = app.task(
        "Finished early",
        json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }),
    );
    app.call("tasks.complete", json!({ "id": task["id"] }));
    app.set("2026-11-02T12:00");
    assert!(app.tick().is_empty());
}

#[test]
fn archiving_a_task_cancels_its_reminders() {
    let app = App::new();
    start(&app);
    let task =
        app.task("Shelved", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    app.call("tasks.archive", json!({ "id": task["id"] }));
    app.set("2026-11-02T12:00");
    assert!(app.tick().is_empty());
}

#[test]
fn rescheduling_fires_the_reminder_again_for_the_new_deadline() {
    let app = App::new();
    start(&app);
    let id = app
        .task("Moved", json!({ "dueDate": "2026-11-02", "dueTime": "11:00", "reminders": [before(0)] }))
        ["id"]
        .clone();
    app.set("2026-11-02T11:00");
    assert_eq!(app.tick(), ["Moved"]);

    app.call("tasks.reschedule", json!({ "id": id, "dueDate": "2026-11-02", "dueTime": "13:00" }));
    app.set("2026-11-02T13:00");
    assert_eq!(app.tick(), ["Moved"]);
}

#[test]
fn moving_a_deadline_earlier_does_not_fire_for_the_old_time() {
    let app = App::new();
    start(&app);
    let id = app
        .task("Moved", json!({ "dueDate": "2026-11-02", "dueTime": "18:00", "reminders": [before(0)] }))
        ["id"]
        .clone();
    app.call("tasks.reschedule", json!({ "id": id, "dueDate": "2026-11-02", "dueTime": "12:00" }));
    app.set("2026-11-02T12:00");
    assert_eq!(app.tick().len(), 1);
    app.set("2026-11-02T18:00");
    assert!(app.tick().is_empty());
}

#[test]
fn adding_a_reminder_whose_time_has_passed_does_not_fire_it() {
    let app = App::new();
    start(&app);
    app.task("Soon", json!({ "dueDate": "2026-11-02", "dueTime": "10:30", "reminders": [before(120)] }));
    app.advance_minutes(1);
    assert!(app.tick().is_empty());
}

#[test]
fn a_reminder_missed_while_closed_arrives_on_the_next_launch() {
    let app = App::new();
    start(&app);
    app.task("Missed", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    app.set("2026-11-02T20:00");
    assert_eq!(app.tick(), ["Missed"]);
}

#[test]
fn several_missed_reminders_arrive_as_one_notice_but_all_reach_the_log() {
    let app = App::new();
    start(&app);
    for title in ["One", "Two", "Three"] {
        app.task(title, json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    }
    app.set("2026-11-03T09:00");
    assert_eq!(app.tick(), ["3 reminders while Dayweave was closed"]);
    assert_eq!(app.call("notifications.list", json!({})).as_array().unwrap().len(), 3);
}

#[test]
fn reminders_too_old_to_be_useful_are_dropped() {
    let app = App::new();
    start(&app);
    app.task("Ancient", json!({ "dueDate": "2026-11-03", "reminders": [before(0)] }));
    app.set("2026-12-20T09:00");
    assert!(app.tick().is_empty());
}

#[test]
fn the_clock_moving_backwards_cannot_repeat_a_notification() {
    let app = App::new();
    start(&app);
    app.task("Once", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    app.set("2026-11-02T12:00");
    assert_eq!(app.tick().len(), 1);
    app.set("2026-11-02T11:00");
    assert!(app.tick().is_empty());
    app.set("2026-11-02T12:05");
    assert!(app.tick().is_empty());
}

#[test]
fn a_custom_reminder_fires_at_its_exact_time() {
    let app = App::new();
    start(&app);
    app.task("Call", json!({ "reminders": [{ "kind": "at", "remindAt": "2026-11-02T14:15" }] }));
    app.set("2026-11-02T14:15");
    assert_eq!(app.tick(), ["Call"]);
}

#[test]
fn snoozing_fires_again_later_and_leaves_no_visible_reminder() {
    let app = App::new();
    start(&app);
    let id = app
        .task("Later", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }))
        ["id"]
        .clone();
    app.set("2026-11-02T12:00");
    assert_eq!(app.tick(), ["Later"]);
    app.call("tasks.snooze", json!({ "id": id, "minutes": 10 }));
    assert_eq!(app.call("tasks.get", json!({ "id": id }))["reminders"].as_array().unwrap().len(), 1);

    app.set("2026-11-02T12:09");
    assert!(app.tick().is_empty());
    app.set("2026-11-02T12:10");
    assert_eq!(app.tick(), ["Later"]);
    app.set("2026-11-02T12:30");
    assert!(app.tick().is_empty());
}

#[test]
fn a_reminder_stays_at_its_wall_clock_time_across_daylight_saving() {
    let app = App::at(New_York, "2026-10-31T08:00");
    start(&app);
    app.task(
        "Sunday check",
        json!({ "dueDate": "2026-11-01", "dueTime": "09:00", "reminders": [before(0)] }),
    );
    // 2026-11-01 is 25 hours long in New York, with 01:00 to 02:00 happening twice.
    app.set("2026-11-01T08:59");
    assert!(app.tick().is_empty());
    app.set("2026-11-01T09:00");
    assert_eq!(app.tick(), ["Sunday check"]);
}

#[test]
fn a_reminder_in_a_skipped_hour_fires_when_the_clock_lands() {
    let app = App::at(New_York, "2027-03-13T10:00");
    start(&app);
    // Clocks jump from 02:00 to 03:00 on 2027-03-14, so 02:30 does not exist.
    app.task("Skipped", json!({ "dueDate": "2027-03-14", "dueTime": "02:30", "reminders": [before(0)] }));
    app.set("2027-03-14T01:59");
    assert!(app.tick().is_empty());
    app.set("2027-03-14T03:00");
    assert_eq!(app.tick(), ["Skipped"]);
    app.set("2027-03-14T04:00");
    assert!(app.tick().is_empty());
}

#[test]
fn a_habit_reminder_fires_on_scheduled_days_until_it_is_done() {
    let app = App::new();
    start(&app);
    let id = app.habit(
        "Evening walk",
        json!({ "frequency": "weekdays", "weekdays": [0, 2], "reminderTime": "18:00" }),
    )["id"]
        .clone();
    app.set("2026-11-02T18:00");
    assert_eq!(app.tick(), ["Evening walk"]);

    // Tuesday is not scheduled.
    app.set("2026-11-03T18:00");
    assert!(app.tick().is_empty());

    // Wednesday is, but it was done early.
    app.set("2026-11-04T17:00");
    app.call("habits.log", json!({ "id": id }));
    app.set("2026-11-04T18:00");
    assert!(app.tick().is_empty());
}

#[test]
fn notification_settings_silence_the_desktop_but_keep_the_log() {
    let app = App::new();
    start(&app);
    app.task("Quiet", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    app.call("settings.set", json!({ "key": "notificationsEnabled", "value": false }));
    app.set("2026-11-02T12:00");
    assert!(app.tick().is_empty());
    assert_eq!(app.call("notifications.list", json!({})).as_array().unwrap().len(), 1);
}

#[test]
fn category_switches_apply_to_their_own_kind() {
    let app = App::new();
    start(&app);
    app.call("settings.set", json!({ "key": "habitNotifications", "value": false }));
    app.habit("Silent", json!({ "reminderTime": "12:00" }));
    app.task("Loud", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    app.set("2026-11-02T12:00");
    assert_eq!(app.tick(), ["Loud"]);
}

#[test]
fn the_notification_center_tracks_unread_and_clearing_cannot_refire() {
    let app = App::new();
    start(&app);
    app.task("Log me", json!({ "dueDate": "2026-11-02", "dueTime": "12:00", "reminders": [before(0)] }));
    app.set("2026-11-02T12:00");
    app.tick();
    assert_eq!(app.call("overview.badges", json!({}))["unread"], 1);

    app.call("notifications.markRead", json!({}));
    assert_eq!(app.call("overview.badges", json!({}))["unread"], 0);
    assert_eq!(app.call("notifications.list", json!({}))[0]["read"], true);

    app.call("notifications.clear", json!({}));
    assert_eq!(app.call("notifications.list", json!({})), json!([]));
    app.set("2026-11-02T11:00");
    app.tick();
    app.set("2026-11-02T12:01");
    assert!(app.tick().is_empty(), "the cleared entry still prevents a repeat");
}
