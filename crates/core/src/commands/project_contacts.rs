use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ProjectContact {
    pub id: u32,
    pub project_id: u32,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
    pub role_label: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub is_primary: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
pub struct ProjectContactInput {
    pub project_id: u32,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
    pub role_label: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub is_primary: Option<bool>,
}

const VALID_ROLES: [&str; 7] = [
    "technical_poc",
    "management",
    "billing",
    "legal",
    "remediation",
    "executive",
    "other",
];

fn validate_role(role: &str) -> crate::error::Result<()> {
    if VALID_ROLES.contains(&role) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Invalid role: {role}. Must be one of: {VALID_ROLES:?}"
        )))
    }
}

fn validate_contact(input: &ProjectContactInput) -> crate::error::Result<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Name is required.".to_string()));
    }
    if input.name.len() > 255 {
        return Err(AppError::Validation("Name must be 255 characters or fewer.".to_string()));
    }
    if input.email.trim().is_empty() {
        return Err(AppError::Validation("Email is required.".to_string()));
    }
    if input.email.len() > 255 {
        return Err(AppError::Validation("Email must be 255 characters or fewer.".to_string()));
    }
    validate_role(&input.role)?;
    Ok(())
}

fn row_to_project_contact(row: &rusqlite::Row) -> Result<ProjectContact, rusqlite::Error> {
    Ok(ProjectContact {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        email: row.get(3)?,
        phone: row.get(4)?,
        role: row.get(5)?,
        role_label: row.get(6)?,
        title: row.get(7)?,
        notes: row.get(8)?,
        is_primary: row.get::<_, i32>(9)? != 0,
        is_active: row.get::<_, i32>(10)? != 0,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

#[must_use]
pub fn do_list_project_contacts(conn: &Connection, project_id: u32) -> crate::error::Result<Vec<ProjectContact>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, name, email, phone, role, role_label, title, notes, is_primary, is_active, created_at, updated_at
             FROM project_contacts WHERE project_id = ? AND is_active = 1 ORDER BY is_primary DESC, name",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![project_id], row_to_project_contact)
        .map_err(AppError::from)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(AppError::from)?);
    }
    Ok(items)
}

#[must_use]
pub fn do_get_project_contact(conn: &Connection, id: u32) -> crate::error::Result<ProjectContact> {
    let item: Option<ProjectContact> = conn
        .query_row(
            "SELECT id, project_id, name, email, phone, role, role_label, title, notes, is_primary, is_active, created_at, updated_at
             FROM project_contacts WHERE id = ? AND is_active = 1",
            params![id],
            row_to_project_contact,
        )
        .optional()
        .map_err(AppError::from)?;
    item.ok_or(AppError::ContactNotFound(id))
}

#[must_use]
pub fn do_create_project_contact(conn: &Connection, input: &ProjectContactInput) -> crate::error::Result<u32> {
    validate_contact(input)?;

    if let Some(true) = input.is_primary {
        conn.execute(
            "UPDATE project_contacts SET is_primary = 0 WHERE project_id = ?",
            params![input.project_id],
        )
        .map_err(AppError::from)?;
    }

    conn.execute(
        "INSERT INTO project_contacts (project_id, name, email, phone, role, role_label, title, notes, is_primary, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%s', 'now'))",
        params![
            input.project_id,
            input.name.trim(),
            input.email.trim(),
            input.phone.as_deref(),
            input.role.trim(),
            input.role_label.as_deref(),
            input.title.as_deref(),
            input.notes.as_deref(),
            input.is_primary.unwrap_or(false) as i32,
        ],
    )
    .map_err(AppError::from)?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| AppError::Generic("ID overflow".to_string()))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["project_contacts", "INSERT", id, "", "", format!("project_id={}", input.project_id)],
    )
    .map_err(AppError::from)?;

    Ok(id)
}

#[must_use]
pub fn do_update_project_contact(conn: &Connection, id: u32, input: &ProjectContactInput) -> crate::error::Result<()> {
    validate_contact(input)?;
    let old = do_get_project_contact(conn, id)?;

    if let Some(true) = input.is_primary {
        conn.execute(
            "UPDATE project_contacts SET is_primary = 0 WHERE project_id = ? AND id != ?",
            params![input.project_id, id],
        )
        .map_err(AppError::from)?;
    }

    conn.execute(
        "UPDATE project_contacts SET name = ?1, email = ?2, phone = ?3, role = ?4, role_label = ?5, title = ?6, notes = ?7, is_primary = ?8, updated_at = strftime('%s', 'now') WHERE id = ?9",
        params![
            input.name.trim(),
            input.email.trim(),
            input.phone.as_deref(),
            input.role.trim(),
            input.role_label.as_deref(),
            input.title.as_deref(),
            input.notes.as_deref(),
            input.is_primary.unwrap_or(false) as i32,
            id,
        ],
    )
    .map_err(AppError::from)?;

    let old_json = serde_json::to_string(&old).map_err(|e| AppError::Generic(e.to_string()))?;
    let new = do_get_project_contact(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| AppError::Generic(e.to_string()))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["project_contacts", "UPDATE", id, old_json, new_json, ""],
    )
    .map_err(AppError::from)?;

    Ok(())
}

#[must_use]
pub fn do_delete_project_contact(conn: &Connection, id: u32) -> crate::error::Result<()> {
    let old = do_get_project_contact(conn, id)?;
    conn.execute(
        "UPDATE project_contacts SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(AppError::from)?;

    let old_json = serde_json::to_string(&old).map_err(|e| AppError::Generic(e.to_string()))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["project_contacts", "DELETE", id, old_json, "", ""],
    )
    .map_err(AppError::from)?;
    Ok(())
}

// Tauri commands
#[tauri::command]
/// List contacts for a client.
pub fn list_project_contacts(state: State<AppState>, project_id: u32) -> Result<Vec<ProjectContact>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_project_contacts(conn, project_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_project_contact(state: State<AppState>, id: u32) -> Result<ProjectContact, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_project_contact(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
/// Create a new client contact.
pub fn create_project_contact(state: State<AppState>, input: ProjectContactInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_project_contact(conn, &input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_project_contact(state: State<AppState>, id: u32, input: ProjectContactInput) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_update_project_contact(conn, id, &input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project_contact(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_delete_project_contact(conn, id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_conn;
    use super::*;
    use crate::db;


    #[test]
    fn test_contact_crud() {
        let conn = test_conn();
        // Seed a parent client so the FK constraint is satisfied.
        conn.execute(
            "INSERT INTO clients (short_name, registered_business_name, email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params!["Acme Corp"],
        ).unwrap();
        // Seed a parent project so the FK constraint is satisfied.
        conn.execute(
            "INSERT INTO projects (client_id, name, status, is_active, created_at, updated_at)
             VALUES (?1, 'Test Project', 'active', 1, strftime('%s','now'), strftime('%s','now'))",
            params![1],
        ).unwrap();
        let input = ProjectContactInput {
            project_id: 1,
            name: "Jane Doe".to_string(),
            email: "jane@acme.com".to_string(),
            phone: None,
            role: "technical_poc".to_string(),
            role_label: None,
            title: None,
            notes: None,
            is_primary: Some(true),
        };
        let id = do_create_project_contact(&conn, &input).unwrap();
        let c = do_get_project_contact(&conn, id).unwrap();
        assert_eq!(c.name, "Jane Doe");
        assert!(c.is_primary);

        let list = do_list_project_contacts(&conn, 1).unwrap();
        assert_eq!(list.len(), 1);

        do_delete_project_contact(&conn, id).unwrap();
        assert!(do_get_project_contact(&conn, id).is_err());
    }
}
