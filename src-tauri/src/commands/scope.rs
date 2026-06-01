use crate::state::AppState;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct ScopeItem {
    pub id: u32,
    pub engagement_id: u32,
    pub item_type: String,
    pub value: String,
    pub is_in_scope: bool,
    pub environment: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
pub struct ScopeItemInput {
    pub engagement_id: u32,
    pub item_type: String,
    pub value: String,
    pub is_in_scope: Option<bool>,
    pub environment: Option<String>,
    pub notes: Option<String>,
}

const VALID_TYPES: [&str; 10] = [
    "url",
    "ip",
    "ip_range",
    "cidr",
    "domain",
    "subdomain",
    "application",
    "api_endpoint",
    "host",
    "other",
];

fn validate_type(t: &str) -> Result<(), String> {
    if VALID_TYPES.contains(&t) {
        Ok(())
    } else {
        Err(format!(
            "Invalid type: {}. Must be one of: {:?}",
            t, VALID_TYPES
        ))
    }
}

fn validate_item(input: &ScopeItemInput) -> Result<(), String> {
    if input.value.trim().is_empty() {
        return Err("Value is required.".to_string());
    }
    if input.value.len() > 2000 {
        return Err("Value must be 2,000 characters or fewer.".to_string());
    }
    validate_type(&input.item_type)?;
    Ok(())
}

fn row_to_item(row: &rusqlite::Row) -> Result<ScopeItem, rusqlite::Error> {
    Ok(ScopeItem {
        id: row.get(0)?,
        engagement_id: row.get(1)?,
        item_type: row.get(2)?,
        value: row.get(3)?,
        is_in_scope: row.get::<_, i32>(4)? != 0,
        environment: row.get(5)?,
        notes: row.get(6)?,
        sort_order: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn do_list_scope_items(
    conn: &Connection,
    engagement_id: u32,
) -> Result<Vec<ScopeItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, engagement_id, item_type, value, is_in_scope, environment, notes, sort_order, created_at, updated_at
             FROM scope_items WHERE engagement_id = ? AND is_active = 1 ORDER BY sort_order, created_at",
        )
        .map_err(|e| format!("Database error: {}", e))?;
    let rows = stmt
        .query_map(params![engagement_id], row_to_item)
        .map_err(|e| format!("Database error: {}", e))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {}", e))?);
    }
    Ok(items)
}

pub fn do_create_scope_item(conn: &Connection, input: &ScopeItemInput) -> Result<u32, String> {
    validate_item(input)?;
    conn.execute(
        "INSERT INTO scope_items (engagement_id, item_type, value, is_in_scope, environment, notes, sort_order, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, strftime('%s', 'now'))",
        params![
            input.engagement_id,
            input.item_type.trim(),
            input.value.trim(),
            input.is_in_scope.unwrap_or(true) as i32,
            input.environment.as_deref(),
            input.notes.as_deref(),
            0,
        ],
    )
    .map_err(|e| format!("Failed to create scope item: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["scope_items", "INSERT", id, "", "", format!("engagement_id={}", input.engagement_id)],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;
    Ok(id)
}

pub fn do_update_scope_item(
    conn: &Connection,
    id: u32,
    input: &ScopeItemInput,
) -> Result<(), String> {
    validate_item(input)?;
    conn.execute(
        "UPDATE scope_items SET item_type = ?1, value = ?2, is_in_scope = ?3, environment = ?4, notes = ?5, updated_at = strftime('%s', 'now') WHERE id = ?6",
        params![
            input.item_type.trim(),
            input.value.trim(),
            input.is_in_scope.unwrap_or(true) as i32,
            input.environment.as_deref(),
            input.notes.as_deref(),
            id,
        ],
    )
    .map_err(|e| format!("Failed to update scope item: {}", e))?;
    Ok(())
}

pub fn do_delete_scope_item(conn: &Connection, id: u32) -> Result<(), String> {
    conn.execute(
        "UPDATE scope_items SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Failed to delete scope item: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["scope_items", "DELETE", id, "", "", ""],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;
    Ok(())
}

pub fn do_bulk_import_scope_items(
    conn: &Connection,
    engagement_id: u32,
    lines: &str,
) -> Result<u32, String> {
    let mut count = 0;
    for line in lines.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let item_type = detect_type(trimmed);
        conn.execute(
            "INSERT INTO scope_items (engagement_id, item_type, value, is_in_scope, sort_order, updated_at)
             VALUES (?1, ?2, ?3, 1, ?4, strftime('%s', 'now'))",
            params![engagement_id, item_type, trimmed, count],
        )
        .map_err(|e| format!("Failed to import scope item: {}", e))?;
        count += 1;
    }
    Ok(count)
}

fn detect_type(value: &str) -> &str {
    if value.starts_with("http://") || value.starts_with("https://") {
        "url"
    } else if regex_simple_ip(value) {
        "ip"
    } else if value.contains('/') {
        "cidr"
    } else if value.contains('-') && value.split('-').count() == 2 {
        "ip_range"
    } else {
        "domain"
    }
}

fn regex_simple_ip(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u8>().is_ok())
}

pub fn do_export_scope_text(conn: &Connection, engagement_id: u32) -> Result<String, String> {
    let items = do_list_scope_items(conn, engagement_id)?;
    let mut lines = Vec::new();
    lines.push("IN SCOPE".to_string());
    for item in &items {
        if item.is_in_scope {
            lines.push(format!("- [{}] {}", item.item_type, item.value));
        }
    }
    lines.push("".to_string());
    lines.push("OUT OF SCOPE".to_string());
    for item in &items {
        if !item.is_in_scope {
            lines.push(format!("- [{}] {}", item.item_type, item.value));
        }
    }
    Ok(lines.join("\n"))
}

// Tauri commands
#[tauri::command]
pub fn list_scope_items(
    state: State<AppState>,
    engagement_id: u32,
) -> Result<Vec<ScopeItem>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked.")?;
    do_list_scope_items(conn, engagement_id)
}

#[tauri::command]
pub fn create_scope_item(state: State<AppState>, input: ScopeItemInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_create_scope_item(conn, &input)
}

#[tauri::command]
pub fn update_scope_item(
    state: State<AppState>,
    id: u32,
    input: ScopeItemInput,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_update_scope_item(conn, id, &input)
}

#[tauri::command]
pub fn delete_scope_item(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_delete_scope_item(conn, id)
}

#[tauri::command]
pub fn bulk_import_scope_items(
    state: State<AppState>,
    engagement_id: u32,
    lines: String,
) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked.")?;
    do_bulk_import_scope_items(conn, engagement_id, &lines)
}

#[tauri::command]
pub fn export_scope_text(state: State<AppState>, engagement_id: u32) -> Result<String, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked.")?;
    do_export_scope_text(conn, engagement_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn make_engagement(conn: &Connection) -> u32 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            rusqlite::params![format!("Client-{}", n)],
        )
        .unwrap();
        let cid = conn.last_insert_rowid() as u32;
        conn.execute(
            "INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model,
                engagement_type, status, objectives, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, 'Eng', 'web', 'pentest', 'auth', 'pentest', 'active', '[]', NULL, '[]', 1,
                strftime('%s','now'), strftime('%s','now'))",
            rusqlite::params![cid],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    #[test]
    fn test_scope_list_empty() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let items = do_list_scope_items(&conn, eid).unwrap();
        assert!(items.is_empty());
    }
}
