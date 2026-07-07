use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
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

fn validate_type(t: &str) -> crate::error::Result<()> {
    if VALID_TYPES.contains(&t) {
        Ok(())
    } else {
        Err(AppError::Generic(format!(
            "Invalid type: {t}. Must be one of: {VALID_TYPES:?}"
        )))
    }
}

fn validate_item(input: &ScopeItemInput) -> crate::error::Result<()> {
    if input.value.trim().is_empty() {
        return Err(AppError::Generic("Value is required.".to_string()));
    }
    if input.value.len() > 2000 {
        return Err(AppError::Generic("Value must be 2,000 characters or fewer.".to_string()));
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

#[must_use]
pub fn do_list_scope_items(
    conn: &Connection,
    engagement_id: u32,
) -> crate::error::Result<Vec<ScopeItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, engagement_id, item_type, value, is_in_scope, environment, notes, sort_order, created_at, updated_at
             FROM scope_items WHERE engagement_id = ? AND is_active = 1 ORDER BY sort_order, created_at",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![engagement_id], row_to_item)
        .map_err(AppError::from)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(AppError::from)?);
    }
    Ok(items)
}

#[must_use]
pub fn do_create_scope_item(conn: &Connection, input: &ScopeItemInput) -> crate::error::Result<u32> {
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
    .map_err(AppError::from)?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| AppError::Generic("ID overflow".to_string()))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["scope_items", "INSERT", id, "", "", format!("engagement_id={}", input.engagement_id)],
    )
    .map_err(AppError::from)?;
    Ok(id)
}

#[must_use]
pub fn do_update_scope_item(
    conn: &Connection,
    id: u32,
    input: &ScopeItemInput,
) -> crate::error::Result<()> {
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
    .map_err(AppError::from)?;
    Ok(())
}

#[must_use]
pub fn do_delete_scope_item(conn: &Connection, id: u32) -> crate::error::Result<()> {
    conn.execute(
        "UPDATE scope_items SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(AppError::from)?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["scope_items", "DELETE", id, "", "", ""],
    )
    .map_err(AppError::from)?;
    Ok(())
}

#[must_use]
pub fn do_bulk_import_scope_items(
    conn: &Connection,
    engagement_id: u32,
    lines: &str,
) -> crate::error::Result<u32> {
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
        .map_err(AppError::from)?;
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

#[must_use]
pub fn do_export_scope_text(conn: &Connection, engagement_id: u32) -> crate::error::Result<String> {
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
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_scope_items(conn, engagement_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_scope_item(state: State<AppState>, input: ScopeItemInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_scope_item(conn, &input).map_err(|e| e.to_string())
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
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_update_scope_item(conn, id, &input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_scope_item(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_delete_scope_item(conn, id).map_err(|e| e.to_string())
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
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_bulk_import_scope_items(conn, engagement_id, &lines).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_scope_text(state: State<AppState>, engagement_id: u32) -> Result<String, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_export_scope_text(conn, engagement_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_conn;
    use super::*;
    use crate::db;


    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn make_engagement(conn: &Connection) -> u32 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conn.execute(
            "INSERT INTO clients (short_name, registered_business_name, email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            rusqlite::params![format!("Client-{n}")],
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
