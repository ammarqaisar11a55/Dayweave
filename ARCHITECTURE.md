# Architecture

Dayweave is a local-first productivity application for Ubuntu: tasks, deadlines, habits, a calendar,
focus sessions and analytics, all stored in one SQLite file on your computer.

## Overview

```
┌────────────────────────── WebKitGTK webview ──────────────────────────┐
│  React pages  →  shared components  →  lib/api.ts  rpc(method, params) │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ one Tauri command: `rpc`
┌───────────────────────────────┴─ src-tauri (desktop shell) ───────────┐
│  window, tray, single instance, autostart, notifications, scheduler   │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ Core::rpc(method, params)
┌───────────────────────────────┴─ crates/dayweave-core ────────────────┐
│  dispatch → tasks, habits, projects, focus, calendar, analytics, ...  │
│  pure engines: streaks, recurrence, reminders                         │
│  db: migrations, transactions                       (no GTK, no Tauri) │
└───────────────────────────────┬───────────────────────────────────────┘
                                │ rusqlite (bundled SQLite)
                     $XDG_DATA_HOME/io.github.ammarqaisar11a55.dayweave/dayweave.db
```

| Path | Holds |
| --- | --- |
| `crates/dayweave-core` | All product logic and storage. Builds and tests without a desktop |
| `src-tauri` | The thin desktop shell. Everything that needs a window or a session bus |
| `src` | The React interface |
| `snap`, `.github` | Packaging and CI |

## Technical decisions

### The core is a separate crate with no Tauri dependency

Streaks, recurrence, reminders and restores are where this application can quietly lose someone's
data or trust. Keeping them in a crate that compiles without WebKit means their tests run in seconds
on any machine, and the scheduler can use them directly without going through the webview.

### One `rpc` command instead of a Tauri command per operation

The interface makes about 70 distinct calls. They all pass through `Core::rpc(method, params)`, which
takes and returns JSON. This has three benefits:

- Adding an operation is one match arm rather than a command, a registration and a capability.
- Integration tests exercise the same entry point the interface uses.
- `dayweave-bridge`, a development-only binary, serves the same dispatcher over HTTP on localhost.
  The interface can then be driven in an ordinary browser against the real Rust code and a real
  database, with no mock backend. It is behind a Cargo feature and is never packaged.

### SQLite through rusqlite, not SQLx

Access is local and synchronous, and there is one writer. SQLx would add an async runtime and a
connection pool to solve problems this application does not have. `rusqlite` with the `bundled`
feature also fixes the SQLite version, so behaviour does not depend on what the snap base ships.

One connection sits behind a mutex. Queries take well under a millisecond at this data size, so
a pool would add complexity without measurable benefit.

### Time: floating local dates, resolved at the last moment

This is the decision the rest of the scheduling depends on.

| Kind of value | Stored as | Why |
| --- | --- |--- |
| A deadline or schedule ("Friday", "Friday 17:00", "every day at 09:00") | Local wall-clock text: `2026-11-06`, `17:00` | It means the same thing to you after a clock change or a trip |
| An event that happened (created, completed, session started) | UTC RFC 3339 | It is a fixed moment |
| The day an event counts towards | Local date, recorded when it happens | History must not move if your timezone later changes |

Wall-clock values become instants only when needed, through the `Clock` trait. That is where
daylight saving is handled: a time skipped by the change moves forward to the first valid moment,
and a repeated time uses the earlier one. Recurring tasks stay at 09:00 across a change. Tests run
the conversion against real zones with `chrono-tz`, which is only a development dependency.

The alternative was storing an instant plus an IANA zone. It would ship a timezone database and
require a decision about what "every day at 09:00" means after travelling, for no benefit to a
single-computer application.

Date-only deadlines stay date-only. They become overdue when the local date passes them, and the
interface never shows an hour countdown for them. A reminder on a date-only deadline fires at a
configurable time of day, 09:00 by default.

### Reminders: a ledger, not a queue of timers

No scheduled timers are stored. Every 20 seconds the scheduler asks the core which reminders became
due since the last check, computed from the tasks as they are now. Delivered notices are recorded in
`notification_log` under a key built from the reminder and the deadline it was computed from.

This produces the required behaviour without any cancellation code:

- **Rescheduling** changes the key, so the reminder fires again for the new deadline.
- **Completing a task** removes it from the query, so its reminders stop.
- **Duplicates** cannot happen, because the key is the primary key.
- **Suspend, restart and clock changes** are covered, because the last check time is stored. Anything
  missed is delivered on the next check, grouped into one notice if there are several.

The same ledger feeds the notification center in the toolbar.

### Background behaviour, and what is promised

Dayweave delivers reminders while its process is running, and closing the window does not stop that
process. It stays in the tray, and launching it again reopens the window. **Once you choose Quit, no
reminders are delivered until the next launch**, and the interface says so. Reminders missed in that
time are delivered at the next launch.

For reminders from the moment you sign in, Settings can add an autostart entry that launches Dayweave
in the background. A separate daemon was rejected: snap services run as root unless user daemons are
used, a second process would need its own access to the database, and it would add a failure mode to
solve something autostart already solves.

### The snap has neither `network` nor `home`

Dayweave needs neither. The GNOME extension sets `GTK_USE_PORTAL=1`, so the file dialog runs outside
the sandbox and grants access to only the file you choose. A portal grant covers that one file, so
nothing can be created beside it. Exports fall back to writing it directly, and a database backup
is read from a private copy because SQLite may need side files.

### Single instance through a Unix socket

`tauri-plugin-single-instance` claims a D-Bus name, which strict confinement denies unless the snap
declares a slot, and a failure there would leave two instances running two schedulers. Instead the
shell binds a socket in `$XDG_RUNTIME_DIR`, which behaves identically inside and outside the snap. The
same socket carries the launcher actions, so `dayweave --new-task` reaches the running instance.

### Focus timer state lives in the core

A hidden webview throttles JavaScript timers, so the interface cannot own the countdown. The session
row stores the elapsed time and when it last resumed. The interface renders from those values, and
the scheduler ends the session and sends the notification on time whether or not the window is open.

### Recurring tasks

Each occurrence is its own row in a series sharing a `task_recurrence` record. Completing one
generates the next from the anchor date, so completion history is preserved as ordinary rows. A
`spawned_next` flag prevents a second copy if a task is reopened and completed again. Monthly rules
keep their original day, so a task on the 31st moves to the 28th in February and returns to the 31st
in March. A task completed late skips to the first occurrence after today rather than generating a
backlog.

### Streaks

`streaks.rs` is a pure function of a habit's definition, its history and today's date.

- Only scheduled days count. An unscheduled day neither extends nor breaks a streak.
- Today is pending until it ends, so an unfinished day never breaks a streak.
- Habits with a weekly target are measured in weeks, and the first day of the week is a setting.
- Quit habits succeed by default, and a logged slip breaks them.
- Quantity and checklist habits count only once they are fully met.

### Backup and restore

A restore never touches the live database until it is known to be good:

1. The file is loaded into a temporary in-memory database at its own schema version.
2. It is migrated to the current version, and foreign keys and integrity are checked.
3. The interface shows what the file contains and what will be replaced, and asks for confirmation.
4. A safety copy of the current database is written.
5. All tables are replaced in one transaction.

JSON is the portable format. A full backup is a consistent SQLite snapshot from `VACUUM INTO`, and
restoring one goes through the same steps. A copy of the database is also taken before any migration,
and a database from a newer version of Dayweave is refused rather than opened.

### Interface data flow

There is no client state library. `useRpc` reads data, and every successful write increments a global
version that makes mounted queries refetch. This is cheap against a local database and removes a whole
class of stale-cache bugs. Changes from outside the interface, such as a notification's Complete
button, arrive as a Tauri event that increments the same version.

### Native window decorations

Dayweave uses the system's title bar rather than drawing its own. It is a Tauri application, not a
GTK or Libadwaita one, and it does not imitate Adwaita widgets or claim to be native. Theme changes
are passed to the window, so the title bar matches the theme chosen in the application.

### Dependencies

Kept to what earns its place. On the Rust side: `tauri`, `rusqlite`, `chrono`, `serde`, `notify-rust`
and `tauri-plugin-dialog`. On the web side: `react`, `lucide-react`, `recharts`, Tailwind and a bundled
copy of Inter. There is no router, date library, component kit or state library.

The application makes no network requests. The content security policy allows only the Tauri IPC
channel, and the snap does not declare the `network` plug.

## Data model

```
projects ─┐
          ├─< tasks >── task_tags ──> tags
habits ───┘    │ │ └─< task_subtasks (self-nesting)
   │           │ └──< task_reminders
   │           └───── task_recurrence
   ├─< habit_schedules
   ├─< habit_checklist_items
   ├─< habit_completions
   └─< focus_sessions >── tasks

app_settings        notification_log
```

The schema is in `crates/dayweave-core/src/migrations`. Each migration runs in its own transaction
and is numbered through `PRAGMA user_version`.

## Security

The webview can reach only the `rpc` command and the capabilities listed in
`src-tauri/capabilities/default.json`. All SQL uses bound parameters. See [PRIVACY.md](PRIVACY.md).
