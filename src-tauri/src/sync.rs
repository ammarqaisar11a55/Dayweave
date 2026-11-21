//! Keeps this computer in step with the sync server, when the person has signed in to one. Changes are
//! sent within seconds while there's a connection; offline, they wait and nothing else is affected.

use std::sync::Arc;
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use dayweave_core::{Core, Error};
use tauri::{AppHandle, Emitter};

/// How soon waiting changes are sent.
const CHANGES_EVERY: Duration = Duration::from_secs(10);
/// How often to fetch other devices' changes when nothing here has changed.
const FETCH_EVERY: Duration = Duration::from_secs(5 * 60);
/// How long to wait after the server couldn't be reached.
const RETRY_AFTER: Duration = Duration::from_secs(60);

/// Starts the loop. Unparking the returned thread makes it look again straight away.
pub fn start(app: AppHandle, core: Arc<Core>, scheduler: Thread) -> Thread {
    thread::spawn(move || {
        // Fetch straight away on start, so another device's changes appear first thing.
        let mut last_sync: Option<Instant> = None;
        loop {
            let mut wait = CHANGES_EVERY;
            if core.sync_enabled().unwrap_or(false) {
                let pending = core.sync_pending().unwrap_or(0);
                let due = last_sync.is_none_or(|time| time.elapsed() >= FETCH_EVERY);
                if pending > 0 || due {
                    match core.sync_now() {
                        Ok(report) => {
                            last_sync = Some(Instant::now());
                            if report.received > 0 {
                                // New deadlines and reminders may have arrived.
                                scheduler.unpark();
                            }
                        }
                        Err(Error::Offline(_)) => wait = RETRY_AFTER,
                        Err(error) => {
                            eprintln!("sync failed: {error}");
                            wait = RETRY_AFTER;
                        }
                    }
                    // Refreshes the open screens and the sync status in Settings.
                    let _ = app.emit("data-changed", ());
                }
            } else {
                last_sync = None;
            }
            thread::park_timeout(wait);
        }
    })
    .thread()
    .clone()
}
