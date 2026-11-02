use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::clock::{format_instant, parse_date, Clock};
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub task_total: i64,
    pub task_done: i64,
    pub task_open: i64,
    pub task_overdue: i64,
    /// Share of the project's tasks that are finished, from 0 to 1.
    pub completion: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectInput {
    pub name: String,
    pub description: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
}

const SELECT: &str = "
    SELECT p.id, p.name, p.description, p.color, p.icon, p.start_date, p.deadline, p.archived_at,
           p.created_at,
           count(t.id) FILTER (WHERE t.status != 'archived'),
           count(t.id) FILTER (WHERE t.status = 'completed'),
           count(t.id) FILTER (WHERE t.status IN ('todo', 'in_progress')),
           count(t.id) FILTER (WHERE t.status IN ('todo', 'in_progress') AND t.due_date < ?1)
    FROM projects p LEFT JOIN tasks t ON t.project_id = p.id";

fn map_row(row: &Row) -> rusqlite::Result<Project> {
    let total: i64 = row.get(9)?;
    let done: i64 = row.get(10)?;
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        color: row.get(3)?,
        icon: row.get(4)?,
        start_date: row.get(5)?,
        deadline: row.get(6)?,
        archived_at: row.get(7)?,
        created_at: row.get(8)?,
        task_total: total,
        task_done: done,
        task_open: row.get(11)?,
        task_overdue: row.get(12)?,
        completion: if total == 0 { 0.0 } else { done as f64 / total as f64 },
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ListParams {
    pub include_archived: bool,
}

pub fn list(conn: &Connection, clock: &dyn Clock, params: ListParams) -> Result<Vec<Project>> {
    let filter = if params.include_archived { "" } else { "WHERE p.archived_at IS NULL" };
    let sql = format!(
        "{SELECT} {filter} GROUP BY p.id
         ORDER BY p.archived_at IS NOT NULL, p.deadline IS NULL, p.deadline, p.name COLLATE NOCASE"
    );
    let today = crate::clock::format_date(clock.today());
    Ok(conn.prepare(&sql)?.query_map([today], map_row)?.collect::<rusqlite::Result<_>>()?)
}

pub fn get(conn: &Connection, clock: &dyn Clock, id: i64) -> Result<Project> {
    let sql = format!("{SELECT} WHERE p.id = ?2 GROUP BY p.id");
    let today = crate::clock::format_date(clock.today());
    conn.query_row(&sql, params![today, id], map_row).map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Error::NotFound("This project"),
        other => other.into(),
    })
}

fn validate(input: &mut ProjectInput) -> Result<()> {
    input.name = input.name.trim().to_string();
    if input.name.is_empty() {
        return Err(Error::invalid("Give the project a name"));
    }
    input.start_date = input.start_date.take().filter(|text| !text.is_empty());
    input.deadline = input.deadline.take().filter(|text| !text.is_empty());
    let start = input.start_date.as_deref().map(parse_date).transpose()?;
    let deadline = input.deadline.as_deref().map(parse_date).transpose()?;
    if let (Some(start), Some(deadline)) = (start, deadline) {
        if start > deadline {
            return Err(Error::invalid("The start date must be on or before the deadline"));
        }
    }
    Ok(())
}

pub fn create(conn: &Connection, clock: &dyn Clock, mut input: ProjectInput) -> Result<Project> {
    validate(&mut input)?;
    conn.execute(
        "INSERT INTO projects (name, description, color, icon, start_date, deadline, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            input.name,
            input.description,
            input.color.as_deref().unwrap_or("orange"),
            input.icon.as_deref().unwrap_or("folder"),
            input.start_date,
            input.deadline,
            format_instant(clock.now())
        ],
    )?;
    get(conn, clock, conn.last_insert_rowid())
}

#[derive(Debug, Deserialize)]
pub struct UpdateParams {
    pub id: i64,
    #[serde(flatten)]
    pub input: ProjectInput,
}

pub fn update(conn: &Connection, clock: &dyn Clock, params: UpdateParams) -> Result<Project> {
    let UpdateParams { id, mut input } = params;
    validate(&mut input)?;
    let current = get(conn, clock, id)?;
    conn.execute(
        "UPDATE projects SET name = ?1, description = ?2, color = ?3, icon = ?4, start_date = ?5,
                deadline = ?6, updated_at = ?7 WHERE id = ?8",
        params![
            input.name,
            input.description,
            input.color.as_deref().unwrap_or(&current.color),
            input.icon.as_deref().unwrap_or(&current.icon),
            input.start_date,
            input.deadline,
            format_instant(clock.now()),
            id
        ],
    )?;
    get(conn, clock, id)
}

pub fn set_archived(conn: &Connection, clock: &dyn Clock, id: i64, archived: bool) -> Result<Project> {
    get(conn, clock, id)?;
    let now = format_instant(clock.now());
    conn.execute(
        "UPDATE projects SET archived_at = ?1, updated_at = ?2 WHERE id = ?3",
        params![archived.then_some(&now), now, id],
    )?;
    get(conn, clock, id)
}

/// Removes a project and leaves its tasks in place without one.
pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    if conn.execute("DELETE FROM projects WHERE id = ?1", [id])? == 0 {
        return Err(Error::NotFound("This project"));
    }
    Ok(())
}
