mod common;

use common::App;
use serde_json::json;

#[test]
fn logging_a_habit_updates_its_streak_and_undo_takes_it_back() {
    let app = App::new();
    let id = app.habit("Exercise", json!({}))["id"].clone();
    for date in ["2026-10-30", "2026-10-31", "2026-11-01"] {
        app.call("habits.log", json!({ "id": id, "date": date }));
    }
    let habit = app.call("habits.log", json!({ "id": id }));
    assert_eq!(habit["today"]["state"], "done");
    assert_eq!(habit["stats"]["currentStreak"], 4);

    let undone = app.call("habits.undo", json!({ "id": id }));
    assert_eq!(undone["today"]["state"], "pending");
    assert_eq!(undone["stats"]["currentStreak"], 3);
}

#[test]
fn a_streak_survives_until_the_day_after_a_miss() {
    let app = App::new();
    let id = app.habit("Exercise", json!({}))["id"].clone();
    app.call("habits.log", json!({ "id": id }));
    app.set("2026-11-03T23:00");
    assert_eq!(app.call("habits.get", json!({ "id": id }))["stats"]["currentStreak"], 1);
    app.set("2026-11-04T00:30");
    assert_eq!(app.call("habits.get", json!({ "id": id }))["stats"]["currentStreak"], 0);
}

#[test]
fn a_quantity_habit_counts_only_once_the_target_is_met() {
    let app = App::new();
    let id = app.habit("Drink water", json!({ "kind": "quantity", "quantityTarget": 2.0, "unit": "liters" }))
        ["id"]
        .clone();

    let habit = app.call("habits.log", json!({ "id": id, "value": 1.25 }));
    assert_eq!(habit["today"]["state"], "pending");
    assert_eq!(habit["today"]["value"], 1.25);
    assert_eq!(habit["stats"]["currentStreak"], 0);

    let habit = app.call("habits.log", json!({ "id": id, "value": 2.0 }));
    assert_eq!(habit["today"]["state"], "done");
    assert_eq!(habit["stats"]["currentStreak"], 1);

    let habit = app.call("habits.log", json!({ "id": id, "value": 0.5 }));
    assert_eq!(habit["today"]["state"], "pending", "lowering the value takes the completion back");
}

#[test]
fn a_checklist_habit_counts_only_when_every_item_is_ticked() {
    let app = App::new();
    let habit = app.habit(
        "Morning routine",
        json!({ "kind": "checklist", "checklist": [{ "title": "Stretch" }, { "title": "Read" }] }),
    );
    let (id, first, second) =
        (habit["id"].clone(), habit["checklist"][0]["id"].clone(), habit["checklist"][1]["id"].clone());

    let logged = app.call("habits.log", json!({ "id": id, "checkedItems": [first] }));
    assert_eq!(logged["today"]["state"], "pending");
    assert_eq!(logged["today"]["checkedItems"], json!([first]));

    let logged = app.call("habits.log", json!({ "id": id, "checkedItems": [first, second] }));
    assert_eq!(logged["today"]["state"], "done");
}

#[test]
fn editing_a_checklist_keeps_the_ids_of_items_that_remain() {
    let app = App::new();
    let habit = app.habit(
        "Routine",
        json!({ "kind": "checklist", "checklist": [{ "title": "One" }, { "title": "Two" }] }),
    );
    let first = habit["checklist"][0].clone();
    let updated = app.call(
        "habits.update",
        json!({ "id": habit["id"], "name": "Routine", "kind": "checklist",
                "checklist": [{ "id": first["id"], "title": "Renamed" }, { "title": "Three" }] }),
    );
    assert_eq!(updated["checklist"][0]["id"], first["id"]);
    assert_eq!(updated["checklist"][0]["title"], "Renamed");
    assert_eq!(updated["checklist"][1]["title"], "Three");
    assert_eq!(updated["checklist"].as_array().unwrap().len(), 2);
}

#[test]
fn a_quit_habit_needs_only_a_logged_slip_to_break() {
    let app = App::new();
    let id = app.habit("Avoid smoking", json!({ "kind": "quit", "startDate": "2026-10-28" }))["id"].clone();
    let habit = app.call("habits.get", json!({ "id": id }));
    assert_eq!(habit["stats"]["currentStreak"], 5);

    let slipped = app.call("habits.log", json!({ "id": id, "slip": true, "note": "Stressful day" }));
    assert_eq!(slipped["stats"]["currentStreak"], 0);
    assert_eq!(slipped["today"]["slipped"], true);
    assert_eq!(slipped["today"]["note"], "Stressful day");

    let undone = app.call("habits.undo", json!({ "id": id }));
    assert_eq!(undone["stats"]["currentStreak"], 5);
}

#[test]
fn a_note_on_a_quit_habit_does_not_count_as_a_slip() {
    let app = App::new();
    let id = app.habit("Avoid sugar", json!({ "kind": "quit", "startDate": "2026-10-28" }))["id"].clone();
    let habit = app.call("habits.log", json!({ "id": id, "note": "Easy today" }));
    assert_eq!(habit["today"]["slipped"], false);
    assert_eq!(habit["today"]["note"], "Easy today");
    assert_eq!(habit["stats"]["currentStreak"], 5);
}

#[test]
fn a_habit_shows_as_due_only_on_its_scheduled_days() {
    let app = App::new();
    // Tuesday and Thursday. 2026-11-02 is a Monday.
    let id = app.habit("Gym", json!({ "frequency": "weekdays", "weekdays": [1, 3] }))["id"].clone();
    assert_eq!(app.call("habits.get", json!({ "id": id }))["today"]["due"], false);
    assert_eq!(app.call("overview.get", json!({}))["habitsTotal"], 0);
    app.set("2026-11-03T10:00");
    assert_eq!(app.call("habits.get", json!({ "id": id }))["today"]["due"], true);
    assert_eq!(app.call("overview.get", json!({}))["habitsTotal"], 1);
}

#[test]
fn the_week_start_setting_reaches_weekly_targets() {
    let app = App::new();
    let id = app.habit("Run", json!({ "frequency": "weekly_target", "weeklyTarget": 2 }))["id"].clone();
    app.call("habits.log", json!({ "id": id, "date": "2026-11-01" }));
    app.call("habits.log", json!({ "id": id, "date": "2026-11-02" }));
    app.set("2026-11-07T10:00");
    let streak = |app: &App| app.call("habits.get", json!({ "id": id }))["stats"]["currentStreak"].clone();
    assert_eq!(streak(&app), 0);
    app.call("settings.set", json!({ "key": "weekStart", "value": "sunday" }));
    assert_eq!(streak(&app), 1);
}

#[test]
fn invalid_habits_and_logs_are_rejected() {
    let app = App::new();
    let cases = [
        (json!({ "name": " " }), "name"),
        (json!({ "name": "a", "kind": "maybe" }), "type"),
        (json!({ "name": "a", "frequency": "weekdays", "weekdays": [] }), "day"),
        (json!({ "name": "a", "frequency": "weekdays", "weekdays": [9] }), "day"),
        (json!({ "name": "a", "frequency": "weekly_target", "weeklyTarget": 9 }), "target"),
        (json!({ "name": "a", "kind": "quantity" }), "target"),
        (json!({ "name": "a", "kind": "quantity", "quantityTarget": 0.0 }), "target"),
        (json!({ "name": "a", "kind": "checklist" }), "checklist"),
        (json!({ "name": "a", "reminderTime": "soon" }), "time"),
        (json!({ "name": "a", "kind": "quit", "frequency": "weekly_target", "weeklyTarget": 3 }), "quitting"),
    ];
    for (input, expected) in cases {
        let message = app.call_err("habits.create", input.clone()).to_string();
        assert!(message.to_lowercase().contains(expected), "{input}: {message}");
    }

    let id = app.habit("Valid", json!({ "startDate": "2026-11-01" }))["id"].clone();
    assert!(app
        .call_err("habits.log", json!({ "id": id, "date": "2026-11-03" }))
        .to_string()
        .contains("future"));
    assert!(app
        .call_err("habits.log", json!({ "id": id, "date": "2026-10-31" }))
        .to_string()
        .contains("before"));
    assert!(app.call_err("habits.log", json!({ "id": id, "value": -1.0 })).to_string().contains("zero"));
}

#[test]
fn logging_the_same_day_twice_replaces_the_entry() {
    let app = App::new();
    let id = app.habit("Read", json!({}))["id"].clone();
    app.call("habits.log", json!({ "id": id, "note": "First" }));
    let habit = app.call("habits.log", json!({ "id": id, "note": "Second" }));
    assert_eq!(habit["today"]["note"], "Second");
    assert_eq!(habit["stats"]["totalDone"], 1);
}

#[test]
fn logging_a_note_keeps_the_recorded_value() {
    let app = App::new();
    let id = app.habit("Read", json!({ "kind": "quantity", "quantityTarget": 20.0, "unit": "pages" }))["id"]
        .clone();
    app.call("habits.log", json!({ "id": id, "value": 25.0 }));
    let habit = app.call("habits.log", json!({ "id": id, "note": "Good chapter" }));
    assert_eq!(habit["today"]["value"], 25.0);
    assert_eq!(habit["today"]["state"], "done");
}

#[test]
fn archived_habits_leave_today_and_can_come_back() {
    let app = App::new();
    let id = app.habit("Paused", json!({}))["id"].clone();
    app.call("habits.archive", json!({ "id": id }));
    assert_eq!(app.call("habits.list", json!({})), json!([]));
    assert_eq!(app.call("habits.list", json!({ "includeArchived": true })).as_array().unwrap().len(), 1);
    assert_eq!(app.call("overview.get", json!({}))["habitsTotal"], 0);
    app.call("habits.restore", json!({ "id": id }));
    assert_eq!(app.call("habits.list", json!({})).as_array().unwrap().len(), 1);
}

#[test]
fn deleting_a_habit_removes_its_history() {
    let app = App::new();
    let id = app.habit("Gone", json!({}))["id"].clone();
    app.call("habits.log", json!({ "id": id }));
    app.call("habits.delete", json!({ "id": id }));
    assert_eq!(app.call_err("habits.get", json!({ "id": id })).code(), "not_found");
}

#[test]
fn habits_can_be_reordered() {
    let app = App::new();
    let first = app.habit("First", json!({}))["id"].clone();
    let second = app.habit("Second", json!({}))["id"].clone();
    app.call("habits.reorder", json!({ "ids": [second, first] }));
    assert_eq!(app.call("habits.list", json!({}))[0]["name"], "Second");
}

#[test]
fn history_describes_each_day_in_a_month() {
    let app = App::new();
    let id = app
        .habit("Gym", json!({ "frequency": "weekdays", "weekdays": [0, 2], "startDate": "2026-10-26" }))
        ["id"]
        .clone();
    app.call("habits.log", json!({ "id": id, "date": "2026-10-26", "note": "Legs" }));
    let days = app.call("habits.history", json!({ "id": id, "from": "2026-10-25", "to": "2026-11-03" }));
    let state = |date: &str| {
        days.as_array().unwrap().iter().find(|day| day["date"] == date).unwrap()["state"].clone()
    };
    assert_eq!(state("2026-10-25"), "inactive");
    assert_eq!(state("2026-10-26"), "done");
    assert_eq!(state("2026-10-27"), "not_scheduled");
    assert_eq!(state("2026-10-28"), "missed");
    assert_eq!(state("2026-11-02"), "pending");
    assert_eq!(state("2026-11-03"), "inactive");
    assert_eq!(days[1]["note"], "Legs");
}

#[test]
fn the_heatmap_shows_intensity_for_one_habit_and_all_habits() {
    let app = App::new();
    let water = app.habit("Water", json!({ "kind": "quantity", "quantityTarget": 4.0 }))["id"].clone();
    let walk = app.habit("Walk", json!({}))["id"].clone();
    app.call("habits.log", json!({ "id": water, "date": "2026-11-01", "value": 1.0 }));
    app.call("habits.log", json!({ "id": water, "date": "2026-10-31", "value": 4.0 }));
    app.call("habits.log", json!({ "id": walk, "date": "2026-10-31" }));

    let day = |list: &serde_json::Value, date: &str| {
        list.as_array().unwrap().iter().find(|day| day["date"] == date).unwrap().clone()
    };
    let one = app.call("habits.heatmap", json!({ "id": water, "from": "2026-10-30", "to": "2026-11-02" }));
    assert_eq!(day(&one, "2026-10-31")["level"], 4);
    assert_eq!(day(&one, "2026-11-01")["level"], 1, "a quarter of the target");
    assert_eq!(day(&one, "2026-10-30")["missed"], true);

    let all = app.call("habits.heatmap", json!({ "from": "2026-10-30", "to": "2026-11-02" }));
    let both = day(&all, "2026-10-31");
    assert_eq!(
        (both["done"].as_i64(), both["due"].as_i64(), both["level"].as_i64()),
        (Some(2), Some(2), Some(4))
    );
    assert_eq!(day(&all, "2026-11-01")["missed"], false, "partial progress is not a miss");
}

#[test]
fn the_heatmap_rejects_absurd_ranges() {
    let app = App::new();
    let error = app.call_err("habits.heatmap", json!({ "from": "2020-01-01", "to": "2026-01-01" }));
    assert!(error.to_string().contains("two years"));
}
