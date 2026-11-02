//! Desktop notifications through the freedesktop service, which GNOME implements and which the snap's
//! `desktop` plug allows. Action buttons work without the window being open.

use dayweave_core::reminders::Notice;
use notify_rust::{Hint, Notification, Timeout};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::{show_window, Shell};

fn desktop_entry() -> &'static str {
    if std::env::var_os("SNAP_NAME").is_some() {
        "dayweave_dayweave"
    } else {
        "Dayweave"
    }
}

pub fn deliver(app: &AppHandle, notice: Notice) {
    let app = app.clone();
    // Waiting for an action blocks until the notification closes, so each gets its own thread.
    std::thread::spawn(move || {
        let mut notification = Notification::new();
        notification
            .appname("Dayweave")
            .summary(&notice.title)
            .body(&notice.body)
            .hint(Hint::DesktopEntry(desktop_entry().into()))
            .timeout(Timeout::Default)
            .action("default", "Open");
        if std::env::var_os("SNAP_NAME").is_none() {
            notification.icon("dayweave");
        }
        if notice.kind == "task" && notice.task_id.is_some() {
            notification.action("complete", "Complete");
            notification.action("snooze", "Snooze 10 min");
        }

        match notification.show() {
            Ok(handle) => handle.wait_for_action(|action| act(&app, &notice, action)),
            Err(error) => eprintln!("could not show a notification: {error}"),
        }
    });
}

fn act(app: &AppHandle, notice: &Notice, action: &str) {
    let core = app.state::<Shell>().core.clone();
    let result = match (action, notice.task_id) {
        ("complete", Some(id)) => core.rpc("tasks.complete", json!({ "id": id })).map(|_| ()),
        ("snooze", Some(id)) => core.rpc("tasks.snooze", json!({ "id": id, "minutes": 10 })).map(|_| ()),
        ("default", _) => {
            show_window(app);
            let target = json!({ "taskId": notice.task_id, "habitId": notice.habit_id, "kind": notice.kind });
            let _ = app.emit("open-notice", target);
            Ok(())
        }
        // Dismissed or expired.
        _ => return,
    };
    if let Err(error) = result {
        eprintln!("notification action failed: {error}");
    }
    let _ = app.emit("data-changed", ());
}
