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
pub mod sync;
pub mod tasks;
pub mod transport;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use clock::{Clock, SystemClock};
use error::Result;
use reminders::Notice;
use transport::{Transport, TransportError};

pub use error::Error;

pub const DATABASE_FILE: &str = "dayweave.db";

/// The application's state. One connection sits behind a mutex: queries are local and take well under
/// a millisecond, so a pool would add nothing.
pub struct Core {
    conn: Mutex<Connection>,
    clock: Arc<dyn Clock>,
    data_dir: Option<PathBuf>,
    transport: Option<Arc<dyn Transport>>,
    /// One sync at a time, without holding the database while the network is slow.
    syncing: Mutex<()>,
}

fn default_transport() -> Option<Arc<dyn Transport>> {
    #[cfg(feature = "http")]
    return Some(Arc::new(transport::HttpTransport::default()));
    #[cfg(not(feature = "http"))]
    None
}

impl Core {
    pub fn open(data_dir: &Path) -> Result<Self> {
        Self::open_with_clock(data_dir, Arc::new(SystemClock))
    }

    pub fn open_with_clock(data_dir: &Path, clock: Arc<dyn Clock>) -> Result<Self> {
        let conn = db::open(&data_dir.join(DATABASE_FILE))?;
        Ok(Core {
            conn: Mutex::new(conn),
            clock,
            data_dir: Some(data_dir.to_path_buf()),
            transport: default_transport(),
            syncing: Mutex::new(()),
        })
    }

    pub fn in_memory(clock: Arc<dyn Clock>) -> Result<Self> {
        Ok(Core {
            conn: Mutex::new(db::open_in_memory()?),
            clock,
            data_dir: None,
            transport: default_transport(),
            syncing: Mutex::new(()),
        })
    }

    /// Replaces how the core reaches a sync server, as tests do with an in-memory one.
    pub fn with_transport(mut self, transport: Arc<dyn Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        // A panic mid-request leaves no partial state, because writes run in transactions.
        self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// One scheduler pass: finds what has fallen due and records it. Returns what to show on the desktop.
    pub fn tick(&self) -> Result<Vec<Notice>> {
        reminders::tick(&self.lock(), self.clock.as_ref())
    }

    /// Seconds until the running focus session ends, so the scheduler can wake exactly then.
    pub fn focus_remaining(&self) -> Result<Option<i64>> {
        let session = focus::current(&self.lock(), self.clock.as_ref())?;
        Ok(session.filter(|session| session.status == "running").map(|session| session.remaining_seconds))
    }

    pub fn setting_bool(&self, key: &str) -> Result<bool> {
        settings::get_bool(&self.lock(), key)
    }

    /// Whether a sync account is set up, so a background loop knows whether to sync at all.
    pub fn sync_enabled(&self) -> Result<bool> {
        Ok(sync::account(&self.lock())?.is_some())
    }

    /// How many records are waiting to be sent.
    pub fn sync_pending(&self) -> Result<i64> {
        sync::pending_count(&self.lock())
    }

    fn transport(&self) -> Result<Arc<dyn Transport>> {
        self.transport.clone().ok_or_else(|| Error::invalid("This build of Dayweave can't sync"))
    }

    fn post(&self, url: String, token: Option<&str>, body: Value) -> Result<Value> {
        self.transport()?.post(&url, token, &body).map_err(|error| match error {
            TransportError::Unreachable(_) => Error::Offline(
                "Couldn't reach the sync server. Your changes are saved here and will sync when you're online".into(),
            ),
            // This device's token was revoked or has expired. Stop trying until the person signs in.
            TransportError::Rejected { status: 401, .. } if token.is_some() => {
                let _ = self.lock().execute("DELETE FROM app_settings WHERE key = ?1", [sync::TOKEN]);
                Error::invalid("This device was signed out of sync. Sign in again to keep syncing")
            }
            TransportError::Rejected { message, .. } => Error::invalid(message),
        })
    }

    /// Creates an account or signs in to one, then sends everything on this device to it.
    fn sign_in(&self, params: SignIn, create: bool) -> Result<Value> {
        let server = sync::normalize_server(&params.server)?;
        let email = params.email.trim().to_lowercase();
        if !email.contains('@') {
            return Err(Error::invalid("Enter your email address"));
        }
        if create && params.password.chars().count() < 8 {
            return Err(Error::invalid("Choose a password of at least 8 characters"));
        }
        let path = if create { "signup" } else { "login" };
        let reply = self.post(
            format!("{server}/auth/{path}"),
            None,
            json!({ "email": email, "password": params.password, "device": params.device }),
        )?;
        let token = reply["token"].as_str().ok_or_else(|| Error::invalid("The server didn't sign you in"))?;
        {
            let conn = self.lock();
            let tx = conn.unchecked_transaction()?;
            sync::forget_account(&tx)?;
            settings::set_internal(&tx, sync::SERVER, &server)?;
            settings::set_internal(&tx, sync::TOKEN, token)?;
            settings::set_internal(&tx, sync::EMAIL, reply["email"].as_str().unwrap_or(&email))?;
            sync::mark_all_dirty(&tx)?;
            tx.commit()?;
        }
        // Signing in worked, so the server is reachable. A failed first sync is retried later.
        let _ = self.sync_now();
        self.sync_status()
    }

    /// Stops syncing on this device. Its data stays, and the server keeps the account's copy.
    fn sign_out(&self) -> Result<Value> {
        let _running = self.syncing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let account = sync::account(&self.lock())?;
        if let Some(account) = account {
            // Revoking this device's token is a courtesy. Signing out works offline too.
            let _ = self.post(format!("{}/auth/logout", account.server), Some(&account.token), json!({}));
        }
        sync::forget_account(&self.lock())?;
        self.sync_status()
    }

    fn sync_status(&self) -> Result<Value> {
        Ok(serde_json::to_value(sync::status(&self.lock())?)?)
    }

    /// Sends waiting changes and fetches everyone else's, in rounds until both sides are current.
    pub fn sync_now(&self) -> Result<SyncReport> {
        const PUSH: usize = 200;
        const PULL: usize = 500;
        let _running = self.syncing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let (account, mut cursor) = {
            let conn = self.lock();
            let account = sync::account(&conn)?.ok_or_else(|| Error::invalid("Sign in to sync first"))?;
            (account, sync::cursor(&conn)?)
        };
        let mut report = SyncReport::default();
        for _round in 0..100 {
            let batch = sync::pending(&self.lock(), PUSH)?;
            let reply = match self.post(
                format!("{}/sync", account.server),
                Some(&account.token),
                json!({ "cursor": cursor, "changes": batch, "limit": PULL }),
            ) {
                Ok(reply) => reply,
                Err(error) => return Err(self.sync_failed(error)),
            };
            let reply: SyncReply = serde_json::from_value(reply)
                .map_err(|_| self.sync_failed(Error::invalid("The server's reply couldn't be read")))?;
            let conn = self.lock();
            sync::mark_sent(&conn, &batch)?;
            let applied = sync::apply(&conn, &reply.changes)?;
            cursor = reply.cursor;
            settings::set_internal(&conn, sync::CURSOR, &cursor.to_string())?;
            report.sent += batch.len();
            report.received += applied.applied;
            report.kept_local += applied.kept_local;
            report.skipped += applied.skipped;
            if batch.len() < PUSH && !reply.more {
                break;
            }
        }
        let conn = self.lock();
        settings::set_internal(&conn, sync::LAST_SYNC, &clock::format_instant(self.clock.now()))?;
        conn.execute("DELETE FROM app_settings WHERE key = ?1", [sync::LAST_ERROR])?;
        Ok(report)
    }

    fn sync_failed(&self, error: Error) -> Error {
        let _ = settings::set_internal(&self.lock(), sync::LAST_ERROR, &error.to_string());
        error
    }

    /// Sync operations, which talk to the network and so run without holding the database.
    pub(crate) fn sync_rpc(&self, method: &str, params: Value) -> Option<Result<Value>> {
        let parse = |params: Value| {
            serde_json::from_value::<SignIn>(params)
                .map_err(|error| Error::invalid(format!("Invalid request: {error}")))
        };
        Some(match method {
            "sync.status" => self.sync_status(),
            "sync.signUp" => parse(params).and_then(|params| self.sign_in(params, true)),
            "sync.signIn" => parse(params).and_then(|params| self.sign_in(params, false)),
            "sync.signOut" => self.sign_out(),
            "sync.run" => self.sync_now().and_then(|report| Ok(serde_json::to_value(report)?)),
            _ => return None,
        })
    }
}

#[derive(Deserialize)]
struct SignIn {
    server: String,
    email: String,
    password: String,
    #[serde(default)]
    device: String,
}

#[derive(Deserialize)]
struct SyncReply {
    cursor: i64,
    #[serde(default)]
    changes: Vec<sync::Change>,
    #[serde(default)]
    more: bool,
}

/// What a sync did. `received` above zero means the screens should reload.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub sent: usize,
    pub received: usize,
    pub kept_local: usize,
    pub skipped: usize,
}
