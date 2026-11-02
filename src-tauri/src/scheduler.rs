//! Runs the core's reminder pass on a timer, whether or not the window is open.

use std::sync::Arc;
use std::thread::{self, Thread};
use std::time::Duration;

use dayweave_core::Core;
use tauri::{AppHandle, Emitter};

use crate::notifier;

const INTERVAL_SECONDS: i64 = 20;

/// Starts the loop. Unparking the returned thread makes it look again straight away, which the
/// shell does after any change to a focus session.
pub fn start(app: AppHandle, core: Arc<Core>) -> Thread {
    thread::spawn(move || loop {
        match core.tick() {
            Ok(notices) => {
                for notice in notices {
                    notifier::deliver(&app, notice);
                }
            }
            Err(error) => eprintln!("reminder check failed: {error}"),
        }
        // Keeps counts and overdue states current, including across midnight.
        let _ = app.emit("data-changed", ());

        let focus_ends = core.focus_remaining().ok().flatten().unwrap_or(INTERVAL_SECONDS);
        thread::park_timeout(Duration::from_secs(focus_ends.clamp(1, INTERVAL_SECONDS) as u64));
    })
    .thread()
    .clone()
}
