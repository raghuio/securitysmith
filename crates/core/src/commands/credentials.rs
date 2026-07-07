use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct Credential {
    pub id: u32,
    pub engagement_id: u32,
    pub label: String,
    pub credential_type: String,
    pub value: String,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Metadata only — used for audit logging (value excluded)
#[derive(Serialize)]
pub struct CredentialMeta {
    pub id: u32,
    pub engagement_id: u32,
    pub label: String,
    pub credential_type: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CredentialInput {
    pub engagement_id: u32,
    pub label: String,
    pub credential_type: String,
    pub value: String,
    pub notes: Option<String>,
}

fn row_to_credential(row: &rusqlite::Row) -> Result<Credential, rusqlite::Error> {
    Ok(Credential {
        id: row.get(0)?,
        engagement_id: row.get(1)?,
        label: row.get(2)?,
        credential_type: row.get(3)?,
        value: row.get(4)?,
        notes: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn to_meta(cred: &Credential) -> CredentialMeta {
    CredentialMeta {
        id: cred.id,
        engagement_id: cred.engagement_id,
        label: cred.label.clone(),
        credential_type: cred.credential_type.clone(),
        status: cred.status.clone(),
    }
}

// ─────────────────────────────────────────────────────────────
// Validation helpers
// ─────────────────────────────────────────────────────────────

fn validate_status(status: &str) -> Result<(), String> {
    match status {
        "not_verified" | "working" | "not_working" | "expired" => Ok(()),
        _ => Err(format!(
            "Invalid status '{}'. Must be one of: not_verified, working, not_working, expired.",
            status
        )),
    }
}

fn validate_input(input: &CredentialInput) -> Result<(), String> {
    let label = input.label.trim();
    if label.is_empty() {
        return Err("Label is required.".to_string());
    }
    if label.len() > 255 {
        return Err("Label must be 255 characters or fewer.".to_string());
    }
    let ctype = input.credential_type.trim();
    if ctype.is_empty() {
        return Err("Credential type is required.".to_string());
    }
    if ctype.len() > 80 {
        return Err("Credential type must be 80 characters or fewer.".to_string());
    }
    if input.value.is_empty() {
        return Err("Credential value is required.".to_string());
    }
    if input.value.len() > 50_000 {
        return Err("Credential value exceeds maximum size of 50KB.".to_string());
    }
    if let Some(ref notes) = input.notes
        && notes.len() > 5_000
    {
        return Err("Notes must be 5,000 characters or fewer.".to_string());
    }
    Ok(())
}

fn engagement_exists_and_active(conn: &Connection, engagement_id: u32) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM engagements WHERE id = ?1 AND is_active = 1",
        params![engagement_id],
        |_| Ok(true),
    )
    .optional()
    .map_err(|e| format!("Database error: {}", e))
    .map(|v| v.unwrap_or(false))
}

// ─────────────────────────────────────────────────────────────
// Gate recalculation
// ─────────────────────────────────────────────────────────────

fn recalculate_credentials_gate(conn: &Connection, engagement_id: u32) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM credentials WHERE engagement_id = ?1",
            params![engagement_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?;

    let all_working: bool = if count == 0 {
        false
    } else {
        conn.query_row(
            "SELECT COUNT(*) = SUM(CASE WHEN status = 'working' THEN 1 ELSE 0 END)
             FROM credentials WHERE engagement_id = ?1",
            params![engagement_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?
    };

    conn.execute(
        "UPDATE engagements SET credentials_ready = ?1, updated_at = strftime('%s', 'now') WHERE id = ?2",
        params![all_working as i32, engagement_id],
    )
    .map_err(|e| format!("Failed to update engagement gate: {}", e))?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────

fn do_create_credential(conn: &Connection, input: &CredentialInput) -> Result<u32, String> {
    validate_input(input)?;

    if !engagement_exists_and_active(conn, input.engagement_id)? {
        return Err("Engagement not found or has been archived.".to_string());
    }

    conn.execute(
        "INSERT INTO credentials
         (engagement_id, label, credential_type, value, notes, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s', 'now'))",
        params![
            input.engagement_id,
            input.label.trim(),
            input.credential_type.trim(),
            input.value,
            input.notes.as_deref(),
        ],
    )
    .map_err(|e| format!("Failed to create credential: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    let new_cred = do_get_credential(conn, id)?;
    let meta = to_meta(&new_cred);
    let meta_json = serde_json::to_string(&meta)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "credentials",
            "INSERT",
            &id.to_string(),
            None::<&str>,
            &meta_json,
            "create_credential command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Update the global search index (PROP-028) — best-effort
    let _ = crate::commands::search::do_update_search_index_for_entity(conn, "credential", id);

    recalculate_credentials_gate(conn, input.engagement_id)?;

    Ok(id)
}

fn do_get_credential(conn: &Connection, id: u32) -> Result<Credential, String> {
    let cred: Option<Credential> = conn
        .query_row(
            "SELECT id, engagement_id, label, credential_type, value, notes, status, created_at, updated_at
             FROM credentials WHERE id = ?1",
            params![id],
            row_to_credential,
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    cred.ok_or("Credential not found.".to_string())
}

#[derive(Deserialize)]
pub struct CredentialUpdate {
    pub label: Option<String>,
    pub credential_type: Option<String>,
    pub value: Option<String>,
    pub notes: Option<String>,
    pub status: Option<String>,
}

fn do_update_credential(
    conn: &Connection,
    id: u32,
    update: &CredentialUpdate,
) -> Result<(), String> {
    let old = do_get_credential(conn, id)?;

    let label = update.label.as_deref().unwrap_or(&old.label).trim();
    if label.is_empty() {
        return Err("Label is required.".to_string());
    }
    if label.len() > 255 {
        return Err("Label must be 255 characters or fewer.".to_string());
    }

    let ctype = update
        .credential_type
        .as_deref()
        .unwrap_or(&old.credential_type)
        .trim();
    if ctype.is_empty() {
        return Err("Credential type is required.".to_string());
    }
    if ctype.len() > 80 {
        return Err("Credential type must be 80 characters or fewer.".to_string());
    }

    let value = update.value.as_deref().unwrap_or(&old.value);
    if value.is_empty() {
        return Err("Credential value is required.".to_string());
    }
    if value.len() > 50_000 {
        return Err("Credential value exceeds maximum size of 50KB.".to_string());
    }

    if let Some(ref s) = update.status {
        validate_status(s)?;
    }

    if let Some(ref notes) = update.notes
        && notes.len() > 5_000
    {
        return Err("Notes must be 5,000 characters or fewer.".to_string());
    }

    let status = update.status.as_deref().unwrap_or(&old.status);

    conn.execute(
        "UPDATE credentials SET
            label = ?1,
            credential_type = ?2,
            value = ?3,
            notes = ?4,
            status = ?5,
            updated_at = strftime('%s', 'now')
         WHERE id = ?6",
        params![label, ctype, value, update.notes.as_deref(), status, id],
    )
    .map_err(|e| format!("Failed to update credential: {}", e))?;

    let new_cred = do_get_credential(conn, id)?;
    let old_meta = to_meta(&old);
    let new_meta = to_meta(&new_cred);
    let old_json = serde_json::to_string(&old_meta)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_json = serde_json::to_string(&new_meta)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "credentials",
            "UPDATE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_credential command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Update the global search index (PROP-028) — best-effort
    let _ = crate::commands::search::do_update_search_index_for_entity(conn, "credential", id);

    recalculate_credentials_gate(conn, old.engagement_id)?;

    Ok(())
}

fn do_delete_credential(conn: &Connection, id: u32) -> Result<(), String> {
    let old = do_get_credential(conn, id)?;
    let engagement_id = old.engagement_id;

    let old_meta = to_meta(&old);
    let old_json = serde_json::to_string(&old_meta)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    conn.execute("DELETE FROM credentials WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete credential: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "credentials",
            "DELETE",
            &id.to_string(),
            &old_json,
            None::<&str>,
            "delete_credential command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Remove from global search index (PROP-028) — best-effort
    let _ = conn.execute(
        "DELETE FROM search_index WHERE entity_type = 'credential' AND entity_id = ?1",
        params![id],
    );

    recalculate_credentials_gate(conn, engagement_id)?;

    Ok(())
}

fn do_list_credentials(conn: &Connection, engagement_id: u32) -> Result<Vec<Credential>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, engagement_id, label, credential_type, value, notes, status, created_at, updated_at
             FROM credentials WHERE engagement_id = ?1 ORDER BY updated_at DESC"
        )
        .map_err(|e| format!("Database error: {}", e))?;
    let results = stmt
        .query_map(params![engagement_id], row_to_credential)
        .map_err(|e| format!("Database error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(results)
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_credential(state: State<AppState>, input: CredentialInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_credential(conn, &input)
}

#[tauri::command]
pub fn get_credential(state: State<AppState>, id: u32) -> Result<Credential, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_credential(conn, id)
}

#[tauri::command]
pub fn update_credential(
    state: State<AppState>,
    id: u32,
    update: CredentialUpdate,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_update_credential(conn, id, &update)
}

#[tauri::command]
pub fn delete_credential(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_delete_credential(conn, id)
}

#[tauri::command]
pub fn list_credentials(
    state: State<AppState>,
    engagement_id: u32,
) -> Result<Vec<Credential>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_credentials(conn, engagement_id)
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::clients::do_create_client;
    use crate::commands::engagements::{EngagementInput, do_create_engagement};
    use crate::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    fn make_engagement_input(client_id: u32, name: &str) -> EngagementInput {
        EngagementInput {
            client_id,
            name: name.to_string(),
            target_area: "Web".to_string(),
            assessment_kind: "Pentest".to_string(),
            access_model: "Authenticated".to_string(),
            engagement_type: "Web Pentest".to_string(),
            status: "planned".to_string(),
            start_date: None,
            end_date: None,
            scope_summary: None,
            objectives: None,
            notes: None,
            tags: None,
            payment_required: None,
            budgeted_hours: None,
        }
    }

    fn make_cred_input(engagement_id: u32, label: &str, value: &str) -> CredentialInput {
        CredentialInput {
            engagement_id,
            label: label.to_string(),
            credential_type: "username_password".to_string(),
            value: value.to_string(),
            notes: None,
        }
    }

    #[test]
    fn test_create_and_get_credential() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_cred_input(eid, "Admin login", "secret");
        let id = do_create_credential(&conn, &input).unwrap();

        let cred = do_get_credential(&conn, id).unwrap();
        assert_eq!(cred.label, "Admin login");
        assert_eq!(cred.value, "secret");
        assert_eq!(cred.status, "not_verified");
    }

    #[test]
    fn test_reject_missing_engagement() {
        let conn = test_conn();
        let input = make_cred_input(9999, "Admin", "secret");
        let result = do_create_credential(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("archived"));
    }

    #[test]
    fn test_gate_all_working() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let id = do_create_credential(&conn, &make_cred_input(eid, "Admin", "s1")).unwrap();

        // Initially not_verified → gate false
        let gate_before: bool = conn
            .query_row(
                "SELECT credentials_ready FROM engagements WHERE id = ?1",
                params![eid],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(!gate_before);

        // Mark working → gate true
        do_update_credential(
            &conn,
            id,
            &CredentialUpdate {
                label: None,
                credential_type: None,
                value: None,
                notes: None,
                status: Some("working".to_string()),
            },
        )
        .unwrap();

        let gate_after: bool = conn
            .query_row(
                "SELECT credentials_ready FROM engagements WHERE id = ?1",
                params![eid],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(gate_after);
    }

    #[test]
    fn test_gate_one_expired_resets() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let id1 = do_create_credential(&conn, &make_cred_input(eid, "Admin", "s1")).unwrap();
        let id2 = do_create_credential(&conn, &make_cred_input(eid, "API key", "s2")).unwrap();

        // Mark both working → gate true
        do_update_credential(
            &conn,
            id1,
            &CredentialUpdate {
                label: None,
                credential_type: None,
                value: None,
                notes: None,
                status: Some("working".to_string()),
            },
        )
        .unwrap();
        do_update_credential(
            &conn,
            id2,
            &CredentialUpdate {
                label: None,
                credential_type: None,
                value: None,
                notes: None,
                status: Some("working".to_string()),
            },
        )
        .unwrap();

        let gate_working: bool = conn
            .query_row(
                "SELECT credentials_ready FROM engagements WHERE id = ?1",
                params![eid],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(gate_working);

        // Mark one expired → gate false
        do_update_credential(
            &conn,
            id2,
            &CredentialUpdate {
                label: None,
                credential_type: None,
                value: None,
                notes: None,
                status: Some("expired".to_string()),
            },
        )
        .unwrap();

        let gate_expired: bool = conn
            .query_row(
                "SELECT credentials_ready FROM engagements WHERE id = ?1",
                params![eid],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(!gate_expired);
    }

    #[test]
    fn test_delete_resets_gate() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let id = do_create_credential(&conn, &make_cred_input(eid, "Admin", "s1")).unwrap();
        do_update_credential(
            &conn,
            id,
            &CredentialUpdate {
                label: None,
                credential_type: None,
                value: None,
                notes: None,
                status: Some("working".to_string()),
            },
        )
        .unwrap();

        let gate_before: bool = conn
            .query_row(
                "SELECT credentials_ready FROM engagements WHERE id = ?1",
                params![eid],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(gate_before);

        do_delete_credential(&conn, id).unwrap();

        let gate_after: bool = conn
            .query_row(
                "SELECT credentials_ready FROM engagements WHERE id = ?1",
                params![eid],
                |row| row.get::<_, bool>(0),
            )
            .unwrap();
        assert!(!gate_after);
    }

    #[test]
    fn test_audit_excludes_value() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_cred_input(eid, "Admin", "super-secret-value");
        let _id = do_create_credential(&conn, &input).unwrap();

        let mut stmt = conn
            .prepare("SELECT new_value FROM audit_log WHERE table_name = 'credentials' AND action = 'INSERT'")
            .unwrap();
        let new_value: Option<String> = stmt.query_row([], |row| row.get(0)).optional().unwrap();

        let json = new_value.unwrap();
        assert!(json.contains("Admin"));
        assert!(!json.contains("super-secret-value"));
    }

    #[test]
    fn test_value_too_large_rejected() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let mut input = make_cred_input(eid, "SSH", "x");
        input.value = "x".repeat(50_001);
        let result = do_create_credential(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("50KB"));
    }
}
