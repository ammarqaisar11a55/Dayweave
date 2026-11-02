mod common;

use common::App;
use serde_json::{json, Value};

fn day<'a>(days: &'a Value, date: &str) -> &'a Value {
    days.as_array().unwrap().iter().find(|day| day["date"] == date).unwrap()
}

fn kinds(day: &Value) -> Vec<&str> {
    day["items"].as_array().unwrap().iter().map(|item| item["kind"].as_str().unwrap()).collect()
}

#[test]
fn the_overview_measures_todays_progress() {
    let app = App::new();
    let first = app.task("Due today", json!({ "dueDate": "2026-11-02" }));
    app.task("Also today", json!({ "dueDate": "2026-11-02", "dueTime": "16:00" }));
    app.task("Next week", json!({ "dueDate": "2026-11-09" }));
    app.task("Far away", json!({ "dueDate": "2027-01-01" }));
    app.task("Overdue", json!({ "dueDate": "2026-10-20" }));
    let habit = app.habit("Exercise", json!({}));
    app.habit("Gym", json!({ "frequency": "weekdays", "weekdays": [4] }));

    app.call("tasks.complete", json!({ "id": first["id"] }));
    app.call("habits.log", json!({ "id": habit["id"] }));

    let overview = app.call("overview.get", json!({}));
    assert_eq!(overview["date"], "2026-11-02");
    assert_eq!(overview["tasksTotal"], 2);
    assert_eq!(overview["tasksDone"], 1);
    assert_eq!(overview["habitsTotal"], 1, "Friday's habit isn't due on a Monday");
    assert_eq!(overview["habitsDone"], 1);
    assert!((overview["progress"].as_f64().unwrap() - 2.0 / 3.0).abs() < 1e-9);
    assert_eq!(common::titles(&overview["overdue"]), ["Overdue"]);
    assert_eq!(common::titles(&overview["upcoming"]), ["Next week"]);
    assert_eq!(overview["tasks"][0]["title"], "Also today", "unfinished tasks come first");
}

#[test]
fn a_task_completed_today_stays_in_todays_count_even_if_it_was_due_earlier() {
    let app = App::new();
    let task = app.task("Late finish", json!({ "dueDate": "2026-10-20" }));
    app.call("tasks.complete", json!({ "id": task["id"] }));
    let overview = app.call("overview.get", json!({}));
    assert_eq!(overview["tasksDone"], 1);
    assert_eq!(overview["overdue"], json!([]));
}

#[test]
fn a_quit_habit_counts_as_met_until_a_slip_is_logged() {
    let app = App::new();
    let habit = app.habit("Avoid sugar", json!({ "kind": "quit" }));
    assert_eq!(app.call("overview.get", json!({}))["habitsDone"], 1);
    app.call("habits.log", json!({ "id": habit["id"], "slip": true }));
    assert_eq!(app.call("overview.get", json!({}))["habitsDone"], 0);
}

#[test]
fn the_overview_week_follows_the_week_start_setting() {
    let app = App::new();
    assert_eq!(app.call("overview.get", json!({}))["week"][0]["date"], "2026-11-02");
    app.call("settings.set", json!({ "key": "weekStart", "value": "sunday" }));
    assert_eq!(app.call("overview.get", json!({}))["week"][0]["date"], "2026-11-01");
}

#[test]
fn an_empty_day_reports_no_progress_rather_than_dividing_by_zero() {
    let app = App::new();
    assert_eq!(app.call("overview.get", json!({}))["progress"], 0.0);
}

#[test]
fn the_calendar_separates_tasks_deadlines_habits_focus_and_reminders() {
    let app = App::new();
    app.task(
        "Report",
        json!({
            "startDate": "2026-11-02",
            "dueDate": "2026-11-04",
            "dueTime": "17:00",
            "reminders": [{ "kind": "before", "offsetMinutes": 60 }],
        }),
    );
    let habit = app.habit("Walk", json!({ "startDate": "2026-11-02" }));
    app.call("habits.log", json!({ "id": habit["id"] }));
    app.call("focus.start", json!({ "kind": "focus", "minutes": 25 }));
    app.advance_minutes(25);
    app.tick();

    let days = app.call("calendar.range", json!({ "from": "2026-11-01", "to": "2026-11-05" }));
    let monday = day(&days, "2026-11-02");
    assert_eq!(kinds(monday), ["focus", "task", "habit"]);
    assert_eq!(monday["habitsDone"], 1);
    assert_eq!(monday["habitsDue"], 1);
    assert_eq!(monday["focusSeconds"], 1500);

    let wednesday = day(&days, "2026-11-04");
    assert_eq!(kinds(wednesday), ["reminder", "deadline", "habit"]);
    assert_eq!(wednesday["items"][0]["time"], "16:00");
    assert_eq!(wednesday["items"][1]["time"], "17:00");
    assert_eq!(kinds(day(&days, "2026-11-01")), Vec::<&str>::new());
}

#[test]
fn a_task_starting_and_due_on_the_same_day_appears_once() {
    let app = App::new();
    app.task("Same day", json!({ "startDate": "2026-11-03", "dueDate": "2026-11-03" }));
    let days = app.call("calendar.range", json!({ "from": "2026-11-03", "to": "2026-11-03" }));
    assert_eq!(kinds(&days[0]), ["deadline"]);
}

#[test]
fn overdue_and_completed_tasks_are_marked_on_the_calendar() {
    let app = App::new();
    app.task("Late", json!({ "dueDate": "2026-10-30" }));
    let done = app.task("Done", json!({ "dueDate": "2026-10-30" }));
    app.call("tasks.complete", json!({ "id": done["id"] }));
    let days = app.call("calendar.range", json!({ "from": "2026-10-30", "to": "2026-10-30" }));
    let items = days[0]["items"].as_array().unwrap();
    let late = items.iter().find(|item| item["title"] == "Late").unwrap();
    let finished = items.iter().find(|item| item["title"] == "Done").unwrap();
    assert_eq!(late["overdue"], true);
    assert_eq!(finished["completed"], true);
    assert_eq!(finished["overdue"], false);
}

#[test]
fn the_calendar_limits_how_much_it_loads_at_once() {
    let app = App::new();
    let error = app.call_err("calendar.range", json!({ "from": "2026-01-01", "to": "2026-12-31" }));
    assert!(error.to_string().contains("two months"));
}

#[test]
fn analytics_summarise_tasks_habits_and_focus() {
    let app = App::new();
    let project = app.call("projects.create", json!({ "name": "Thesis" }));
    let finished = app.task("Finished", json!({ "priority": "high", "projectId": project["id"] }));
    app.call("tasks.complete", json!({ "id": finished["id"] }));
    app.task("Open", json!({ "priority": "urgent", "projectId": project["id"] }));
    app.task("Late", json!({ "dueDate": "2026-10-01", "priority": "low" }));

    let good = app.habit("Steady", json!({ "startDate": "2026-11-01" }));
    app.call("habits.log", json!({ "id": good["id"], "date": "2026-11-01" }));
    app.call("habits.log", json!({ "id": good["id"] }));
    app.habit("Neglected", json!({ "startDate": "2026-10-20" }));

    app.call("focus.start", json!({ "kind": "focus", "minutes": 25 }));
    app.advance_minutes(25);
    app.tick();

    let stats = app.call("analytics.get", json!({ "days": 7 }));
    assert_eq!(stats["tasksCompleted"], 1);
    assert_eq!(stats["tasksOverdue"], 1);
    assert_eq!(stats["tasksPending"], 2);
    assert_eq!(stats["taskCompletionRate"], 0.5);
    assert_eq!(stats["focusSeconds"], 1500);
    assert_eq!(stats["focusSessions"], 1);
    assert_eq!(stats["daily"].as_array().unwrap().len(), 7);
    assert_eq!(stats["daily"][6]["tasksCompleted"], 1);
    assert_eq!(stats["daily"][6]["tasksCreated"], 3);
    assert_eq!(stats["priorities"][0]["priority"], "urgent");
    assert_eq!(stats["projects"][0]["completion"], 0.5);
    assert_eq!(stats["mostConsistent"][0]["name"], "Steady");
    assert_eq!(stats["needsImprovement"][0]["name"], "Neglected");
}

#[test]
fn analytics_handle_an_empty_database() {
    let app = App::new();
    let stats = app.call("analytics.get", json!({ "days": 30 }));
    assert_eq!(stats["taskCompletionRate"], 0.0);
    assert_eq!(stats["habitCompletionRate"], 0.0);
    assert_eq!(stats["habits"], json!([]));
    assert_eq!(stats["daily"].as_array().unwrap().len(), 30);
}

#[test]
fn a_task_created_late_at_night_counts_on_its_local_day() {
    let app = App::new();
    app.set("2026-11-02T01:00");
    app.task("Night owl", json!({}));
    let stats = app.call("analytics.get", json!({ "days": 2 }));
    assert_eq!(stats["daily"][1]["date"], "2026-11-02");
    assert_eq!(stats["daily"][1]["tasksCreated"], 1, "the UTC date is still the 1st here");
}

#[test]
fn search_covers_tasks_habits_projects_and_tags() {
    let app = App::new();
    app.task("Study graphs", json!({ "tags": ["study-group"] }));
    app.habit("Study DSA", json!({}));
    app.call("projects.create", json!({ "name": "Study abroad" }));
    app.task("Unrelated", json!({}));

    let results = app.call("search.query", json!({ "query": "study" }));
    assert_eq!(results["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(results["habits"][0]["name"], "Study DSA");
    assert_eq!(results["projects"][0]["name"], "Study abroad");
    assert_eq!(results["tags"][0]["name"], "study-group");
    assert_eq!(app.call("search.query", json!({ "query": "   " }))["tasks"], json!([]));
}
