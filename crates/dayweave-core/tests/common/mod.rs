#![allow(dead_code)]

use std::sync::Arc;

use chrono::Duration;
use chrono_tz::{Asia::Karachi, Tz};
use dayweave_core::clock::ManualClock;
use dayweave_core::{Core, Error};
use serde_json::{json, Value};

/// A database in memory and a clock that only moves when told to. 2026-11-02 is a Monday.
pub struct App {
    pub core: Core,
    pub clock: Arc<ManualClock<Tz>>,
}

impl App {
    pub fn new() -> Self {
        Self::at(Karachi, "2026-11-02T10:00")
    }

    pub fn at(tz: Tz, wall: &str) -> Self {
        let clock = Arc::new(ManualClock::new(tz, wall));
        let core = Core::in_memory(clock.clone()).unwrap();
        App { core, clock }
    }

    pub fn call(&self, method: &str, params: Value) -> Value {
        self.core.rpc(method, params).unwrap_or_else(|error| panic!("{method} failed: {error}"))
    }

    pub fn call_err(&self, method: &str, params: Value) -> Error {
        self.core.rpc(method, params).expect_err("the call should fail")
    }

    pub fn task(&self, title: &str, extra: Value) -> Value {
        let mut input = json!({ "title": title });
        merge(&mut input, extra);
        self.call("tasks.create", input)
    }

    pub fn habit(&self, name: &str, extra: Value) -> Value {
        let mut input = json!({ "name": name, "startDate": "2026-10-01" });
        merge(&mut input, extra);
        self.call("habits.create", input)
    }

    /// Runs the scheduler and returns the titles it would show on the desktop.
    pub fn tick(&self) -> Vec<String> {
        self.core.tick().unwrap().into_iter().map(|notice| notice.title).collect()
    }

    pub fn set(&self, wall: &str) {
        self.clock.set(wall);
    }

    pub fn advance_minutes(&self, minutes: i64) {
        self.clock.advance(Duration::minutes(minutes));
    }

    pub fn list(&self, view: &str) -> Vec<String> {
        titles(&self.call("tasks.list", json!({ "view": view })))
    }
}

pub fn titles(tasks: &Value) -> Vec<String> {
    tasks.as_array().unwrap().iter().map(|task| task["title"].as_str().unwrap().to_string()).collect()
}

pub fn merge(target: &mut Value, extra: Value) {
    if let (Value::Object(target), Value::Object(extra)) = (target, extra) {
        target.extend(extra);
    }
}
