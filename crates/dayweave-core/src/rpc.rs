//! The single entry point the interface calls. Everything crosses this boundary as JSON, so the
//! integration tests and the development bridge exercise the same path the desktop application does.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::{
    analytics, backup, calendar, focus, habits, overview, projects, reminders, search, settings, tasks,
};
use crate::{db, Core};

fn input<T: DeserializeOwned>(params: Value) -> Result<T> {
    serde_json::from_value(params).map_err(|error| Error::invalid(format!("Invalid request: {error}")))
}

fn output<T: Serialize>(value: T) -> Result<Value> {
    Ok(serde_json::to_value(value)?)
}

#[derive(Deserialize)]
struct Id {
    id: i64,
}

#[derive(Deserialize)]
struct FilePath {
    path: PathBuf,
}

impl Core {
    pub fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        let guard = self.lock();
        let conn: &rusqlite::Connection = &guard;
        let clock = self.clock.as_ref();
        let id = |params: Value| input::<Id>(params).map(|value| value.id);
        let path = |params: Value| input::<FilePath>(params).map(|value| value.path);
        let done = || Ok(Value::Null);

        match method {
            "overview.get" => output(overview::get(conn, clock)?),
            "overview.badges" => output(overview::badges(conn, clock)?),

            "tasks.list" => output(tasks::list(conn, clock, input(params)?)?),
            "tasks.counts" => output(tasks::counts(conn, clock)?),
            "tasks.get" => output(tasks::get(conn, clock, id(params)?)?),
            "tasks.create" => output(tasks::create(conn, clock, input(params)?)?),
            "tasks.update" => output(tasks::update(conn, clock, input(params)?)?),
            "tasks.complete" => output(tasks::complete(conn, clock, id(params)?)?),
            "tasks.reopen" => output(tasks::reopen(conn, clock, id(params)?)?),
            "tasks.archive" => output(tasks::archive(conn, clock, id(params)?)?),
            "tasks.restore" => output(tasks::restore(conn, clock, id(params)?)?),
            "tasks.duplicate" => output(tasks::duplicate(conn, clock, id(params)?)?),
            "tasks.reschedule" => output(tasks::reschedule(conn, clock, input(params)?)?),
            "tasks.snooze" => {
                tasks::snooze(conn, clock, input(params)?)?;
                done()
            }
            "tasks.delete" => {
                tasks::delete(conn, id(params)?)?;
                done()
            }
            "tasks.categories" => output(tasks::categories(conn)?),
            "tasks.tags" => output(tasks::tags(conn)?),
            "subtasks.add" => output(tasks::subtask_add(conn, clock, input(params)?)?),
            "subtasks.toggle" => output(tasks::subtask_toggle(conn, clock, id(params)?)?),
            "subtasks.rename" => output(tasks::subtask_rename(conn, clock, input(params)?)?),
            "subtasks.delete" => output(tasks::subtask_delete(conn, clock, id(params)?)?),

            "projects.list" => output(projects::list(conn, clock, input(params)?)?),
            "projects.get" => output(projects::get(conn, clock, id(params)?)?),
            "projects.create" => output(projects::create(conn, clock, input(params)?)?),
            "projects.update" => output(projects::update(conn, clock, input(params)?)?),
            "projects.archive" => output(projects::set_archived(conn, clock, id(params)?, true)?),
            "projects.restore" => output(projects::set_archived(conn, clock, id(params)?, false)?),
            "projects.delete" => {
                projects::delete(conn, id(params)?)?;
                done()
            }

            "habits.list" => output(habits::list(conn, clock, input(params)?)?),
            "habits.get" => output(habits::get(conn, clock, id(params)?)?),
            "habits.create" => output(habits::create(conn, clock, input(params)?)?),
            "habits.update" => output(habits::update(conn, clock, input(params)?)?),
            "habits.log" => output(habits::log(conn, clock, input(params)?)?),
            "habits.undo" => output(habits::undo(conn, clock, input(params)?)?),
            "habits.archive" => output(habits::set_archived(conn, clock, id(params)?, true)?),
            "habits.restore" => output(habits::set_archived(conn, clock, id(params)?, false)?),
            "habits.history" => output(habits::history(conn, clock, input(params)?)?),
            "habits.heatmap" => output(habits::heatmap(conn, clock, input(params)?)?),
            "habits.reorder" => {
                habits::reorder(conn, input(params)?)?;
                done()
            }
            "habits.delete" => {
                habits::delete(conn, id(params)?)?;
                done()
            }

            "focus.current" => output(focus::current(conn, clock)?),
            "focus.start" => output(focus::start(conn, clock, input(params)?)?),
            "focus.pause" => output(focus::pause(conn, clock)?),
            "focus.resume" => output(focus::resume(conn, clock)?),
            "focus.stop" => output(focus::stop(conn, clock)?),
            "focus.complete" => output(focus::complete(conn, clock)?),
            "focus.history" => output(focus::history(conn, clock, input(params)?)?),
            "focus.stats" => output(focus::stats(conn, clock)?),
            "focus.delete" => {
                focus::delete(conn, id(params)?)?;
                done()
            }

            "calendar.range" => output(calendar::range(conn, clock, input(params)?)?),
            "analytics.get" => output(analytics::get(conn, clock, input(params)?)?),
            "search.query" => output(search::query(conn, clock, input(params)?)?),

            "settings.get" => settings::all(conn),
            "settings.set" => settings::set(conn, input(params)?),

            "notifications.list" => output(reminders::log(conn, input(params)?)?),
            "notifications.markRead" => {
                reminders::mark_all_read(conn, clock)?;
                done()
            }
            "notifications.clear" => {
                reminders::clear(conn, clock)?;
                done()
            }

            "backup.exportJson" => {
                backup::export_json(conn, clock, &path(params)?)?;
                done()
            }
            "backup.exportDatabase" => {
                backup::export_database(conn, &path(params)?)?;
                done()
            }
            "backup.exportTasksCsv" => {
                backup::export_tasks_csv(conn, &path(params)?)?;
                done()
            }
            "backup.exportHabitsCsv" => {
                backup::export_habits_csv(conn, &path(params)?)?;
                done()
            }
            "backup.preview" => output(backup::preview(conn, clock, &path(params)?)?),
            "backup.restore" => {
                let safety = backup::restore(conn, clock, self.data_dir.as_deref(), &path(params)?)?;
                output(json!({ "safetyCopy": safety }))
            }

            "app.info" => output(json!({
                "version": env!("CARGO_PKG_VERSION"),
                "schemaVersion": db::SCHEMA_VERSION,
                "dataDir": self.data_dir,
            })),

            other => Err(Error::UnknownMethod(other.to_string())),
        }
    }
}
