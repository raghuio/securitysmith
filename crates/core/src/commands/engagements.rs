use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct Engagement {
    pub id: u32,
    pub client_id: u32,
    pub client_name: String,
    pub name: String,
    pub target_area: String,
    pub assessment_kind: String,
    pub access_model: String,
    pub engagement_type: String,
    pub status: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub scope_summary: Option<String>,
    pub objectives: Vec<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub credentials_ready: bool,
    pub payment_required: bool,
    pub payment_cleared: bool,
    pub budgeted_hours: Option<f64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize, Clone)]
pub struct EngagementInput {
    pub client_id: u32,
    pub name: String,
    pub target_area: String,
    pub assessment_kind: String,
    pub access_model: String,
    pub engagement_type: String,
    pub status: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub scope_summary: Option<String>,
    pub objectives: Option<Vec<String>>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub payment_required: Option<bool>,
    pub budgeted_hours: Option<f64>,
}

fn parse_json_list(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn row_to_engagement(row: &rusqlite::Row) -> Result<Engagement, rusqlite::Error> {
    let objectives_str: String = row.get(12)?;
    let tags_str: String = row.get(14)?;
    Ok(Engagement {
        id: row.get(0)?,
        client_id: row.get(1)?,
        client_name: row.get(2)?,
        name: row.get(3)?,
        target_area: row.get(4)?,
        assessment_kind: row.get(5)?,
        access_model: row.get(6)?,
        engagement_type: row.get(7)?,
        status: row.get(8)?,
        start_date: row.get(9)?,
        end_date: row.get(10)?,
        scope_summary: row.get(11)?,
        objectives: parse_json_list(&objectives_str),
        notes: row.get(13)?,
        tags: parse_json_list(&tags_str),
        credentials_ready: row.get(15)?,
        payment_required: row.get(16)?,
        payment_cleared: row.get(17)?,
        budgeted_hours: row.get(20)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

// ─────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────

fn validate_status(status: &str) -> Result<(), String> {
    match status {
        "planned" | "scheduled" | "active" | "paused" | "completed" => Ok(()),
        _ => Err(format!(
            "Invalid status '{}'. Must be one of: planned, scheduled, active, paused, completed.",
            status
        )),
    }
}

fn validate_date(date: &str) -> Result<(), String> {
    if date.len() != 10 {
        return Err(format!("Date '{}' must be YYYY-MM-DD.", date));
    }
    if date.chars().nth(4) != Some('-') || date.chars().nth(7) != Some('-') {
        return Err(format!("Date '{}' must be YYYY-MM-DD.", date));
    }
    Ok(())
}

fn validate_dates(start_date: Option<&str>, end_date: Option<&str>) -> Result<(), String> {
    if let Some(s) = start_date {
        validate_date(s)?;
    }
    if let Some(e) = end_date {
        validate_date(e)?;
    }
    if let (Some(s), Some(e)) = (start_date, end_date)
        && e < s
    {
        return Err("End date cannot be before start date.".to_string());
    }
    Ok(())
}

fn validate_input(input: &EngagementInput) -> Result<(), String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("Engagement name is required.".to_string());
    }
    if name.len() > 255 {
        return Err("Engagement name must be 255 characters or fewer.".to_string());
    }

    let validate_text = |value: &str, max: usize, label: &str| -> Result<(), String> {
        let v = value.trim();
        if v.is_empty() {
            return Err(format!("{} is required.", label));
        }
        if v.len() > max {
            return Err(format!("{} must be {} characters or fewer.", label, max));
        }
        Ok(())
    };

    validate_text(&input.target_area, 80, "Target area")?;
    validate_text(&input.assessment_kind, 80, "Assessment kind")?;
    validate_text(&input.access_model, 80, "Access model")?;
    validate_text(&input.engagement_type, 160, "Engagement type")?;

    validate_status(&input.status)?;
    validate_dates(input.start_date.as_deref(), input.end_date.as_deref())?;

    if let Some(ref scope) = input.scope_summary
        && scope.len() > 5_000
    {
        return Err("Scope summary must be 5,000 characters or fewer.".to_string());
    }

    if let Some(ref notes) = input.notes
        && notes.len() > 20_000
    {
        return Err("Notes must be 20,000 characters or fewer.".to_string());
    }

    if let Some(ref objectives) = input.objectives {
        for o in objectives {
            if o.len() > 500 {
                return Err("Each objective must be 500 characters or fewer.".to_string());
            }
        }
    }

    if let Some(ref tags) = input.tags {
        for t in tags {
            if t.len() > 64 {
                return Err("Each tag must be 64 characters or fewer.".to_string());
            }
        }
    }

    Ok(())
}

fn client_exists_and_active(conn: &Connection, client_id: u32) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM clients WHERE id = ?1 AND is_active = 1",
        params![client_id],
        |_| Ok(true),
    )
    .optional()
    .map_err(|e| format!("Database error: {}", e))
    .map(|v| v.unwrap_or(false))
}

fn are_gates_enabled(conn: &Connection) -> Result<bool, String> {
    let enabled: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params!["engagement.gates_enabled"],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;
    // Default to true (gates enabled) if setting not present
    Ok(enabled.as_deref() != Some("false"))
}

fn get_engagement_row(conn: &Connection, id: u32) -> Result<Option<Engagement>, String> {
    conn.query_row(
        "SELECT
            e.id, e.client_id, c.short_name as client_name, e.name,
            e.target_area, e.assessment_kind, e.access_model, e.engagement_type,
            e.status, e.start_date, e.end_date, e.scope_summary,
            e.objectives, e.notes, e.tags, e.credentials_ready,
            e.payment_required, e.payment_cleared, e.created_at, e.updated_at,
            e.budgeted_hours
         FROM engagements e
         JOIN clients c ON c.id = e.client_id
         WHERE e.id = ?1 AND e.is_active = 1",
        params![id],
        row_to_engagement,
    )
    .optional()
    .map_err(|e| format!("Database error: {}", e))
}

#[must_use]
pub fn do_create_engagement(conn: &Connection, input: &EngagementInput) -> Result<u32, String> {
    validate_input(input)?;

    if !client_exists_and_active(conn, input.client_id)? {
        return Err("Client not found or is not active.".to_string());
    }

    let objectives_json = serde_json::to_string(&input.objectives.clone().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize objectives: {}", e))?;
    let tags_json = serde_json::to_string(&input.tags.clone().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;

    let payment_required = input.payment_required.unwrap_or(false) as i32;
    let budgeted_hours = input.budgeted_hours;

    conn.execute(
        "INSERT INTO engagements
         (client_id, name, target_area, assessment_kind, access_model,
          engagement_type, status, start_date, end_date, scope_summary,
          objectives, notes, tags, payment_required, budgeted_hours, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, strftime('%s', 'now'))",
        params![
            input.client_id,
            input.name.trim(),
            input.target_area.trim(),
            input.assessment_kind.trim(),
            input.access_model.trim(),
            input.engagement_type.trim(),
            input.status.trim(),
            input.start_date.as_deref(),
            input.end_date.as_deref(),
            input.scope_summary.as_deref(),
            objectives_json,
            input.notes.as_deref(),
            tags_json,
            payment_required,
            budgeted_hours,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            "An engagement with this name already exists for this client.".to_string()
        } else {
            format!("Failed to create engagement: {}", e)
        }
    })?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    let new_engagement = do_get_engagement(conn, id)?;
    let new_json = serde_json::to_string(&new_engagement)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "engagements",
            "INSERT",
            &id.to_string(),
            None::<&str>,
            &new_json,
            "create_engagement command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Update the global search index (PROP-028)
    crate::commands::search::do_update_search_index_for_entity(conn, "engagement", id)
        .map_err(|e| format!("Search index update failed: {}", e))?;

    Ok(id)
}

fn do_get_engagement(conn: &Connection, id: u32) -> Result<Engagement, String> {
    let engagement = get_engagement_row(conn, id)?;
    engagement.ok_or("Engagement not found.".to_string())
}

fn do_update_engagement(conn: &Connection, id: u32, input: &EngagementInput) -> Result<(), String> {
    validate_input(input)?;

    if !client_exists_and_active(conn, input.client_id)? {
        return Err("Client not found or is not active.".to_string());
    }

    let old = get_engagement_row(conn, id)?;
    let old = old.ok_or("Engagement not found.".to_string())?;

    let objectives_json = serde_json::to_string(&input.objectives.clone().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize objectives: {}", e))?;
    let tags_json = serde_json::to_string(&input.tags.clone().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;

    let payment_required = input.payment_required.unwrap_or(old.payment_required) as i32;
    let budgeted_hours = input.budgeted_hours.or(old.budgeted_hours);

    conn.execute(
        "UPDATE engagements SET
            client_id = ?1,
            name = ?2,
            target_area = ?3,
            assessment_kind = ?4,
            access_model = ?5,
            engagement_type = ?6,
            status = ?7,
            start_date = ?8,
            end_date = ?9,
            scope_summary = ?10,
            objectives = ?11,
            notes = ?12,
            tags = ?13,
            payment_required = ?14,
            budgeted_hours = ?15,
            updated_at = strftime('%s', 'now')
         WHERE id = ?16",
        params![
            input.client_id,
            input.name.trim(),
            input.target_area.trim(),
            input.assessment_kind.trim(),
            input.access_model.trim(),
            input.engagement_type.trim(),
            input.status.trim(),
            input.start_date.as_deref(),
            input.end_date.as_deref(),
            input.scope_summary.as_deref(),
            objectives_json,
            input.notes.as_deref(),
            tags_json,
            payment_required,
            budgeted_hours,
            id,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            "An engagement with this name already exists for this client.".to_string()
        } else {
            format!("Failed to update engagement: {}", e)
        }
    })?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_engagement = do_get_engagement(conn, id)?;
    let new_json = serde_json::to_string(&new_engagement)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "engagements",
            "UPDATE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_engagement command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Update the global search index (PROP-028)
    crate::commands::search::do_update_search_index_for_entity(conn, "engagement", id)
        .map_err(|e| format!("Search index update failed: {}", e))?;

    Ok(())
}

fn do_archive_engagement(conn: &Connection, id: u32) -> Result<(), String> {
    let old = get_engagement_row(conn, id)?;
    let old = old.ok_or("Engagement not found.".to_string())?;

    conn.execute(
        "UPDATE engagements SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?1",
        params![id],
    )
    .map_err(|e| format!("Failed to archive engagement: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_json = serde_json::json!({"id": id, "is_active": 0}).to_string();

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "engagements",
            "DELETE",
            &id.to_string(),
            &old_json,
            &new_json,
            "archive_engagement command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Remove from global search index (PROP-028)
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = 'engagement' AND entity_id = ?1",
        params![id],
    )
    .map_err(|e| format!("Search index removal failed: {}", e))?;

    Ok(())
}

/// Transition engagement status with gate checks.
/// planned → scheduled requires gates to pass (if enabled).
fn do_transition_status(conn: &Connection, id: u32, new_status: &str) -> Result<(), String> {
    validate_status(new_status)?;

    let old = get_engagement_row(conn, id)?;
    let old = old.ok_or("Engagement not found.".to_string())?;

    let old_status = old.status.as_str();

    // Valid transitions
    let valid = match (old_status, new_status) {
        ("planned", "scheduled")
        | ("planned", "active")
        | ("scheduled", "active")
        | ("active", "paused")
        | ("paused", "active")
        | ("active", "completed")
        | ("scheduled", "completed")
        | ("paused", "completed")
        | ("planned", "completed") => true,
        (a, b) if a == b => true,
        _ => false,
    };

    if !valid {
        return Err(format!(
            "Invalid status transition: {} → {}. Allowed: planned→scheduled/active/completed, scheduled→active/completed, active→paused/completed, paused→active/completed.",
            old_status, new_status
        ));
    }

    // Gate checks for planned → scheduled
    if old_status == "planned" && new_status == "scheduled" {
        let gates_enabled = are_gates_enabled(conn)?;
        if gates_enabled {
            if !old.credentials_ready {
                return Err(
                    "Credentials gate not passed. All test credentials must be verified as working before scheduling."
                        .to_string(),
                );
            }
            if old.payment_required && !old.payment_cleared {
                return Err(
                    "Payment gate not passed. The advance payment must be received before scheduling."
                        .to_string(),
                );
            }
        }
    }

    conn.execute(
        "UPDATE engagements SET status = ?1, updated_at = strftime('%s', 'now') WHERE id = ?2",
        params![new_status, id],
    )
    .map_err(|e| format!("Failed to update status: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_engagement = do_get_engagement(conn, id)?;
    let new_json = serde_json::to_string(&new_engagement)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "engagements",
            "STATUS_CHANGE",
            &id.to_string(),
            &old_json,
            &new_json,
            "transition_engagement_status command"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(())
}

/// Toggle a gate field on an engagement.
fn do_toggle_gate(conn: &Connection, id: u32, gate: &str, value: bool) -> Result<(), String> {
    let old = get_engagement_row(conn, id)?;
    let old = old.ok_or("Engagement not found.".to_string())?;

    let (column, old_val) = match gate {
        "credentials_ready" => ("credentials_ready", old.credentials_ready),
        "payment_cleared" => ("payment_cleared", old.payment_cleared),
        _ => {
            return Err(format!(
                "Unknown gate '{}'. Must be 'credentials_ready' or 'payment_cleared'.",
                gate
            ));
        }
    };

    if old_val == value {
        return Ok(()); // No change needed
    }

    let sql = format!(
        "UPDATE engagements SET {} = ?1, updated_at = strftime('%s', 'now') WHERE id = ?2",
        column
    );
    conn.execute(&sql, params![value as i32, id])
        .map_err(|e| format!("Failed to toggle gate: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_engagement = do_get_engagement(conn, id)?;
    let new_json = serde_json::to_string(&new_engagement)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "engagements",
            "GATE_TOGGLE",
            &id.to_string(),
            &old_json,
            &new_json,
            &format!("toggle_engagement_gate: {} → {}", gate, value)
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(())
}

fn do_list_engagements(
    conn: &Connection,
    client_id: Option<u32>,
    search: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<Engagement>, String> {
    let mut sql = String::from(
        "SELECT
            e.id, e.client_id, c.short_name as client_name, e.name,
            e.target_area, e.assessment_kind, e.access_model, e.engagement_type,
            e.status, e.start_date, e.end_date, e.scope_summary,
            e.objectives, e.notes, e.tags, e.credentials_ready,
            e.payment_required, e.payment_cleared, e.created_at, e.updated_at,
            e.budgeted_hours
         FROM engagements e
         JOIN clients c ON c.id = e.client_id
         WHERE e.is_active = 1",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(cid) = client_id {
        sql.push_str(" AND e.client_id = ?1");
        params_vec.push(Box::new(cid as i64));
    }

    if let Some(s) = status {
        let next = (params_vec.len() + 1).to_string();
        sql.push_str(&format!(" AND e.status = ?{}", next));
        params_vec.push(Box::new(s.to_string()));
    }

    if let Some(s) = search {
        let term = format!("%{}%", s.trim());
        let base = params_vec.len() + 1;
        let p1 = base;
        let p2 = base + 1;
        let p3 = base + 2;
        let p4 = base + 3;
        let p5 = base + 4;
        let p6 = base + 5;
        sql.push_str(&format!(
            " AND (e.name LIKE ?{} OR e.target_area LIKE ?{} OR e.assessment_kind LIKE ?{} OR e.access_model LIKE ?{} OR e.engagement_type LIKE ?{} OR c.short_name LIKE ?{})",
            p1, p2, p3, p4, p5, p6
        ));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term));
    }

    sql.push_str(" ORDER BY e.updated_at DESC");

    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Database error: {}", e))?;
    let results = stmt
        .query_map(&*param_refs, row_to_engagement)
        .map_err(|e| format!("Database error: {}", e))?
        .collect::<Result<Vec<Engagement>, _>>()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(results)
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_engagement(state: State<AppState>, input: EngagementInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_engagement(conn, &input)
}

#[tauri::command]
pub fn get_engagement(state: State<AppState>, id: u32) -> Result<Engagement, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_engagement(conn, id)
}

#[tauri::command]
pub fn update_engagement(
    state: State<AppState>,
    id: u32,
    input: EngagementInput,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_update_engagement(conn, id, &input)
}

#[tauri::command]
pub fn archive_engagement(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_archive_engagement(conn, id)
}

#[tauri::command]
pub fn list_engagements(
    state: State<AppState>,
    client_id: Option<u32>,
    search: Option<String>,
    status: Option<String>,
) -> Result<Vec<Engagement>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_engagements(conn, client_id, search.as_deref(), status.as_deref())
}

#[tauri::command]
pub fn transition_engagement_status(
    state: State<AppState>,
    id: u32,
    new_status: String,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_transition_status(conn, id, &new_status)
}

#[tauri::command]
pub fn toggle_engagement_gate(
    state: State<AppState>,
    id: u32,
    gate: String,
    value: bool,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_toggle_gate(conn, id, &gate, value)
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::clients::do_create_client;
    use crate::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    fn make_input(client_id: u32, name: &str, status: &str) -> EngagementInput {
        EngagementInput {
            client_id,
            name: name.to_string(),
            target_area: "Web".to_string(),
            assessment_kind: "Pentest".to_string(),
            access_model: "Authenticated".to_string(),
            engagement_type: name.to_string(),
            status: status.to_string(),
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

    #[test]
    fn test_create_and_get_engagement() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1 Pentest", "planned");
        let id = do_create_engagement(&conn, &input).unwrap();

        let e = do_get_engagement(&conn, id).unwrap();
        assert_eq!(e.name, "Q1 Pentest");
        assert_eq!(e.status, "planned");
        assert_eq!(e.client_name, "Acme");
        assert!(!e.payment_required);
        assert!(!e.payment_cleared);
    }

    #[test]
    fn test_duplicate_name_per_client_rejected() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1 Pentest", "planned");
        do_create_engagement(&conn, &input).unwrap();

        let result = do_create_engagement(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_same_name_across_clients_allowed() {
        let conn = test_conn();
        let cid1 = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let cid2 = do_create_client(&conn, "Wayne", "Wayne Enterprises", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input1 = make_input(cid1, "Q1 Pentest", "planned");
        let input2 = make_input(cid2, "Q1 Pentest", "planned");
        do_create_engagement(&conn, &input1).unwrap();
        do_create_engagement(&conn, &input2).unwrap();
    }

    #[test]
    fn test_archive_hides_from_default_list() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1 Pentest", "planned");
        let id = do_create_engagement(&conn, &input).unwrap();

        let list = do_list_engagements(&conn, None, None, None).unwrap();
        assert_eq!(list.len(), 1);

        do_archive_engagement(&conn, id).unwrap();
        let after = do_list_engagements(&conn, None, None, None).unwrap();
        assert_eq!(after.len(), 0);
    }

    #[test]
    fn test_list_filters() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let mut input1 = make_input(cid, "Web Pentest", "planned");
        input1.target_area = "Web".to_string();
        input1.assessment_kind = "Pentest".to_string();
        let mut input2 = make_input(cid, "API Review", "active");
        input2.target_area = "API".to_string();
        input2.assessment_kind = "Security Review".to_string();
        input2.access_model = "Mixed".to_string();

        do_create_engagement(&conn, &input1).unwrap();
        do_create_engagement(&conn, &input2).unwrap();

        let all = do_list_engagements(&conn, None, None, None).unwrap();
        assert_eq!(all.len(), 2);

        let by_status = do_list_engagements(&conn, None, None, Some("active")).unwrap();
        assert_eq!(by_status.len(), 1);
        assert_eq!(by_status[0].name, "API Review");

        let by_search = do_list_engagements(&conn, None, Some("Web"), None).unwrap();
        assert_eq!(by_search.len(), 1);
        assert_eq!(by_search[0].name, "Web Pentest");
    }

    #[test]
    fn test_invalid_status_rejected() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let mut input = make_input(cid, "Q1", "planned");
        input.status = "invalid".to_string();
        let result = do_create_engagement(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid status"));
    }

    #[test]
    fn test_scheduled_status_accepted() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let mut input = make_input(cid, "Q1", "scheduled");
        input.status = "scheduled".to_string();
        let id = do_create_engagement(&conn, &input).unwrap();
        let e = do_get_engagement(&conn, id).unwrap();
        assert_eq!(e.status, "scheduled");
    }

    #[test]
    fn test_invalid_client_rejected() {
        let conn = test_conn();
        let input = make_input(9999, "Q1 Pentest", "planned");
        let result = do_create_engagement(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Client not found"));
    }

    #[test]
    fn test_date_validation() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let mut input = make_input(cid, "Q1", "planned");
        input.start_date = Some("bad-date".to_string());
        let result = do_create_engagement(&conn, &input);
        assert!(result.is_err());

        input.start_date = Some("2025-01-01".to_string());
        input.end_date = Some("2024-12-31".to_string());
        let result = do_create_engagement(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("End date cannot be before"));
    }

    #[test]
    fn test_audit_snapshots() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1", "planned");
        let id = do_create_engagement(&conn, &input).unwrap();

        let mut update = input.clone();
        update.name = "Q1 Updated".to_string();
        update.status = "active".to_string();
        do_update_engagement(&conn, id, &update).unwrap();
        do_archive_engagement(&conn, id).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT action, old_value, new_value
                 FROM audit_log
                 WHERE table_name = 'engagements'
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

        assert_eq!(rows[1].0, "UPDATE");
        let old_json: serde_json::Value =
            serde_json::from_str(rows[1].1.as_deref().unwrap()).unwrap();
        let new_json: serde_json::Value =
            serde_json::from_str(rows[1].2.as_deref().unwrap()).unwrap();
        assert_eq!(old_json["name"], "Q1");
        assert_eq!(new_json["name"], "Q1 Updated");

        assert_eq!(rows[2].0, "DELETE");
        let delete_new: serde_json::Value =
            serde_json::from_str(rows[2].2.as_deref().unwrap()).unwrap();
        assert_eq!(delete_new["is_active"], 0);
    }

    #[test]
    fn test_transition_status_gates_blocked() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let mut input = make_input(cid, "Q1", "planned");
        input.payment_required = Some(true);
        let id = do_create_engagement(&conn, &input).unwrap();

        // Gates enabled by default — blocked because credentials_ready=false and payment_cleared=false
        let result = do_transition_status(&conn, id, "scheduled");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Credentials gate not passed"));

        // Toggle credentials_ready
        do_toggle_gate(&conn, id, "credentials_ready", true).unwrap();
        let result = do_transition_status(&conn, id, "scheduled");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Payment gate not passed"));

        // Toggle payment_cleared
        do_toggle_gate(&conn, id, "payment_cleared", true).unwrap();
        do_transition_status(&conn, id, "scheduled").unwrap();

        let e = do_get_engagement(&conn, id).unwrap();
        assert_eq!(e.status, "scheduled");
    }

    #[test]
    fn test_transition_status_gates_disabled() {
        let conn = test_conn();
        // Disable gates globally
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params!["engagement.gates_enabled", "false"],
        )
        .unwrap();

        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1", "planned");
        let id = do_create_engagement(&conn, &input).unwrap();

        // Gates disabled — should succeed even with credentials_ready=false
        do_transition_status(&conn, id, "scheduled").unwrap();
        let e = do_get_engagement(&conn, id).unwrap();
        assert_eq!(e.status, "scheduled");
    }

    #[test]
    fn test_invalid_status_transition() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1", "active");
        let id = do_create_engagement(&conn, &input).unwrap();

        let result = do_transition_status(&conn, id, "planned");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid status transition"));
    }

    #[test]
    fn test_scheduled_to_active_transition() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1", "scheduled");
        let id = do_create_engagement(&conn, &input).unwrap();

        do_transition_status(&conn, id, "active").unwrap();
        let e = do_get_engagement(&conn, id).unwrap();
        assert_eq!(e.status, "active");
    }

    #[test]
    fn test_gate_toggle_audit() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1", "planned");
        let id = do_create_engagement(&conn, &input).unwrap();

        do_toggle_gate(&conn, id, "credentials_ready", true).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT action, context FROM audit_log WHERE table_name = 'engagements' ORDER BY id DESC LIMIT 1",
            )
            .unwrap();
        let row: (String, String) = stmt
            .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        assert_eq!(row.0, "GATE_TOGGLE");
        assert!(row.1.contains("credentials_ready"));
    }

    #[test]
    fn test_payment_required_default_false() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let input = make_input(cid, "Q1", "planned");
        let id = do_create_engagement(&conn, &input).unwrap();
        let e = do_get_engagement(&conn, id).unwrap();
        assert!(!e.payment_required);
    }

    #[test]
    fn test_payment_required_explicit_true() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", "Acme", None, None, None, None, None, None, None, None, None, None, None).unwrap();
        let mut input = make_input(cid, "Q1", "planned");
        input.payment_required = Some(true);
        let id = do_create_engagement(&conn, &input).unwrap();
        let e = do_get_engagement(&conn, id).unwrap();
        assert!(e.payment_required);
    }
}
