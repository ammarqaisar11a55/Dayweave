//! Storage and product logic for Dayweave. Nothing here depends on Tauri or GTK, so all of it can be
//! built and tested without a desktop.

pub mod analytics;
pub mod backup;
pub mod calendar;
pub mod clock;
pub mod db;
pub mod error;
pub mod focus;
pub mod habits;
pub mod overview;
pub mod projects;
pub mod recurrence;
pub mod reminders;
pub mod rpc;
pub mod search;
pub mod settings;
pub mod streaks;
pub mod tasks;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use clock::{Clock, SystemClock};
use error::Result;
use reminders::Notice;

pub use error::Error;

pub const DATABASE_FILE: &str = "dayweave.db";

/// The application's state. One connection sits behind a mutex: queries are local and take well under
/// a millisecond, so a pool would add nothing.
pub struct Core {
    conn: Mutex<Connection>,
    clock: Arc<dyn Clock>,
    data_dir: Option<PathBuf>,
}

impl Core {
    pub fn open(data_dir: &Path) -> Result<Self> {
        Self::open_with_clock(data_dir, Arc::new(SystemClock))
    }

    pub fn open_with_clock(data_dir: &Path, clock: Arc<dyn Clock>) -> Result<Self> {
        let conn = db::open(&data_dir.join(DATABASE_FILE))?;
        Ok(Core { conn: Mutex::new(conn), clock, data_dir: Some(data_dir.to_path_buf()) })
    }

    pub fn in_memory(clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Core { conn: Mutex::new(db::open_in_memory()?), clock, data_dir: None })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        // A panic mid-request leaves no partial state, because writes run in transactions.
        self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// One scheduler pass: finds what has fallen due and records it. Returns what to show on the desktop.
    pub fn tick(&self) -> Result<Vec<Notice>> {
        reminders::tick(&self.lock(), self.clock.as_ref())
    }

    pub fn setting_bool(&self, key: &str) -> Result<bool> {
        settings::get_bool(&self.lock(), key)
    }
}
