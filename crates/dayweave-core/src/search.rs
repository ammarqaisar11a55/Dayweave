use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::clock::Clock;
use crate::error::Result;
use crate::tasks::{self, like_pattern, ListParams, Task};

#[derive(Debug, Deserialize)]
pub struct Params {
    pub query: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Named {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    pub tasks: Vec<Task>,
    pub habits: Vec<Named>,
    pub projects: Vec<Named>,
    pub tags: Vec<Named>,
}

fn named(conn: &Connection, sql: &str, pattern: &str) -> Result<Vec<Named>> {
    Ok(conn
        .prepare(sql)?
        .query_map([pattern], |row| {
            Ok(Named { id: row.get(0)?, name: row.get(1)?, color: row.get(2)?, icon: row.get(3)? })
        })?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn query(conn: &Connection, clock: &dyn Clock, params: Params) -> Result<Results> {
    let text = params.query.trim();
    if text.is_empty() {
        return Ok(Results::default());
    }
    let pattern = like_pattern(text);
    Ok(Results {
        tasks: tasks::list(
            conn,
            clock,
            ListParams { search: Some(text.into()), limit: Some(8), ..Default::default() },
        )?,
        habits: named(
            conn,
            "SELECT id, name, color, icon FROM habits
             WHERE archived_at IS NULL AND (name LIKE ?1 ESCAPE '\\' OR description LIKE ?1 ESCAPE '\\')
             ORDER BY position LIMIT 5",
            &pattern,
        )?,
        projects: named(
            conn,
            "SELECT id, name, color, icon FROM projects
             WHERE archived_at IS NULL AND (name LIKE ?1 ESCAPE '\\' OR description LIKE ?1 ESCAPE '\\')
             ORDER BY name COLLATE NOCASE LIMIT 5",
            &pattern,
        )?,
        tags: named(
            conn,
            "SELECT id, name, NULL, NULL FROM tags WHERE name LIKE ?1 ESCAPE '\\'
             ORDER BY name COLLATE NOCASE LIMIT 5",
            &pattern,
        )?,
    })
}
