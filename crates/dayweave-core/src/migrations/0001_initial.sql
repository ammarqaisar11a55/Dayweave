-- Dates and times written as text are local wall-clock values ('2026-11-06', '17:00').
-- Columns ending in _at hold UTC RFC 3339 instants. See ARCHITECTURE.md.

CREATE TABLE projects (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    color       TEXT NOT NULL DEFAULT 'orange',
    icon        TEXT NOT NULL DEFAULT 'folder',
    start_date  TEXT,
    deadline    TEXT,
    archived_at TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE habits (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL CHECK (length(trim(name)) > 0),
    description     TEXT NOT NULL DEFAULT '',
    icon            TEXT NOT NULL DEFAULT 'sparkles',
    color           TEXT NOT NULL DEFAULT 'orange',
    kind            TEXT NOT NULL DEFAULT 'build'
                    CHECK (kind IN ('build', 'quit', 'quantity', 'checklist')),
    frequency       TEXT NOT NULL DEFAULT 'daily'
                    CHECK (frequency IN ('daily', 'weekdays', 'weekly_target')),
    weekly_target   INTEGER CHECK (weekly_target BETWEEN 1 AND 7),
    start_date      TEXT NOT NULL,
    reminder_time   TEXT,
    quantity_target REAL CHECK (quantity_target > 0),
    unit            TEXT NOT NULL DEFAULT '',
    position        INTEGER NOT NULL DEFAULT 0,
    archived_at     TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- weekday: 0 is Monday, 6 is Sunday.
CREATE TABLE habit_schedules (
    habit_id INTEGER NOT NULL REFERENCES habits (id) ON DELETE CASCADE,
    weekday  INTEGER NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    PRIMARY KEY (habit_id, weekday)
) WITHOUT ROWID;

CREATE TABLE habit_checklist_items (
    id       INTEGER PRIMARY KEY,
    habit_id INTEGER NOT NULL REFERENCES habits (id) ON DELETE CASCADE,
    title    TEXT NOT NULL CHECK (length(trim(title)) > 0),
    position INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX habit_checklist_items_habit ON habit_checklist_items (habit_id);

-- status: 'done' met the goal, 'partial' made progress, 'slip' broke a quit habit.
CREATE TABLE habit_completions (
    id            INTEGER PRIMARY KEY,
    habit_id      INTEGER NOT NULL REFERENCES habits (id) ON DELETE CASCADE,
    date          TEXT NOT NULL,
    status        TEXT NOT NULL CHECK (status IN ('done', 'partial', 'slip')),
    value         REAL,
    checked_items TEXT NOT NULL DEFAULT '[]',
    note          TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    UNIQUE (habit_id, date)
);
CREATE INDEX habit_completions_date ON habit_completions (date);

-- Shared by every occurrence in a series. weekdays is a bitmask where bit 0 is Monday.
-- anchor_day keeps a monthly rule on the 31st after it was clamped in a shorter month.
CREATE TABLE task_recurrence (
    id            INTEGER PRIMARY KEY,
    frequency     TEXT NOT NULL CHECK (frequency IN ('daily', 'weekly', 'monthly', 'yearly')),
    interval      INTEGER NOT NULL DEFAULT 1 CHECK (interval >= 1),
    weekdays      INTEGER NOT NULL DEFAULT 0 CHECK (weekdays BETWEEN 0 AND 127),
    anchor_day    INTEGER CHECK (anchor_day BETWEEN 1 AND 31),
    end_date      TEXT,
    max_count     INTEGER CHECK (max_count >= 1),
    created_count INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE tasks (
    id                    INTEGER PRIMARY KEY,
    title                 TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description           TEXT NOT NULL DEFAULT '',
    status                TEXT NOT NULL DEFAULT 'todo'
                          CHECK (status IN ('todo', 'in_progress', 'completed', 'archived')),
    status_before_archive TEXT,
    priority              TEXT NOT NULL DEFAULT 'medium'
                          CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    category              TEXT,
    project_id            INTEGER REFERENCES projects (id) ON DELETE SET NULL,
    habit_id              INTEGER REFERENCES habits (id) ON DELETE SET NULL,
    recurrence_id         INTEGER REFERENCES task_recurrence (id) ON DELETE SET NULL,
    spawned_next          INTEGER NOT NULL DEFAULT 0,
    start_date            TEXT,
    due_date              TEXT,
    due_time              TEXT,
    estimated_minutes     INTEGER CHECK (estimated_minutes >= 0),
    created_at            TEXT NOT NULL,
    updated_at            TEXT NOT NULL,
    completed_at          TEXT,
    completed_on          TEXT,
    archived_at           TEXT,
    CHECK (due_time IS NULL OR due_date IS NOT NULL)
);
CREATE INDEX tasks_status_due ON tasks (status, due_date);
CREATE INDEX tasks_project ON tasks (project_id);
CREATE INDEX tasks_completed_on ON tasks (completed_on);

CREATE TABLE task_subtasks (
    id           INTEGER PRIMARY KEY,
    task_id      INTEGER NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    parent_id    INTEGER REFERENCES task_subtasks (id) ON DELETE CASCADE,
    title        TEXT NOT NULL CHECK (length(trim(title)) > 0),
    position     INTEGER NOT NULL DEFAULT 0,
    completed_at TEXT
);
CREATE INDEX task_subtasks_task ON task_subtasks (task_id);

-- kind 'before' fires offset_minutes ahead of the deadline, and 'at' fires at remind_at.
-- snoozed rows come from a notification's Snooze button and are removed once they fire.
CREATE TABLE task_reminders (
    id             INTEGER PRIMARY KEY,
    task_id        INTEGER NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    kind           TEXT NOT NULL CHECK (kind IN ('before', 'at')),
    offset_minutes INTEGER CHECK (offset_minutes >= 0),
    remind_at      TEXT,
    snoozed        INTEGER NOT NULL DEFAULT 0,
    CHECK ((kind = 'before' AND offset_minutes IS NOT NULL) OR (kind = 'at' AND remind_at IS NOT NULL))
);
CREATE INDEX task_reminders_task ON task_reminders (task_id);

CREATE TABLE tags (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (length(trim(name)) > 0)
);

CREATE TABLE task_tags (
    task_id INTEGER NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, tag_id)
) WITHOUT ROWID;
CREATE INDEX task_tags_tag ON task_tags (tag_id);

-- elapsed_seconds holds time up to the last pause. While running, the time since resumed_at
-- is added to it, so the countdown does not depend on the webview being awake.
CREATE TABLE focus_sessions (
    id              INTEGER PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('focus', 'short_break', 'long_break')),
    status          TEXT NOT NULL CHECK (status IN ('running', 'paused', 'completed', 'stopped')),
    planned_seconds INTEGER NOT NULL CHECK (planned_seconds > 0),
    elapsed_seconds INTEGER NOT NULL DEFAULT 0,
    task_id         INTEGER REFERENCES tasks (id) ON DELETE SET NULL,
    habit_id        INTEGER REFERENCES habits (id) ON DELETE SET NULL,
    local_date      TEXT NOT NULL,
    started_at      TEXT NOT NULL,
    resumed_at      TEXT,
    ended_at        TEXT
);
CREATE INDEX focus_sessions_date ON focus_sessions (local_date);
CREATE UNIQUE INDEX focus_sessions_one_active ON focus_sessions ((1))
    WHERE status IN ('running', 'paused');

CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

-- The key identifies a reminder and the deadline it was computed from. Being the primary key
-- is what makes duplicate notifications impossible. See ARCHITECTURE.md.
CREATE TABLE notification_log (
    key        TEXT PRIMARY KEY,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL,
    body       TEXT NOT NULL DEFAULT '',
    task_id    INTEGER,
    habit_id   INTEGER,
    created_at TEXT NOT NULL,
    read_at    TEXT,
    cleared_at TEXT
) WITHOUT ROWID;
CREATE INDEX notification_log_created ON notification_log (created_at);
