mod common;

use common::{titles, App};
use serde_json::json;

#[test]
fn a_task_keeps_everything_it_was_created_with() {
    let app = App::new();
    let task = app.task(
        "Prepare Software Engineering Assignment",
        json!({
            "description": "Weekly submission",
            "priority": "high",
            "category": "University",
            "dueDate": "2026-11-06",
            "dueTime": "17:00",
            "startDate": "2026-11-03",
            "estimatedMinutes": 120,
            "tags": ["uni", "#writing"],
            "subtasks": ["Research topic", "Write introduction"],
            "reminders": [{ "kind": "before", "offsetMinutes": 60 }],
        }),
    );
    let loaded = app.call("tasks.get", json!({ "id": task["id"] }));
    assert_eq!(loaded["priority"], "high");
    assert_eq!(loaded["status"], "todo");
    assert_eq!(loaded["dueTime"], "17:00");
    assert_eq!(loaded["estimatedMinutes"], 120);
    assert_eq!(loaded["subtasks"].as_array().unwrap().len(), 2);
    assert_eq!(loaded["reminders"][0]["offsetMinutes"], 60);
    assert_eq!(loaded["tags"][1]["name"], "writing");
    assert_eq!(loaded["subtaskTotal"], 2);
    assert_eq!(loaded["createdAt"], "2026-11-02T05:00:00Z");
}

#[test]
fn invalid_tasks_are_rejected_with_a_useful_message() {
    let app = App::new();
    let cases = [
        (json!({ "title": "   " }), "title"),
        (json!({ "title": "a", "priority": "extreme" }), "priority"),
        (json!({ "title": "a", "dueTime": "10:00" }), "due date"),
        (json!({ "title": "a", "dueDate": "soon" }), "valid date"),
        (json!({ "title": "a", "dueDate": "2026-11-01", "startDate": "2026-11-02" }), "start date"),
        (json!({ "title": "a", "reminders": [{ "kind": "before", "offsetMinutes": 10 }] }), "due date"),
        (json!({ "title": "a", "recurrence": { "frequency": "daily" } }), "due date"),
        (json!({ "title": "a", "dueDate": "2026-11-02", "recurrence": { "frequency": "hourly" } }), "repeat"),
    ];
    for (input, expected) in cases {
        let message = app.call_err("tasks.create", input.clone()).to_string();
        assert!(message.to_lowercase().contains(expected), "{input}: {message}");
    }
    assert_eq!(app.list("all").len(), 0);
}

#[test]
fn updating_replaces_fields_tags_and_reminders() {
    let app = App::new();
    let task = app.task("Draft", json!({ "dueDate": "2026-11-06", "tags": ["a", "b"] }));
    let updated = app.call(
        "tasks.update",
        json!({
            "id": task["id"],
            "title": "Final",
            "dueDate": "2026-11-07",
            "tags": ["b", "c"],
            "reminders": [{ "kind": "before", "offsetMinutes": 0 }],
        }),
    );
    assert_eq!(updated["title"], "Final");
    assert_eq!(updated["dueDate"], "2026-11-07");
    let names: Vec<_> = updated["tags"].as_array().unwrap().iter().map(|tag| &tag["name"]).collect();
    assert_eq!(names, ["b", "c"]);
    assert_eq!(updated["reminderCount"], 1);
    let tags = app.call("tasks.tags", json!({}));
    assert_eq!(tags.as_array().unwrap().len(), 2, "the unused tag is removed");
}

#[test]
fn views_return_the_right_tasks() {
    let app = App::new();
    app.task("No date", json!({}));
    app.task("Past", json!({ "dueDate": "2026-11-01" }));
    app.task("Earlier today", json!({ "dueDate": "2026-11-02", "dueTime": "08:00" }));
    app.task("Later today", json!({ "dueDate": "2026-11-02", "dueTime": "18:00" }));
    app.task("All day today", json!({ "dueDate": "2026-11-02" }));
    app.task("Starts today", json!({ "startDate": "2026-11-02", "dueDate": "2026-11-10" }));
    app.task("Tomorrow", json!({ "dueDate": "2026-11-03" }));
    let done = app.task("Finished", json!({ "dueDate": "2026-10-01" }));
    app.call("tasks.complete", json!({ "id": done["id"] }));
    let archived = app.task("Stored", json!({ "dueDate": "2026-10-01" }));
    app.call("tasks.archive", json!({ "id": archived["id"] }));

    assert_eq!(app.list("overdue"), ["Past", "Earlier today"]);
    assert_eq!(app.list("today"), ["Earlier today", "Later today", "All day today", "Starts today"]);
    assert_eq!(app.list("upcoming"), ["Tomorrow", "Starts today"]);
    assert_eq!(app.list("completed"), ["Finished"]);
    assert_eq!(app.list("archived"), ["Stored"]);
    assert_eq!(app.list("all").len(), 8);

    let counts = app.call("tasks.counts", json!({}));
    assert_eq!(counts["overdue"], 2);
    assert_eq!(counts["today"], 4);
    assert_eq!(counts["upcoming"], 2);
    assert_eq!(counts["completed"], 1);
    assert_eq!(counts["archived"], 1);
}

#[test]
fn a_date_only_deadline_is_not_overdue_until_the_day_has_passed() {
    let app = App::new();
    app.task("All day", json!({ "dueDate": "2026-11-02" }));
    app.set("2026-11-02T23:59");
    assert!(app.list("overdue").is_empty());
    app.set("2026-11-03T00:00");
    assert_eq!(app.list("overdue"), ["All day"]);
}

#[test]
fn a_timed_deadline_is_overdue_the_minute_after_it_passes() {
    let app = App::new();
    let task = app.task("Timed", json!({ "dueDate": "2026-11-02", "dueTime": "11:00" }));
    app.set("2026-11-02T11:00");
    assert!(app.list("overdue").is_empty());
    app.set("2026-11-02T11:01");
    assert_eq!(app.list("overdue"), ["Timed"]);
    assert_eq!(app.call("tasks.get", json!({ "id": task["id"] }))["overdue"], true);
}

#[test]
fn completed_and_archived_tasks_are_never_overdue() {
    let app = App::new();
    let task = app.task("Old", json!({ "dueDate": "2026-01-01" }));
    assert_eq!(task["overdue"], true);
    let completed = app.call("tasks.complete", json!({ "id": task["id"] }));
    assert_eq!(completed["task"]["overdue"], false);
    assert!(app.list("overdue").is_empty());

    let other = app.task("Other", json!({ "dueDate": "2026-01-01" }));
    let archived = app.call("tasks.archive", json!({ "id": other["id"] }));
    assert_eq!(archived["overdue"], false);
    assert!(app.list("overdue").is_empty());
}

#[test]
fn a_task_can_be_completed_reopened_archived_and_restored() {
    let app = App::new();
    let id = app.task("Cycle", json!({}))["id"].clone();

    let completed = app.call("tasks.complete", json!({ "id": id }));
    assert_eq!(completed["task"]["status"], "completed");
    assert_eq!(completed["task"]["completedOn"], "2026-11-02");
    assert!(completed["task"]["completedAt"].is_string());

    let reopened = app.call("tasks.reopen", json!({ "id": id }));
    assert_eq!(reopened["status"], "todo");
    assert!(reopened["completedAt"].is_null());

    app.call("tasks.update", json!({ "id": id, "title": "Cycle", "status": "in_progress" }));
    assert_eq!(app.call("tasks.archive", json!({ "id": id }))["status"], "archived");
    let restored = app.call("tasks.restore", json!({ "id": id }));
    assert_eq!(restored["status"], "in_progress", "a restored task returns to where it was");
    assert!(restored["archivedAt"].is_null());
}

#[test]
fn an_archived_task_has_to_be_restored_before_completing() {
    let app = App::new();
    let id = app.task("Stored", json!({}))["id"].clone();
    app.call("tasks.archive", json!({ "id": id }));
    assert!(app.call_err("tasks.complete", json!({ "id": id })).to_string().contains("Restore"));
}

#[test]
fn status_changes_through_update_cannot_skip_completion_bookkeeping() {
    let app = App::new();
    let id = app.task("Sneaky", json!({}))["id"].clone();
    let error = app.call_err("tasks.update", json!({ "id": id, "title": "Sneaky", "status": "completed" }));
    assert!(error.to_string().contains("Complete"));
}

#[test]
fn deleting_a_task_removes_its_children() {
    let app = App::new();
    let id = app.task("Gone", json!({ "dueDate": "2026-11-03", "subtasks": ["one"], "tags": ["solo"] }))
        ["id"]
        .clone();
    app.call("tasks.delete", json!({ "id": id }));
    assert_eq!(app.call_err("tasks.get", json!({ "id": id })).code(), "not_found");
    assert_eq!(app.call("tasks.tags", json!({})), json!([]));
    assert_eq!(app.call_err("tasks.delete", json!({ "id": id })).code(), "not_found");
}

#[test]
fn duplicating_copies_the_details_and_resets_progress() {
    let app = App::new();
    let source = app.task(
        "Template",
        json!({ "priority": "urgent", "dueDate": "2026-11-05", "tags": ["x"], "subtasks": ["a", "b"],
                "reminders": [{ "kind": "before", "offsetMinutes": 30 }] }),
    );
    let subtask = app.call("tasks.get", json!({ "id": source["id"] }))["subtasks"][0]["id"].clone();
    app.call("subtasks.toggle", json!({ "id": subtask }));
    app.call("tasks.complete", json!({ "id": source["id"] }));

    let copy = app.call("tasks.duplicate", json!({ "id": source["id"] }));
    assert_eq!(copy["title"], "Template (copy)");
    assert_eq!(copy["status"], "todo");
    assert_eq!(copy["priority"], "urgent");
    assert_eq!(copy["subtaskTotal"], 2);
    assert_eq!(copy["subtaskDone"], 0);
    assert_eq!(copy["reminderCount"], 1);
    assert_eq!(copy["tags"][0]["name"], "x");
}

#[test]
fn searching_finds_titles_and_descriptions_and_treats_wildcards_literally() {
    let app = App::new();
    app.task("100% done", json!({}));
    app.task("Plain", json!({ "description": "contains the word needle" }));
    let search = |text: &str| titles(&app.call("tasks.list", json!({ "search": text })));
    assert_eq!(search("needle"), ["Plain"]);
    assert_eq!(search("100%"), ["100% done"]);
    assert!(search("1%0").is_empty());
    assert!(search("P_ain").is_empty());
}

#[test]
fn filters_combine() {
    let app = App::new();
    let project = app.call("projects.create", json!({ "name": "Thesis" }))["id"].clone();
    app.task("A", json!({ "priority": "high", "category": "Work", "tags": ["deep"], "dueDate": "2026-11-05", "projectId": project }));
    app.task("B", json!({ "priority": "high", "category": "Home", "dueDate": "2026-11-20" }));
    app.task("C", json!({ "priority": "low", "category": "Work", "tags": ["deep"] }));
    let filter = |params| titles(&app.call("tasks.list", params));

    assert_eq!(filter(json!({ "priorities": ["high"] })), ["A", "B"]);
    assert_eq!(filter(json!({ "category": "Work" })), ["A", "C"]);
    assert_eq!(filter(json!({ "tag": "DEEP" })), ["A", "C"]);
    assert_eq!(filter(json!({ "projectId": project })), ["A"]);
    assert_eq!(filter(json!({ "dateFrom": "2026-11-10" })), ["B"]);
    assert_eq!(filter(json!({ "dateTo": "2026-11-10" })), ["A"]);
    assert_eq!(filter(json!({ "priorities": ["high"], "category": "Home" })), ["B"]);
    assert_eq!(filter(json!({ "statuses": ["todo"], "priorities": ["low"] })), ["C"]);
}

#[test]
fn every_sort_order_works_and_tasks_without_a_deadline_come_last() {
    let app = App::new();
    app.task("Banana", json!({ "priority": "low", "dueDate": "2026-11-09" }));
    app.task("apple", json!({ "priority": "urgent" }));
    app.task("Cherry", json!({ "priority": "medium", "dueDate": "2026-11-03", "dueTime": "09:00" }));
    app.task("Date", json!({ "priority": "medium", "dueDate": "2026-11-03" }));
    let sorted = |sort: &str, descending: bool| {
        titles(&app.call("tasks.list", json!({ "sort": sort, "descending": descending })))
    };
    assert_eq!(sorted("deadline", false), ["Cherry", "Date", "Banana", "apple"]);
    assert_eq!(sorted("deadline", true), ["Banana", "Cherry", "Date", "apple"]);
    assert_eq!(sorted("priority", false), ["apple", "Cherry", "Date", "Banana"]);
    assert_eq!(sorted("title", false), ["apple", "Banana", "Cherry", "Date"]);
    assert_eq!(sorted("created", true), ["Date", "Cherry", "apple", "Banana"]);

    let done = app.call("tasks.list", json!({}))[0]["id"].clone();
    app.call("tasks.complete", json!({ "id": done }));
    assert_eq!(sorted("status", false).last().unwrap(), "Cherry");
    assert!(app
        .call_err("tasks.list", json!({ "sort": "1; DROP TABLE tasks" }))
        .to_string()
        .contains("Unknown sort"));
}

#[test]
fn rescheduling_moves_a_task_out_of_overdue() {
    let app = App::new();
    let id = app.task("Late", json!({ "dueDate": "2026-10-01", "dueTime": "09:00" }))["id"].clone();
    assert_eq!(app.list("overdue"), ["Late"]);
    let moved = app.call("tasks.reschedule", json!({ "id": id, "dueDate": "2026-11-09" }));
    assert_eq!(moved["dueDate"], "2026-11-09");
    assert!(moved["dueTime"].is_null());
    assert!(app.list("overdue").is_empty());
}

#[test]
fn rescheduling_before_the_start_date_moves_the_start_too() {
    let app = App::new();
    let id = app.task("Move", json!({ "startDate": "2026-11-10", "dueDate": "2026-11-12" }))["id"].clone();
    let moved = app.call("tasks.reschedule", json!({ "id": id, "dueDate": "2026-11-05" }));
    assert_eq!(moved["startDate"], "2026-11-05");
}

#[test]
fn clearing_a_date_is_refused_while_a_reminder_depends_on_it() {
    let app = App::new();
    let id = app.task(
        "Dated",
        json!({ "dueDate": "2026-11-05", "reminders": [{ "kind": "before", "offsetMinutes": 0 }] }),
    )["id"]
        .clone();
    assert!(app.call_err("tasks.reschedule", json!({ "id": id })).to_string().contains("reminders"));
}

#[test]
fn subtasks_track_progress_including_nested_ones() {
    let app = App::new();
    let id = app.task("Parent", json!({ "subtasks": ["Research"] }))["id"].clone();
    let task = app.call("tasks.get", json!({ "id": id }));
    let research = task["subtasks"][0]["id"].clone();

    let task =
        app.call("subtasks.add", json!({ "taskId": id, "parentId": research, "title": "Find sources" }));
    let task_two = app.call("subtasks.add", json!({ "taskId": id, "title": "Write" }));
    assert_eq!(task["subtaskTotal"], 2);
    assert_eq!(task_two["subtaskTotal"], 3);

    let toggled = app.call("subtasks.toggle", json!({ "id": research }));
    assert_eq!(toggled["subtaskDone"], 2, "completing a parent completes what is nested under it");
    let toggled = app.call("subtasks.toggle", json!({ "id": research }));
    assert_eq!(toggled["subtaskDone"], 0, "and reopening it reopens them");

    let renamed = app.call("subtasks.rename", json!({ "id": research, "title": "Reading" }));
    assert_eq!(renamed["subtasks"][0]["title"], "Reading");

    let removed = app.call("subtasks.delete", json!({ "id": research }));
    assert_eq!(removed["subtaskTotal"], 1, "deleting a parent removes what is nested under it");
}

#[test]
fn a_subtask_cannot_be_nested_under_another_task() {
    let app = App::new();
    let first = app.task("First", json!({ "subtasks": ["a"] }));
    let second = app.task("Second", json!({}));
    let subtask = app.call("tasks.get", json!({ "id": first["id"] }))["subtasks"][0]["id"].clone();
    let error =
        app.call_err("subtasks.add", json!({ "taskId": second["id"], "parentId": subtask, "title": "x" }));
    assert_eq!(error.code(), "not_found");
}

#[test]
fn projects_group_tasks_and_track_completion() {
    let app = App::new();
    let project = app.call(
        "projects.create",
        json!({ "name": "Thesis", "color": "blue", "icon": "book", "deadline": "2026-12-01" }),
    );
    let id = project["id"].clone();
    let first = app.task("One", json!({ "projectId": id }));
    app.task("Two", json!({ "projectId": id, "dueDate": "2026-10-01" }));
    assert_eq!(first["projectName"], "Thesis");
    assert_eq!(first["projectColor"], "blue");

    app.call("tasks.complete", json!({ "id": first["id"] }));
    let loaded = app.call("projects.get", json!({ "id": id }));
    assert_eq!(loaded["taskTotal"], 2);
    assert_eq!(loaded["taskDone"], 1);
    assert_eq!(loaded["taskOverdue"], 1);
    assert_eq!(loaded["completion"], 0.5);

    let visible = |extra| app.call("projects.list", extra).as_array().unwrap().len();
    app.call("projects.archive", json!({ "id": id }));
    assert_eq!(visible(json!({})), 0);
    assert_eq!(visible(json!({ "includeArchived": true })), 1);
    app.call("projects.restore", json!({ "id": id }));
    assert_eq!(visible(json!({})), 1);
}

#[test]
fn deleting_a_project_keeps_its_tasks() {
    let app = App::new();
    let id = app.call("projects.create", json!({ "name": "Temp" }))["id"].clone();
    let task = app.task("Survivor", json!({ "projectId": id }));
    app.call("projects.delete", json!({ "id": id }));
    assert!(app.call("tasks.get", json!({ "id": task["id"] }))["projectId"].is_null());
}

#[test]
fn a_task_can_be_associated_with_a_habit() {
    let app = App::new();
    let habit = app.habit("Study DSA", json!({}));
    let task = app.task("Solve graphs", json!({ "habitId": habit["id"] }));
    assert_eq!(task["habitName"], "Study DSA");
    app.call("habits.delete", json!({ "id": habit["id"] }));
    assert!(app.call("tasks.get", json!({ "id": task["id"] }))["habitId"].is_null());
}

#[test]
fn an_unknown_method_is_an_error() {
    let app = App::new();
    assert_eq!(app.call_err("tasks.explode", json!({})).code(), "unknown_method");
}
