use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct RetestEngagement {
    pub id: u32,
    pub original_engagement_id: u32,
    pub original_name: String,
    pub name: String,
    pub client_name: String,
    pub status: String,
    pub created_at: i64,
}

fn row_to_retest(row: &rusqlite::Row) -> Result<RetestEngagement, rusqlite::Error> {
    Ok(RetestEngagement {
        id: row.get(0)?,
        original_engagement_id: row.get(1)?,
        original_name: row.get(2)?,
        name: row.get(3)?,
        client_name: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
    })
}

#[must_use]
pub fn do_create_retest_engagement(
    conn: &Connection,
    original_engagement_id: u32,
) -> crate::error::Result<u32> {
    // Verify original exists
    let original: Option<(u32, String, String, String, String)> = conn
        .query_row(
            "SELECT e.client_id, e.name, c.short_name, e.target_area, e.assessment_kind
             FROM engagements e JOIN clients c ON c.id = e.client_id
             WHERE e.id = ? AND e.is_active = 1",
            params![original_engagement_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()
        .map_err(AppError::from)?;

    let (client_id, orig_name, _client_name, target_area, assessment_kind) =
        original.ok_or(AppError::Generic("Original engagement not found.".to_string()))?;

    let retest_name = format!("Retest: {orig_name}");

    conn.execute(
        "INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model, engagement_type, status, original_engagement_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'Not Applicable', 'Retest', 'planned', ?5, strftime('%s', 'now'), strftime('%s', 'now'))",
        params![client_id, &retest_name, target_area, assessment_kind, original_engagement_id],
    )
    .map_err(AppError::from)?;

    let new_id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| AppError::Generic("ID overflow".to_string()))?;

    // Clone findings from original engagement that are reported/fixed/accepted
    let mut stmt = conn
        .prepare(
            "SELECT title, severity, cvss_score, owasp_category, cwe_id, overview, summary,
             affected_endpoints, evidence, impact_items, remediation_items, steps_to_reproduce,
             references_json, tags, notes
             FROM findings WHERE engagement_id = ? AND is_active = 1
             AND status IN ('reported', 'fixed', 'accepted')",
        )
        .map_err(AppError::from)?;

    let rows = stmt
        .query_map(params![original_engagement_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, Option<String>>(14)?,
            ))
        })
        .map_err(AppError::from)?;

    for row in rows {
        let (
            title,
            severity,
            cvss,
            owasp,
            cwe,
            overview,
            summary,
            endpoints,
            evidence,
            impact,
            remediation,
            steps,
            refs,
            tags,
            notes,
        ) = row.map_err(AppError::from)?;
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, cvss_score, owasp_category, cwe_id,
             overview, summary, affected_endpoints, evidence, impact_items, remediation_items,
             steps_to_reproduce, references_json, status, tags, notes, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'draft', ?15, ?16, strftime('%s', 'now'))",
            params![
                new_id, title, severity, cvss, owasp.as_deref(), cwe.as_deref(),
                overview, summary, endpoints, evidence, impact, remediation, steps, refs, tags, notes.as_deref(),
            ],
        )
        .map_err(AppError::from)?;
    }

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["engagements", "RETEST_CREATED", new_id, "", "", format!("original_engagement_id={original_engagement_id}")],
    )
    .map_err(AppError::from)?;

    Ok(new_id)
}

#[must_use]
pub fn do_list_retest_engagements(
    conn: &Connection,
    original_engagement_id: u32,
) -> crate::error::Result<Vec<RetestEngagement>> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.original_engagement_id, oe.name, e.name, c.short_name, e.status, e.created_at
             FROM engagements e
             JOIN engagements oe ON oe.id = e.original_engagement_id
             JOIN clients c ON c.id = e.client_id
             WHERE e.original_engagement_id = ? AND e.is_active = 1
             ORDER BY e.created_at DESC",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![original_engagement_id], row_to_retest)
        .map_err(AppError::from)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(AppError::from)?);
    }
    Ok(items)
}

#[must_use]
pub fn do_get_retest_comparison(
    conn: &Connection,
    retest_engagement_id: u32,
) -> crate::error::Result<Vec<serde_json::Value>> {
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.title, f.severity, f.status, f.retest_result, f.retest_notes,
             f.original_finding_id, of.title as original_title, of.severity as original_severity, of.status as original_status
             FROM findings f
             LEFT JOIN findings of ON of.id = f.original_finding_id
             WHERE f.engagement_id = ? AND f.is_active = 1
             ORDER BY f.severity, f.title",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![retest_engagement_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, u32>(0)?,
                "title": row.get::<_, String>(1)?,
                "severity": row.get::<_, String>(2)?,
                "status": row.get::<_, String>(3)?,
                "retest_result": row.get::<_, String>(4)?,
                "retest_notes": row.get::<_, Option<String>>(5)?,
                "original_finding_id": row.get::<_, Option<u32>>(6)?,
                "original_title": row.get::<_, Option<String>>(7)?,
                "original_severity": row.get::<_, Option<String>>(8)?,
                "original_status": row.get::<_, Option<String>>(9)?,
            }))
        })
        .map_err(AppError::from)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(AppError::from)?);
    }
    Ok(items)
}

#[must_use]
pub fn do_bulk_update_finding_status(
    conn: &Connection,
    finding_ids: &[u32],
    client_response: &str,
) -> crate::error::Result<()> {
    let valid = [
        "acknowledged",
        "in_progress",
        "fixed",
        "accepted_risk",
        "disputed",
        "deferred",
        "no_response",
    ];
    if !valid.contains(&client_response) {
        return Err(AppError::Generic(format!("Invalid client_response: {client_response}")));
    }
    for id in finding_ids {
        conn.execute(
            "UPDATE findings SET client_response = ?, updated_at = strftime('%s', 'now') WHERE id = ?",
            params![client_response, id],
        )
        .map_err(AppError::from)?;
    }
    Ok(())
}

#[must_use]
pub fn do_get_overdue_findings(conn: &Connection) -> crate::error::Result<Vec<serde_json::Value>> {
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.title, f.fix_deadline, f.severity, e.name as engagement_name, c.name as client_name
             FROM findings f
             JOIN engagements e ON e.id = f.engagement_id
             JOIN clients c ON c.id = e.client_id
             WHERE f.is_active = 1 AND f.fix_deadline IS NOT NULL
             AND f.fix_deadline < date('now')
             AND f.client_response NOT IN ('fixed', 'accepted_risk')
             ORDER BY f.fix_deadline",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, u32>(0)?,
                "title": row.get::<_, String>(1)?,
                "fix_deadline": row.get::<_, Option<String>>(2)?,
                "severity": row.get::<_, String>(3)?,
                "engagement_name": row.get::<_, String>(4)?,
                "client_name": row.get::<_, String>(5)?,
            }))
        })
        .map_err(AppError::from)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(AppError::from)?);
    }
    Ok(items)
}

// Tauri commands
#[tauri::command]
pub fn create_retest_engagement(
    state: State<AppState>,
    original_engagement_id: u32,
) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_retest_engagement(conn, original_engagement_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_retest_engagements(
    state: State<AppState>,
    original_engagement_id: u32,
) -> Result<Vec<RetestEngagement>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_retest_engagements(conn, original_engagement_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_retest_comparison(
    state: State<AppState>,
    retest_engagement_id: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_retest_comparison(conn, retest_engagement_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bulk_update_finding_status(
    state: State<AppState>,
    finding_ids: Vec<u32>,
    client_response: String,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_bulk_update_finding_status(conn, &finding_ids, &client_response).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_overdue_findings(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_overdue_findings(conn).map_err(|e| e.to_string())
}
