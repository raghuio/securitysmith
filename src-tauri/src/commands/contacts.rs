use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct Contact {
    pub id: u32,
    pub client_id: u32,
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
pub struct ContactInput {
    pub client_id: u32,
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

fn validate_role(role: &str) -> Result<(), String> {
    if VALID_ROLES.contains(&role) {
        Ok(())
    } else {
        Err(format!(
            "Invalid role: {}. Must be one of: {:?}",
            role, VALID_ROLES
        ))
    }
}

fn validate_contact(input: &ContactInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Name is required.".to_string());
    }
    if input.name.len() > 255 {
        return Err("Name must be 255 characters or fewer.".to_string());
    }
    if input.email.trim().is_empty() {
        return Err("Email is required.".to_string());
    }
    if input.email.len() > 255 {
        return Err("Email must be 255 characters or fewer.".to_string());
    }
    validate_role(&input.role)?;
    Ok(())
}

fn row_to_contact(row: &rusqlite::Row) -> Result<Contact, rusqlite::Error> {
    Ok(Contact {
        id: row.get(0)?,
        client_id: row.get(1)?,
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

pub fn do_list_contacts(conn: &Connection, client_id: u32) -> Result<Vec<Contact>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, client_id, name, email, phone, role, role_label, title, notes, is_primary, is_active, created_at, updated_at
             FROM client_contacts WHERE client_id = ? AND is_active = 1 ORDER BY is_primary DESC, name",
        )
        .map_err(|e| format!("Database error: {}", e))?;
    let rows = stmt
        .query_map(params![client_id], row_to_contact)
        .map_err(|e| format!("Database error: {}", e))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {}", e))?);
    }
    Ok(items)
}

pub fn do_get_contact(conn: &Connection, id: u32) -> Result<Contact, String> {
    let item: Option<Contact> = conn
        .query_row(
            "SELECT id, client_id, name, email, phone, role, role_label, title, notes, is_primary, is_active, created_at, updated_at
             FROM client_contacts WHERE id = ? AND is_active = 1",
            params![id],
            row_to_contact,
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;
    item.ok_or_else(|| "Contact not found.".to_string())
}

pub fn do_create_contact(conn: &Connection, input: &ContactInput) -> Result<u32, String> {
    validate_contact(input)?;

    if let Some(true) = input.is_primary {
        conn.execute(
            "UPDATE client_contacts SET is_primary = 0 WHERE client_id = ?",
            params![input.client_id],
        )
        .map_err(|e| format!("Failed to reset primary: {}", e))?;
    }

    conn.execute(
        "INSERT INTO client_contacts (client_id, name, email, phone, role, role_label, title, notes, is_primary, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, strftime('%s', 'now'))",
        params![
            input.client_id,
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
    .map_err(|e| format!("Failed to create contact: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["client_contacts", "INSERT", id, "", "", format!("client_id={}", input.client_id)],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(id)
}

pub fn do_update_contact(conn: &Connection, id: u32, input: &ContactInput) -> Result<(), String> {
    validate_contact(input)?;
    let old = do_get_contact(conn, id)?;

    if let Some(true) = input.is_primary {
        conn.execute(
            "UPDATE client_contacts SET is_primary = 0 WHERE client_id = ? AND id != ?",
            params![input.client_id, id],
        )
        .map_err(|e| format!("Failed to reset primary: {}", e))?;
    }

    conn.execute(
        "UPDATE client_contacts SET name = ?1, email = ?2, phone = ?3, role = ?4, role_label = ?5, title = ?6, notes = ?7, is_primary = ?8, updated_at = strftime('%s', 'now') WHERE id = ?9",
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
    .map_err(|e| format!("Failed to update contact: {}", e))?;

    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {}", e))?;
    let new = do_get_contact(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["client_contacts", "UPDATE", id, old_json, new_json, ""],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(())
}

pub fn do_delete_contact(conn: &Connection, id: u32) -> Result<(), String> {
    let old = do_get_contact(conn, id)?;
    conn.execute(
        "UPDATE client_contacts SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Failed to delete contact: {}", e))?;

    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["client_contacts", "DELETE", id, old_json, "", ""],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;
    Ok(())
}

// Tauri commands
#[tauri::command]
pub fn list_contacts(state: State<AppState>, client_id: u32) -> Result<Vec<Contact>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked.")?;
    do_list_contacts(conn, client_id)
}

#[tauri::command]
pub fn get_contact(state: State<AppState>, id: u32) -> Result<Contact, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked.")?;
    do_get_contact(conn, id)
}

#[tauri::command]
pub fn create_contact(state: State<AppState>, input: ContactInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_create_contact(conn, &input)
}

#[tauri::command]
pub fn update_contact(state: State<AppState>, id: u32, input: ContactInput) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_update_contact(conn, id, &input)
}

#[tauri::command]
pub fn delete_contact(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_delete_contact(conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let key = [9u8; 32];
        let conn = db::open_vault(tmp.path(), &key).unwrap();
        db::init_db(&conn).unwrap();
        conn.execute("INSERT INTO clients (name) VALUES ('Acme')", [])
            .unwrap();
        conn
    }

    #[test]
    fn test_contact_crud() {
        let conn = test_conn();
        let input = ContactInput {
            client_id: 1,
            name: "Jane Doe".to_string(),
            email: "jane@acme.com".to_string(),
            phone: None,
            role: "technical_poc".to_string(),
            role_label: None,
            title: None,
            notes: None,
            is_primary: Some(true),
        };
        let id = do_create_contact(&conn, &input).unwrap();
        let c = do_get_contact(&conn, id).unwrap();
        assert_eq!(c.name, "Jane Doe");
        assert!(c.is_primary);

        let list = do_list_contacts(&conn, 1).unwrap();
        assert_eq!(list.len(), 1);

        do_delete_contact(&conn, id).unwrap();
        assert!(do_get_contact(&conn, id).is_err());
    }
}
