use securitysmith_report_engine::{ReportData, generate_pdf};
use securitysmith_core::state::AppState;
use rusqlite::OptionalExtension;
use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct Report {
    pub id: u32,
    pub engagement_id: u32,
    pub engagement_name: String,
    pub client_name: String,
    pub name: String,
    pub executive_summary: String,
    pub appendix: String,
    pub included_finding_ids: Vec<u32>,
    pub status: String,
    pub generated_at: Option<i64>,
    pub file_path: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_to_report(row: &rusqlite::Row) -> Result<Report, rusqlite::Error> {
    let ids_str: String = row.get(5)?;
    Ok(Report {
        id: row.get(0)?,
        engagement_id: row.get(1)?,
        engagement_name: row.get(2)?,
        client_name: row.get(3)?,
        name: row.get(4)?,
        included_finding_ids: serde_json::from_str(&ids_str).unwrap_or_default(),
        executive_summary: row.get(6)?,
        appendix: row.get(7)?,
        status: row.get(8)?,
        generated_at: row.get(9)?,
        file_path: row.get(10)?,
        is_active: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn do_list_reports(conn: &Connection, engagement_id: Option<u32>) -> Result<Vec<Report>, String> {
    let sql = if engagement_id.is_some() {
        "SELECT r.id, r.engagement_id, e.name, c.name, r.name, r.included_finding_ids, r.executive_summary, r.appendix, r.status, r.generated_at, r.file_path, r.is_active, r.created_at, r.updated_at
         FROM reports r JOIN engagements e ON r.engagement_id = e.id JOIN clients c ON e.client_id = c.id
         WHERE r.is_active = 1 AND r.engagement_id = ? ORDER BY r.updated_at DESC"
    } else {
        "SELECT r.id, r.engagement_id, e.name, c.name, r.name, r.included_finding_ids, r.executive_summary, r.appendix, r.status, r.generated_at, r.file_path, r.is_active, r.created_at, r.updated_at
         FROM reports r JOIN engagements e ON r.engagement_id = e.id JOIN clients c ON e.client_id = c.id
         WHERE r.is_active = 1 ORDER BY r.updated_at DESC"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Failed to prepare: {e}"))?;
    let rows: Vec<Report> = if let Some(eid) = engagement_id {
        stmt.query_map(params![eid], row_to_report)
            .map_err(|e| format!("Query failed: {e}"))?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map([], row_to_report)
            .map_err(|e| format!("Query failed: {e}"))?
            .filter_map(|r| r.ok())
            .collect()
    };
    Ok(rows)
}

fn do_get_report(conn: &Connection, id: u32) -> Result<Report, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.engagement_id, e.name, c.name, r.name, r.included_finding_ids, r.executive_summary, r.appendix, r.status, r.generated_at, r.file_path, r.is_active, r.created_at, r.updated_at
             FROM reports r JOIN engagements e ON r.engagement_id = e.id JOIN clients c ON e.client_id = c.id
             WHERE r.id = ? AND r.is_active = 1"
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let item: Option<Report> = stmt
        .query_map(params![id], row_to_report)
        .map_err(|e| format!("Query failed: {e}"))?
        .next()
        .transpose()
        .map_err(|e| format!("Row parse failed: {e}"))?;
    item.ok_or_else(|| "Report not found.".to_string())
}

fn do_create_report(
    conn: &Connection,
    engagement_id: u32,
    name: &str,
    executive_summary: &str,
    appendix: &str,
    included_ids: &[u32],
) -> Result<u32, String> {
    let ids_json =
        serde_json::to_string(included_ids).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO reports (engagement_id, name, executive_summary, appendix, included_finding_ids, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'draft', strftime('%s', 'now'), strftime('%s', 'now'))",
        params![engagement_id, name, executive_summary, appendix, ids_json],
    )
    .map_err(|e| format!("Failed to create report: {e}"))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    let new = do_get_report(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["reports", "CREATE", id, "", new_json],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;
    Ok(id)
}

fn do_update_report(
    conn: &Connection,
    id: u32,
    name: Option<&str>,
    executive_summary: Option<&str>,
    appendix: Option<&str>,
    included_ids: Option<&[u32]>,
) -> Result<(), String> {
    let old = do_get_report(conn, id)?;
    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {e}"))?;

    let mut updates: Vec<(&str, rusqlite::types::Value)> = Vec::new();
    if let Some(n) = name {
        updates.push(("name = ?", n.to_string().into()));
    }
    if let Some(e) = executive_summary {
        updates.push(("executive_summary = ?", e.to_string().into()));
    }
    if let Some(a) = appendix {
        updates.push(("appendix = ?", a.to_string().into()));
    }
    if let Some(ids) = included_ids {
        let json = serde_json::to_string(ids).map_err(|e| format!("Serialize failed: {e}"))?;
        updates.push(("included_finding_ids = ?", json.to_string().into()));
    }
    if updates.is_empty() {
        return Ok(());
    }

    let set_clause = updates
        .iter()
        .map(|(col, _)| *col)
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE reports SET {}, updated_at = strftime('%s', 'now') WHERE id = ?",
        set_clause
    );

    let mut params_list: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for (_, v) in &updates {
        params_list.push(v);
    }
    params_list.push(&id);

    conn.execute(&sql, rusqlite::params_from_iter(params_list))
        .map_err(|e| format!("Update failed: {e}"))?;

    let new = do_get_report(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["reports", "UPDATE", id, old_json, new_json],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;
    Ok(())
}

fn do_archive_report(conn: &Connection, id: u32) -> Result<(), String> {
    let old = do_get_report(conn, id)?;
    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "UPDATE reports SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Archive failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["reports", "ARCHIVE", id, old_json, ""],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn list_reports(
    state: State<AppState>,
    engagement_id: Option<u32>,
) -> Result<Vec<Report>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_list_reports(conn, engagement_id)
}

#[tauri::command]
pub fn get_report(state: State<AppState>, id: u32) -> Result<Report, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_get_report(conn, id)
}

#[tauri::command]
pub fn create_report(
    state: State<AppState>,
    engagement_id: u32,
    name: String,
    executive_summary: String,
    appendix: String,
    included_finding_ids: Vec<u32>,
) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_create_report(
        conn,
        engagement_id,
        &name,
        &executive_summary,
        &appendix,
        &included_finding_ids,
    )
}

#[tauri::command]
pub fn update_report(
    state: State<AppState>,
    id: u32,
    name: Option<String>,
    executive_summary: Option<String>,
    appendix: Option<String>,
    included_finding_ids: Option<Vec<u32>>,
) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_update_report(
        conn,
        id,
        name.as_deref(),
        executive_summary.as_deref(),
        appendix.as_deref(),
        included_finding_ids.as_deref(),
    )
}

#[tauri::command]
pub fn archive_report(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_archive_report(conn, id)
}

#[tauri::command]
pub fn generate_report_pdf(
    state: State<AppState>,
    report_id: u32,
    save_path: String,
) -> Result<String, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;

    let report = do_get_report(conn, report_id)?;

    let mut finding_titles: Vec<String> = Vec::new();
    for fid in &report.included_finding_ids {
        let title: Option<String> = conn
            .query_row("SELECT title FROM findings WHERE id = ?", [*fid], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| format!("DB: {e}"))?;
        if let Some(t) = title {
            finding_titles.push(t);
        }
    }

    let mut data = ReportData::new(
        report.name,
        report.client_name,
        report.engagement_name,
    );
    data.set_executive_summary(report.executive_summary);
    data.set_appendix(report.appendix);
    data.set_finding_titles(finding_titles);

    generate_pdf(&data, &save_path)?;

    // Update report metadata
    conn.execute(
        "UPDATE reports SET status = 'generated', generated_at = strftime('%s', 'now'), file_path = ? WHERE id = ?",
        params![&save_path, report_id],
    )
    .map_err(|e| format!("Update report: {e}"))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["reports", "PDF_GENERATED", report_id, "", "", format!("path={save_path}")],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;

    Ok(save_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use securitysmith_core::db;

    fn conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let key = [11u8; 32];
        let c = db::open_vault(tmp.path(), &key).unwrap();
        db::init_db(&c).unwrap();
        c.execute("INSERT INTO clients (name) VALUES ('Acme')", [])
            .unwrap();
        c.execute("INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model, engagement_type, status, start_date) VALUES (1, 'Pentest', 'Web', 'pentest', 'remote', 'one-time', 'active', '2026-01-01')", []).unwrap();
        c
    }

    #[test]
    fn test_report_crud() {
        let c = conn();
        let id = do_create_report(
            &c,
            1,
            "My Report",
            "Summary text.",
            "Appendix text.",
            &[1, 2],
        )
        .unwrap();
        let r = do_get_report(&c, id).unwrap();
        assert_eq!(r.name, "My Report");
        assert_eq!(r.included_finding_ids, vec![1, 2]);
        do_update_report(&c, id, Some("Updated"), None, None, None).unwrap();
        let r2 = do_get_report(&c, id).unwrap();
        assert_eq!(r2.name, "Updated");
        do_archive_report(&c, id).unwrap();
        assert!(do_get_report(&c, id).is_err());
        let list = do_list_reports(&c, Some(1)).unwrap();
        assert!(list.is_empty());
    }
}
