use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct Client {
    pub id: u32,
    pub name: String,
    pub contact_email: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub tech_stack: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
pub struct DashboardStats {
    pub client_count: u32,
    pub finding_count: u32,
    pub engagement_count: u32,
    pub findings_ready: bool,
    pub engagements_ready: bool,
}

fn parse_json_arr(tag_str: &str) -> Vec<String> {
    serde_json::from_str(tag_str).unwrap_or_default()
}

fn row_to_client(row: &rusqlite::Row) -> Result<Client, rusqlite::Error> {
    let tags_str: String = row.get(4)?;
    let stack_str: String = row.get(7)?;
    Ok(Client {
        id: row.get(0)?,
        name: row.get(1)?,
        contact_email: row.get(2)?,
        notes: row.get(3)?,
        tags: parse_json_arr(&tags_str),
        tech_stack: parse_json_arr(&stack_str),
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

// ─────────────────────────────────────────────────────────────
// Core logic (testable without Tauri State)
// ─────────────────────────────────────────────────────────────

pub fn do_create_client(
    conn: &Connection,
    name: &str,
    contact_email: Option<&str>,
    notes: Option<&str>,
    tags: Option<&Vec<String>>,
    tech_stack: Option<&Vec<String>>,
) -> Result<u32, String> {
    let tags_json = serde_json::to_string(&tags.cloned().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;
    let stack_json = serde_json::to_string(&tech_stack.cloned().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize tech_stack: {}", e))?;

    conn.execute(
        "INSERT INTO clients (name, contact_email, notes, tags, tech_stack, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s', 'now'))",
        params![name, contact_email, notes, tags_json, stack_json],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            "A client with this name already exists.".to_string()
        } else {
            format!("Failed to create client: {}", e)
        }
    })?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    let new_client = do_get_client(conn, id)?;
    let new_json = serde_json::to_string(&new_client)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "clients",
            "INSERT",
            &id.to_string(),
            None::<&str>,
            &new_json,
            "create_client command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Update the global search index (PROP-028)
    crate::commands::search::do_update_search_index_for_entity(conn, "client", id)
        .map_err(|e| format!("Search index update failed: {}", e))?;

    Ok(id)
}

fn do_get_client(conn: &Connection, id: u32) -> Result<Client, String> {
    let client: Option<Client> = conn
        .query_row(
            "SELECT id, name, contact_email, notes, tags, created_at, updated_at, tech_stack
             FROM clients WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_client,
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    client.ok_or("Client not found.".to_string())
}

fn do_update_client(
    conn: &Connection,
    id: u32,
    name: Option<&str>,
    contact_email: Option<&str>,
    notes: Option<&str>,
    tags: Option<&Vec<String>>,
    tech_stack: Option<&Vec<String>>,
) -> Result<(), String> {
    let old: Option<Client> = conn
        .query_row(
            "SELECT id, name, contact_email, notes, tags, created_at, updated_at, tech_stack
             FROM clients WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_client,
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    let old = old.ok_or("Client not found.".to_string())?;

    let update_name = name.unwrap_or(&old.name);
    let update_email = contact_email.unwrap_or(old.contact_email.as_deref().unwrap_or(""));
    let update_notes = notes.unwrap_or(old.notes.as_deref().unwrap_or(""));
    let update_tags = tags.cloned().unwrap_or(old.tags.clone());
    let update_stack = tech_stack.cloned().unwrap_or(old.tech_stack.clone());
    let tags_json = serde_json::to_string(&update_tags)
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;
    let stack_json = serde_json::to_string(&update_stack)
        .map_err(|e| format!("Failed to serialize tech_stack: {}", e))?;

    conn.execute(
        "UPDATE clients SET
            name = ?1,
            contact_email = ?2,
            notes = ?3,
            tags = ?4,
            tech_stack = ?5,
            updated_at = strftime('%s', 'now')
         WHERE id = ?6",
        params![
            update_name,
            update_email,
            update_notes,
            tags_json,
            stack_json,
            id
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            "A client with this name already exists.".to_string()
        } else {
            format!("Failed to update client: {}", e)
        }
    })?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_client = do_get_client(conn, id)?;
    let new_json = serde_json::to_string(&new_client)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "clients",
            "UPDATE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_client command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Update the global search index (PROP-028)
    crate::commands::search::do_update_search_index_for_entity(conn, "client", id)
        .map_err(|e| format!("Search index update failed: {}", e))?;

    Ok(())
}

fn do_delete_client(conn: &Connection, id: u32) -> Result<(), String> {
    let old: Option<Client> = conn
        .query_row(
            "SELECT id, name, contact_email, notes, tags, created_at, updated_at, tech_stack
             FROM clients WHERE id = ?1 AND is_active = 1",
            params![id],
            row_to_client,
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    let old = old.ok_or("Client not found.".to_string())?;

    conn.execute(
        "UPDATE clients SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?1",
        params![id],
    )
    .map_err(|e| format!("Failed to delete client: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_json = serde_json::json!({
        "id": id,
        "is_active": 0
    })
    .to_string();

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "clients",
            "DELETE",
            &id.to_string(),
            &old_json,
            &new_json,
            "delete_client command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Remove from global search index (PROP-028)
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = 'client' AND entity_id = ?1",
        params![id],
    )
    .map_err(|e| format!("Search index removal failed: {}", e))?;

    Ok(())
}

fn do_list_clients(conn: &Connection, search: Option<&str>) -> Result<Vec<Client>, String> {
    let mut sql = String::from(
        "SELECT id, name, contact_email, notes, tags, created_at, updated_at, tech_stack
         FROM clients WHERE is_active = 1",
    );

    let results = if let Some(s) = search {
        let pattern = format!("%{}%", s.trim());
        sql.push_str(
            " AND (name LIKE ?1 OR contact_email LIKE ?1 OR tags LIKE ?1 OR tech_stack LIKE ?1)",
        );
        sql.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Database error: {}", e))?;
        stmt.query_map(params![pattern], row_to_client)
            .map_err(|e| format!("Database error: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Database error: {}", e))?
    } else {
        sql.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Database error: {}", e))?;
        stmt.query_map([], row_to_client)
            .map_err(|e| format!("Database error: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Database error: {}", e))?
    };

    Ok(results)
}

fn do_get_dashboard_stats(conn: &Connection) -> Result<DashboardStats, String> {
    let client_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM clients WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?;

    let engagement_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM engagements WHERE is_active = 1 AND status = 'active'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?;

    let finding_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(DashboardStats {
        client_count,
        finding_count,
        engagement_count,
        findings_ready: true,
        engagements_ready: true,
    })
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Client name is required.".to_string());
    }
    if name.len() > 255 {
        return Err("Client name must be 255 characters or fewer.".to_string());
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), String> {
    if !email.is_empty() && !is_valid_email(email) {
        return Err("Contact email is invalid.".to_string());
    }
    Ok(())
}

fn validate_notes(notes: &str) -> Result<(), String> {
    if notes.len() > 10_000 {
        return Err("Notes must be 10,000 characters or fewer.".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn create_client(
    state: State<AppState>,
    name: String,
    contact_email: Option<String>,
    notes: Option<String>,
    tags: Option<Vec<String>>,
    tech_stack: Option<Vec<String>>,
) -> Result<u32, String> {
    validate_name(&name)?;
    if let Some(ref email) = contact_email {
        validate_email(email)?;
    }
    if let Some(ref n) = notes {
        validate_notes(n)?;
    }

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;

    do_create_client(
        conn,
        name.trim(),
        contact_email.as_deref(),
        notes.as_deref(),
        tags.as_ref(),
        tech_stack.as_ref(),
    )
}

#[tauri::command]
pub fn get_client(state: State<AppState>, id: u32) -> Result<Client, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;

    do_get_client(conn, id)
}

#[tauri::command]
pub fn update_client(
    state: State<AppState>,
    id: u32,
    name: Option<String>,
    contact_email: Option<String>,
    notes: Option<String>,
    tags: Option<Vec<String>>,
    tech_stack: Option<Vec<String>>,
) -> Result<(), String> {
    if let Some(ref n) = name {
        validate_name(n)?;
    }
    if let Some(ref email) = contact_email {
        validate_email(email)?;
    }
    if let Some(ref n) = notes {
        validate_notes(n)?;
    }

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;

    do_update_client(
        conn,
        id,
        name.as_deref(),
        contact_email.as_deref(),
        notes.as_deref(),
        tags.as_ref(),
        tech_stack.as_ref(),
    )
}

#[tauri::command]
pub fn delete_client(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;

    do_delete_client(conn, id)
}

#[tauri::command]
pub fn list_clients(state: State<AppState>, search: Option<String>) -> Result<Vec<Client>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;

    do_list_clients(conn, search.as_deref())
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<AppState>) -> Result<DashboardStats, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;

    do_get_dashboard_stats(conn)
}

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let domain = parts[1];
    domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

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

    #[test]
    fn test_create_and_get_client() {
        let conn = test_conn();
        let id = do_create_client(
            &conn,
            "Acme Corp",
            Some("security@acme.com"),
            Some("Main client"),
            Some(&vec!["fintech".to_string()]),
            Some(&vec!["nginx".to_string(), "wordpress".to_string()]),
        )
        .unwrap();

        let client = do_get_client(&conn, id).unwrap();
        assert_eq!(client.name, "Acme Corp");
        assert_eq!(client.contact_email, Some("security@acme.com".to_string()));
        assert_eq!(client.tags, vec!["fintech"]);
        assert_eq!(client.tech_stack, vec!["nginx", "wordpress"]);
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let conn = test_conn();
        do_create_client(&conn, "Acme Corp", None, None, None, None).unwrap();
        let result = do_create_client(&conn, "Acme Corp", None, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_soft_delete_and_list() {
        let conn = test_conn();
        let id = do_create_client(&conn, "Acme Corp", None, None, None, None).unwrap();
        let list = do_list_clients(&conn, None).unwrap();
        assert_eq!(list.len(), 1);

        do_delete_client(&conn, id).unwrap();
        let list_after = do_list_clients(&conn, None).unwrap();
        assert_eq!(list_after.len(), 0);

        let get_result = do_get_client(&conn, id);
        assert!(get_result.is_err());
    }

    #[test]
    fn test_search_clients() {
        let conn = test_conn();
        do_create_client(
            &conn,
            "Acme Corp",
            Some("a@b.com"),
            None,
            Some(&vec!["fintech".to_string()]),
            None,
        )
        .unwrap();
        do_create_client(&conn, "Wayne Enterprises", None, None, None, None).unwrap();

        assert_eq!(do_list_clients(&conn, Some("Acme")).unwrap().len(), 1);
        assert_eq!(do_list_clients(&conn, Some("a@b")).unwrap().len(), 1);
        assert_eq!(do_list_clients(&conn, Some("fintech")).unwrap().len(), 1);
        assert_eq!(do_list_clients(&conn, Some("zzzz")).unwrap().len(), 0);
    }

    #[test]
    fn test_dashboard_stats() {
        let conn = test_conn();
        let stats = do_get_dashboard_stats(&conn).unwrap();
        assert_eq!(stats.client_count, 0);
        assert_eq!(stats.finding_count, 0);
        assert!(stats.findings_ready);

        do_create_client(&conn, "Acme Corp", None, None, None, None).unwrap();
        let stats = do_get_dashboard_stats(&conn).unwrap();
        assert_eq!(stats.client_count, 1);
    }

    #[test]
    fn test_update_client() {
        let conn = test_conn();
        let id =
            do_create_client(&conn, "Acme Corp", Some("old@acme.com"), None, None, None).unwrap();

        do_update_client(
            &conn,
            id,
            Some("Acme Inc"),
            Some("new@acme.com"),
            None,
            None,
            None,
        )
        .unwrap();

        let client = do_get_client(&conn, id).unwrap();
        assert_eq!(client.name, "Acme Inc");
        assert_eq!(client.contact_email, Some("new@acme.com".to_string()));
    }

    #[test]
    fn test_client_mutations_write_audit_snapshots() {
        let conn = test_conn();
        let id =
            do_create_client(&conn, "Acme Corp", Some("old@acme.com"), None, None, None).unwrap();
        do_update_client(
            &conn,
            id,
            Some("Acme Inc"),
            Some("new@acme.com"),
            None,
            None,
            None,
        )
        .unwrap();
        do_delete_client(&conn, id).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT action, old_value, new_value
                 FROM audit_log
                 WHERE table_name = 'clients'
                 ORDER BY id",
            )
            .unwrap();
        let rows: Vec<(String, Option<String>, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "INSERT");
        assert!(rows[0].1.is_none());
        let created: serde_json::Value =
            serde_json::from_str(rows[0].2.as_deref().unwrap()).unwrap();
        assert_eq!(created["name"], "Acme Corp");

        assert_eq!(rows[1].0, "UPDATE");
        let update_old: serde_json::Value =
            serde_json::from_str(rows[1].1.as_deref().unwrap()).unwrap();
        let update_new: serde_json::Value =
            serde_json::from_str(rows[1].2.as_deref().unwrap()).unwrap();
        assert_eq!(update_old["name"], "Acme Corp");
        assert_eq!(update_new["name"], "Acme Inc");

        assert_eq!(rows[2].0, "DELETE");
        let delete_old: serde_json::Value =
            serde_json::from_str(rows[2].1.as_deref().unwrap()).unwrap();
        let delete_new: serde_json::Value =
            serde_json::from_str(rows[2].2.as_deref().unwrap()).unwrap();
        assert_eq!(delete_old["name"], "Acme Inc");
        assert_eq!(delete_new["is_active"], 0);
    }
}
