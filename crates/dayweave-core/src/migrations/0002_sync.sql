-- Sync. Data is grouped into records that travel as one piece: a task with its subtasks, reminders
-- and tags; a habit with its schedule and checklist; a check-in; a project; a repeat rule; a finished
-- focus session; a preference. See ARCHITECTURE.md.
--
-- Triggers keep these tables current, so no feature has to remember to. Every change marks its
-- record dirty with the time it changed, and a sync sends dirty records and deletions. Changes are
-- tracked whether or not sync is on, so turning it on later sends everything.

-- Each record's stable ID, shared by every device. local_id is the row's own ID as text, or the key
-- for a preference. A check-in's sync_id is its habit's sync_id and its date, so two devices that log
-- the same habit on the same day share one record. Checklist items have IDs here only so check-ins
-- can name them; they travel inside their habit.
CREATE TABLE sync_records (
    entity      TEXT NOT NULL,
    local_id    TEXT NOT NULL,
    sync_id     TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    dirty       INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (entity, local_id)
) WITHOUT ROWID;
CREATE UNIQUE INDEX sync_records_sync_id ON sync_records (entity, sync_id);
CREATE INDEX sync_records_dirty ON sync_records (dirty) WHERE dirty = 1;

-- Deleted records not yet sent.
CREATE TABLE sync_tombstones (
    entity     TEXT NOT NULL,
    sync_id    TEXT NOT NULL,
    deleted_at TEXT NOT NULL,
    PRIMARY KEY (entity, sync_id)
) WITHOUT ROWID;

-- Set to 1 while changes from the server are written, so they aren't sent straight back.
CREATE TABLE sync_guard (applying INTEGER NOT NULL);
INSERT INTO sync_guard (applying) VALUES (0);

-- Projects

CREATE TRIGGER sync_projects_write AFTER INSERT ON projects
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('project', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_projects_update AFTER UPDATE ON projects
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('project', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_projects_delete AFTER DELETE ON projects
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT OR REPLACE INTO sync_tombstones (entity, sync_id, deleted_at)
    SELECT entity, sync_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sync_records
    WHERE entity = 'project' AND local_id = CAST(OLD.id AS TEXT);
    DELETE FROM sync_records WHERE entity = 'project' AND local_id = CAST(OLD.id AS TEXT);
END;

-- Habits, with their schedules and checklists

CREATE TRIGGER sync_habits_insert AFTER INSERT ON habits
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('habit', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_habits_update AFTER UPDATE ON habits
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('habit', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_habits_delete AFTER DELETE ON habits
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT OR REPLACE INTO sync_tombstones (entity, sync_id, deleted_at)
    SELECT entity, sync_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sync_records
    WHERE entity = 'habit' AND local_id = CAST(OLD.id AS TEXT);
    DELETE FROM sync_records WHERE entity = 'habit' AND local_id = CAST(OLD.id AS TEXT);
END;

CREATE TRIGGER sync_habit_schedules_insert AFTER INSERT ON habit_schedules
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'habit' AND local_id = CAST(NEW.habit_id AS TEXT);
END;

CREATE TRIGGER sync_habit_schedules_delete AFTER DELETE ON habit_schedules
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'habit' AND local_id = CAST(OLD.habit_id AS TEXT);
END;

CREATE TRIGGER sync_checklist_insert AFTER INSERT ON habit_checklist_items
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('checklist_item', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0)
    ON CONFLICT (entity, local_id) DO NOTHING;
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'habit' AND local_id = CAST(NEW.habit_id AS TEXT);
END;

CREATE TRIGGER sync_checklist_update AFTER UPDATE ON habit_checklist_items
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'habit' AND local_id = CAST(NEW.habit_id AS TEXT);
END;

CREATE TRIGGER sync_checklist_delete AFTER DELETE ON habit_checklist_items
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    DELETE FROM sync_records WHERE entity = 'checklist_item' AND local_id = CAST(OLD.id AS TEXT);
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'habit' AND local_id = CAST(OLD.habit_id AS TEXT);
END;

-- Check-ins

CREATE TRIGGER sync_completions_insert AFTER INSERT ON habit_completions
WHEN (SELECT applying FROM sync_guard) = 0
 AND EXISTS (SELECT 1 FROM sync_records WHERE entity = 'habit' AND local_id = CAST(NEW.habit_id AS TEXT))
BEGIN
    DELETE FROM sync_tombstones WHERE entity = 'completion' AND sync_id =
        (SELECT sync_id FROM sync_records WHERE entity = 'habit' AND local_id = CAST(NEW.habit_id AS TEXT))
        || ':' || NEW.date;
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES (
        'completion', CAST(NEW.id AS TEXT),
        (SELECT sync_id FROM sync_records WHERE entity = 'habit' AND local_id = CAST(NEW.habit_id AS TEXT))
            || ':' || NEW.date,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1
    )
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_completions_update AFTER UPDATE ON habit_completions
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'completion' AND local_id = CAST(NEW.id AS TEXT);
END;

-- A check-in removed on its own is a deletion to send. One removed with its habit isn't: the habit's
-- deletion covers it.
CREATE TRIGGER sync_completions_delete AFTER DELETE ON habit_completions
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT OR REPLACE INTO sync_tombstones (entity, sync_id, deleted_at)
    SELECT entity, sync_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sync_records
    WHERE entity = 'completion' AND local_id = CAST(OLD.id AS TEXT)
      AND EXISTS (SELECT 1 FROM habits WHERE id = OLD.habit_id);
    DELETE FROM sync_records WHERE entity = 'completion' AND local_id = CAST(OLD.id AS TEXT);
END;

-- Repeat rules, shared by every task in a series

CREATE TRIGGER sync_recurrence_insert AFTER INSERT ON task_recurrence
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('recurrence', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_recurrence_update AFTER UPDATE ON task_recurrence
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('recurrence', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_recurrence_delete AFTER DELETE ON task_recurrence
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT OR REPLACE INTO sync_tombstones (entity, sync_id, deleted_at)
    SELECT entity, sync_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sync_records
    WHERE entity = 'recurrence' AND local_id = CAST(OLD.id AS TEXT);
    DELETE FROM sync_records WHERE entity = 'recurrence' AND local_id = CAST(OLD.id AS TEXT);
END;

-- Tasks, with their subtasks, reminders and tags

CREATE TRIGGER sync_tasks_insert AFTER INSERT ON tasks
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('task', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_tasks_update AFTER UPDATE ON tasks
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('task', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_tasks_delete AFTER DELETE ON tasks
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT OR REPLACE INTO sync_tombstones (entity, sync_id, deleted_at)
    SELECT entity, sync_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sync_records
    WHERE entity = 'task' AND local_id = CAST(OLD.id AS TEXT);
    DELETE FROM sync_records WHERE entity = 'task' AND local_id = CAST(OLD.id AS TEXT);
END;

CREATE TRIGGER sync_subtasks_insert AFTER INSERT ON task_subtasks
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(NEW.task_id AS TEXT);
END;

CREATE TRIGGER sync_subtasks_update AFTER UPDATE ON task_subtasks
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(NEW.task_id AS TEXT);
END;

CREATE TRIGGER sync_subtasks_delete AFTER DELETE ON task_subtasks
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(OLD.task_id AS TEXT);
END;

CREATE TRIGGER sync_task_reminders_insert AFTER INSERT ON task_reminders
WHEN (SELECT applying FROM sync_guard) = 0 AND NEW.snoozed = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(NEW.task_id AS TEXT);
END;

CREATE TRIGGER sync_task_reminders_delete AFTER DELETE ON task_reminders
WHEN (SELECT applying FROM sync_guard) = 0 AND OLD.snoozed = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(OLD.task_id AS TEXT);
END;

CREATE TRIGGER sync_task_tags_insert AFTER INSERT ON task_tags
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(NEW.task_id AS TEXT);
END;

CREATE TRIGGER sync_task_tags_delete AFTER DELETE ON task_tags
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    UPDATE sync_records SET dirty = 1, modified_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE entity = 'task' AND local_id = CAST(OLD.task_id AS TEXT);
END;

-- Focus sessions travel once they've finished. A running session belongs to the device running it.

CREATE TRIGGER sync_focus_insert AFTER INSERT ON focus_sessions
WHEN (SELECT applying FROM sync_guard) = 0 AND NEW.status IN ('completed', 'stopped')
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('focus', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_focus_update AFTER UPDATE ON focus_sessions
WHEN (SELECT applying FROM sync_guard) = 0 AND NEW.status IN ('completed', 'stopped')
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('focus', CAST(NEW.id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_focus_delete AFTER DELETE ON focus_sessions
WHEN (SELECT applying FROM sync_guard) = 0
BEGIN
    INSERT OR REPLACE INTO sync_tombstones (entity, sync_id, deleted_at)
    SELECT entity, sync_id, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') FROM sync_records
    WHERE entity = 'focus' AND local_id = CAST(OLD.id AS TEXT);
    DELETE FROM sync_records WHERE entity = 'focus' AND local_id = CAST(OLD.id AS TEXT);
END;

-- Preferences. Internal state, and choices that only make sense on one computer, stay put.

CREATE TRIGGER sync_settings_insert AFTER INSERT ON app_settings
WHEN (SELECT applying FROM sync_guard) = 0
 AND NEW.key NOT LIKE 'internal.%'
 AND NEW.key NOT IN ('startAtLogin', 'closeToBackground', 'sidebarCollapsed')
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('setting', NEW.key, NEW.key, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

CREATE TRIGGER sync_settings_update AFTER UPDATE ON app_settings
WHEN (SELECT applying FROM sync_guard) = 0
 AND NEW.key NOT LIKE 'internal.%'
 AND NEW.key NOT IN ('startAtLogin', 'closeToBackground', 'sidebarCollapsed')
BEGIN
    INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
    VALUES ('setting', NEW.key, NEW.key, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1)
    ON CONFLICT (entity, local_id) DO UPDATE SET dirty = 1, modified_at = excluded.modified_at;
END;

-- Existing data gets IDs now and is sent on the first sync.

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'project', CAST(id AS TEXT), lower(hex(randomblob(16))), updated_at, 1 FROM projects;

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'habit', CAST(id AS TEXT), lower(hex(randomblob(16))), updated_at, 1 FROM habits;

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'checklist_item', CAST(id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0
FROM habit_checklist_items;

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'completion', CAST(c.id AS TEXT), h.sync_id || ':' || c.date, c.updated_at, 1
FROM habit_completions c
JOIN sync_records h ON h.entity = 'habit' AND h.local_id = CAST(c.habit_id AS TEXT);

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'recurrence', CAST(id AS TEXT), lower(hex(randomblob(16))), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1
FROM task_recurrence;

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'task', CAST(id AS TEXT), lower(hex(randomblob(16))), updated_at, 1 FROM tasks;

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'focus', CAST(id AS TEXT), lower(hex(randomblob(16))), COALESCE(ended_at, started_at), 1
FROM focus_sessions WHERE status IN ('completed', 'stopped');

INSERT INTO sync_records (entity, local_id, sync_id, modified_at, dirty)
SELECT 'setting', key, key, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 1 FROM app_settings
WHERE key NOT LIKE 'internal.%' AND key NOT IN ('startAtLogin', 'closeToBackground', 'sidebarCollapsed');
