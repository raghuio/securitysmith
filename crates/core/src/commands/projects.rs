use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: u32,
    pub client_id: u32,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub budgeted_hours: Option<u32>,
    pub tech_stack: Vec<String>,
    pub tentative_dates: Option<String>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub engagement_count: Option<u32>,
    pub client_name: Option<String>,
}

fn parse_json_arr(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn row_to_project(row: &rusqlite::Row) -> Result<Project, rusqlite::Error> {
    let ts: String = row.get(8)?;
    let tg: String = row.get(10)?;
    Ok(Project {
        id: row.get(0)?,
        client_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        start_date: row.get(5)?,
        end_date: row.get(6)?,
        budgeted_hours: row.get(7)?,
        tech_stack: parse_json_arr(&ts),
        tentative_dates: row.get(9)?,
        tags: parse_json_arr(&tg),
        notes: row.get(11)?,
        is_active: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        engagement_count: None,
        client_name: row.get(15).ok(),
    })
}

// ─────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────

#[must_use]
pub fn do_create_project(
    conn: &Connection,
    client_id: u32,
    name: &str,
    description: Option<&str>,
    status: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
    budgeted_hours: Option<u32>,
    tech_stack: Option<&Vec<String>>,
    tentative_dates: Option<&str>,
    tags: Option<&Vec<String>>,
    notes: Option<&str>,
) -> crate::error::Result<u32> {
    let ts_json =
        serde_json::to_string(&tech_stack.cloned().unwrap_or_default()).map_err(AppError::from)?;
    let tags_json =
        serde_json::to_string(&tags.cloned().unwrap_or_default()).map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO projects (client_id, name, description, status, start_date, end_date,
                               budgeted_hours, tech_stack, tentative_dates, tags, notes, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, strftime('%s', 'now'))",
        params![
            client_id,
            name,
            description,
            status,
            start_date,
            end_date,
            budgeted_hours,
            ts_json,
            tentative_dates,
            tags_json,
            notes,
        ],
    )
    .map_err(AppError::from)?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| AppError::Generic("ID overflow".to_string()))?;

    let new_project = do_get_project(conn, id).map_err(|e| e.to_string())?;
    let new_json = serde_json::to_string(&new_project).map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "projects",
            "INSERT",
            &id.to_string(),
            None::<&str>,
            &new_json,
            "create_project command"
        ],
    )
    .map_err(AppError::from)?;

    crate::commands::search::do_update_search_index_for_entity(conn, "project", id)
        .map_err(AppError::from)?;

    Ok(id)
}

fn do_get_project(conn: &Connection, id: u32) -> Result<Project, AppError> {
    let project: Option<Project> = conn
        .query_row(
            "SELECT id, client_id, name, description, status, start_date, end_date,
                    budgeted_hours, tech_stack, tentative_dates, tags, notes, is_active,
                    created_at, updated_at, NULL, NULL
             FROM projects WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_project,
        )
        .optional()
        .map_err(AppError::from)?;

    project.ok_or(AppError::Generic(format!("Project {} not found", id)))
}

fn do_update_project(
    conn: &Connection,
    id: u32,
    name: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    budgeted_hours: Option<u32>,
    tech_stack: Option<&Vec<String>>,
    tentative_dates: Option<&str>,
    tags: Option<&Vec<String>>,
    notes: Option<&str>,
) -> crate::error::Result<()> {
    let old: Option<Project> = conn
        .query_row(
            "SELECT id, client_id, name, description, status, start_date, end_date,
                    budgeted_hours, tech_stack, tentative_dates, tags, notes, is_active,
                    created_at, updated_at, NULL, NULL
             FROM projects WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_project,
        )
        .optional()
        .map_err(AppError::from)?;

    let old = old.ok_or_else(|| AppError::Generic(format!("Project {} not found", id)))?;

    let update_name = name.unwrap_or(&old.name);
    let update_desc = description.or(old.description.as_deref());
    let update_status = status.unwrap_or(&old.status);
    let update_start = start_date.or(old.start_date.as_deref());
    let update_end = end_date.or(old.end_date.as_deref());
    let update_budget = budgeted_hours.or(old.budgeted_hours);
    let update_tech = tech_stack.cloned().unwrap_or(old.tech_stack.clone());
    let update_tentative = tentative_dates.or(old.tentative_dates.as_deref());
    let update_tags = tags.cloned().unwrap_or(old.tags.clone());
    let update_notes = notes.or(old.notes.as_deref());

    let ts_json = serde_json::to_string(&update_tech).map_err(AppError::from)?;
    let tags_json = serde_json::to_string(&update_tags).map_err(AppError::from)?;

    conn.execute(
        "UPDATE projects SET
            name = ?1, description = ?2, status = ?3, start_date = ?4, end_date = ?5,
            budgeted_hours = ?6, tech_stack = ?7, tentative_dates = ?8, tags = ?9,
            notes = ?10, updated_at = strftime('%s', 'now')
         WHERE id = ?11",
        params![
            update_name,
            update_desc,
            update_status,
            update_start,
            update_end,
            update_budget,
            ts_json,
            update_tentative,
            tags_json,
            update_notes,
            id,
        ],
    )
    .map_err(AppError::from)?;

    let old_json = serde_json::to_string(&old).map_err(AppError::from)?;
    let new_project = do_get_project(conn, id).map_err(|e| e.to_string())?;
    let new_json = serde_json::to_string(&new_project).map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "projects",
            "UPDATE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_project command"
        ],
    )
    .map_err(AppError::from)?;

    crate::commands::search::do_update_search_index_for_entity(conn, "project", id)
        .map_err(AppError::from)?;

    Ok(())
}

fn do_archive_project(conn: &Connection, id: u32) -> crate::error::Result<()> {
    conn.execute(
        "UPDATE projects SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?1",
        params![id],
    )
    .map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "projects",
            "DELETE",
            &id.to_string(),
            None::<&str>,
            None::<&str>,
            "archive_project command"
        ],
    )
    .map_err(AppError::from)?;

    conn.execute(
        "DELETE FROM search_index WHERE entity_type = 'project' AND entity_id = ?1",
        params![id],
    )
    .map_err(AppError::from)?;

    Ok(())
}

fn do_list_projects(
    conn: &Connection,
    client_id: u32,
    search: Option<&str>,
) -> crate::error::Result<Vec<Project>> {
    let mut sql = String::from(
        "SELECT p.id, p.client_id, p.name, p.description, p.status, p.start_date, p.end_date,
                p.budgeted_hours, p.tech_stack, p.tentative_dates, p.tags, p.notes, p.is_active,
                p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM engagements e WHERE e.project_id = p.id AND e.is_active = 1) AS engagement_count,
                c.short_name AS client_name
         FROM projects p
         JOIN clients c ON c.id = p.client_id
         WHERE p.is_active = 1 AND p.client_id = ?1"
    );

    let results = if let Some(s) = search {
        let pattern = format!("%{}%", s.trim());
        sql.push_str(
            " AND (p.name LIKE ?2 OR p.description LIKE ?2 OR p.tags LIKE ?2 OR p.notes LIKE ?2)",
        );
        sql.push_str(" ORDER BY p.updated_at DESC");
        let mut stmt = conn.prepare(&sql).map_err(AppError::from)?;
        stmt.query_map(params![client_id, pattern], |row| {
            let mut p = row_to_project(row)?;
            p.engagement_count = row.get(16).ok();
            p.client_name = row.get(17).ok();
            Ok(p)
        })
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?
    } else {
        sql.push_str(" ORDER BY p.updated_at DESC");
        let mut stmt = conn.prepare(&sql).map_err(AppError::from)?;
        stmt.query_map(params![client_id], |row| {
            let mut p = row_to_project(row)?;
            p.engagement_count = row.get(16).ok();
            p.client_name = row.get(17).ok();
            Ok(p)
        })
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?
    };

    Ok(results)
}

fn do_search_projects(conn: &Connection, query: &str) -> crate::error::Result<Vec<Project>> {
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
            "SELECT p.id, p.client_id, p.name, p.description, p.status, p.start_date, p.end_date,
                    p.budgeted_hours, p.tech_stack, p.tentative_dates, p.tags, p.notes, p.is_active,
                    p.created_at, p.updated_at,
                    (SELECT COUNT(*) FROM engagements e WHERE e.project_id = p.id AND e.is_active = 1) AS engagement_count,
                    c.short_name AS client_name
             FROM projects p
             JOIN clients c ON c.id = p.client_id
             WHERE p.is_active = 1 AND (p.name LIKE ?1 OR p.description LIKE ?1 OR p.tags LIKE ?1)
             ORDER BY p.updated_at DESC"
        )
        .map_err(AppError::from)?;
    let results = stmt
        .query_map(params![pattern], |row| {
            let mut p = row_to_project(row)?;
            p.engagement_count = row.get(16).ok();
            p.client_name = row.get(17).ok();
            Ok(p)
        })
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    Ok(results)
}

// ─────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────

fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation(
            "Project name is required.".to_string(),
        ));
    }
    if name.len() > 255 {
        return Err(AppError::Validation(
            "Project name must be 255 characters or fewer.".to_string(),
        ));
    }
    Ok(())
}

fn validate_dates(start: Option<&str>, end: Option<&str>) -> Result<(), AppError> {
    if let (Some(s), Some(e)) = (start, end)
        && e < s
    {
        return Err(AppError::Validation(
            "End date cannot be before start date.".to_string(),
        ));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    client_id: u32,
    name: String,
    description: Option<String>,
    status: String,
    start_date: Option<String>,
    end_date: Option<String>,
    budgeted_hours: Option<u32>,
    tech_stack: Option<Vec<String>>,
    tentative_dates: Option<String>,
    tags: Option<Vec<String>>,
    notes: Option<String>,
) -> Result<u32, String> {
    validate_name(&name)?;
    validate_dates(start_date.as_deref(), end_date.as_deref())?;

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_create_project(
        conn,
        client_id,
        name.trim(),
        description.as_deref(),
        &status,
        start_date.as_deref(),
        end_date.as_deref(),
        budgeted_hours,
        tech_stack.as_ref(),
        tentative_dates.as_deref(),
        tags.as_ref(),
        notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project(state: State<AppState>, id: u32) -> Result<Project, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_get_project(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_project(
    state: State<AppState>,
    id: u32,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    budgeted_hours: Option<u32>,
    tech_stack: Option<Vec<String>>,
    tentative_dates: Option<String>,
    tags: Option<Vec<String>>,
    notes: Option<String>,
) -> Result<(), String> {
    if let Some(ref n) = name {
        validate_name(n)?;
    }
    validate_dates(start_date.as_deref(), end_date.as_deref())?;

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_update_project(
        conn,
        id,
        name.as_deref(),
        description.as_deref(),
        status.as_deref(),
        start_date.as_deref(),
        end_date.as_deref(),
        budgeted_hours,
        tech_stack.as_ref(),
        tentative_dates.as_deref(),
        tags.as_ref(),
        notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn archive_project(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_archive_project(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_projects(
    state: State<AppState>,
    client_id: u32,
    search: Option<String>,
) -> Result<Vec<Project>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_list_projects(conn, client_id, search.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_projects(state: State<AppState>, query: String) -> Result<Vec<Project>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_search_projects(conn, &query).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::clients::do_create_client;
    use crate::test_helpers::test_conn;

    #[test]
    fn test_create_and_get_project() {
        let conn = test_conn();
        let cid = do_create_client(
            &conn,
            "Acme",
            "Acme Corp",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let pid = do_create_project(
            &conn,
            cid,
            "Q3 Program",
            Some("Desc"),
            "active",
            Some("2024-01-01"),
            Some("2024-12-31"),
            Some(100),
            Some(&vec!["nginx".to_string(), "wordpress".to_string()]),
            None,
            Some(&vec!["critical".to_string()]),
            None,
        )
        .unwrap();

        let project = do_get_project(&conn, pid).unwrap();
        assert_eq!(project.name, "Q3 Program");
        assert_eq!(project.client_id, cid);
        assert_eq!(project.status, "active");
        assert_eq!(project.start_date, Some("2024-01-01".to_string()));
        assert_eq!(project.tech_stack, vec!["nginx", "wordpress"]);
    }

    #[test]
    fn test_list_projects_by_client() {
        let conn = test_conn();
        let cid = do_create_client(
            &conn,
            "Acme",
            "Acme Corp",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        do_create_project(
            &conn, cid, "P1", None, "active", None, None, None, None, None, None, None,
        )
        .unwrap();
        do_create_project(
            &conn, cid, "P2", None, "active", None, None, None, None, None, None, None,
        )
        .unwrap();

        let list = do_list_projects(&conn, cid, None).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_archive_project() {
        let conn = test_conn();
        let cid = do_create_client(
            &conn,
            "Acme",
            "Acme Corp",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let pid = do_create_project(
            &conn, cid, "P1", None, "active", None, None, None, None, None, None, None,
        )
        .unwrap();

        do_archive_project(&conn, pid).unwrap();
        let result = do_get_project(&conn, pid);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_project_name_rejected() {
        let conn = test_conn();
        let cid = do_create_client(
            &conn,
            "Acme",
            "Acme Corp",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        do_create_project(
            &conn, cid, "P1", None, "active", None, None, None, None, None, None, None,
        )
        .unwrap();
        let result = do_create_project(
            &conn, cid, "P1", None, "active", None, None, None, None, None, None, None,
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("UNIQUE constraint")
        );
    }

    #[test]
    fn test_date_validation() {
        assert!(validate_dates(Some("2024-01-01"), Some("2024-12-31")).is_ok());
        assert!(validate_dates(Some("2024-12-31"), Some("2024-01-01")).is_err());
    }
}
