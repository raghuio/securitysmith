use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

// ─────────────────────────────────────────────────────────────
// JSON sub-structs
// ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AffectedEndpoint {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Evidence {
    pub title: String,
    pub request: String,
    pub response: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ImpactItem {
    pub title: String,
    pub explanation: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RemediationItem {
    pub action: String,
    pub fix: String,
    pub code_snippet: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Reference {
    pub title: String,
    pub url: String,
}

// ─────────────────────────────────────────────────────────────
// Main Finding struct
// ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct Finding {
    pub id: u32,
    pub engagement_id: u32,
    pub engagement_name: String,
    pub client_name: String,
    pub title: String,
    pub severity: String,
    pub cvss_score: Option<f32>,
    pub owasp_category: Option<String>,
    pub cwe_id: Option<String>,
    pub overview: String,
    pub summary: String,
    pub affected_endpoints: Vec<AffectedEndpoint>,
    pub evidence: Vec<Evidence>,
    pub impact_items: Vec<ImpactItem>,
    pub remediation_items: Vec<RemediationItem>,
    pub steps_to_reproduce: String,
    pub references: Vec<Reference>,
    pub status: String,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
pub struct FindingPage {
    pub items: Vec<Finding>,
    pub total: u32,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Serialize)]
pub struct FindingCounts {
    pub total: u32,
    pub by_severity: HashMap<String, u32>,
}

// ─────────────────────────────────────────────────────────────
// Input struct
// ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct FindingInput {
    pub engagement_id: u32,
    pub title: String,
    pub severity: String,
    pub overview: String,
    pub summary: String,
    pub affected_endpoints: Vec<AffectedEndpoint>,
    pub evidence: Vec<Evidence>,
    pub impact_items: Vec<ImpactItem>,
    pub remediation_items: Vec<RemediationItem>,
    pub steps_to_reproduce: String,
    pub cvss_score: Option<f32>,
    pub owasp_category: Option<String>,
    pub cwe_id: Option<String>,
    pub references: Option<Vec<Reference>>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

// ─────────────────────────────────────────────────────────────
// JSON helpers
// ─────────────────────────────────────────────────────────────

fn to_json<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| format!("Failed to serialize JSON: {}", e))
}

fn parse_json<T: for<'de> Deserialize<'de>>(s: &str) -> Vec<T> {
    serde_json::from_str(s).unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────
// Row mapping
// ─────────────────────────────────────────────────────────────

fn row_to_finding(row: &rusqlite::Row) -> Result<Finding, rusqlite::Error> {
    Ok(Finding {
        id: row.get(0)?,
        engagement_id: row.get(1)?,
        engagement_name: row.get(2)?,
        client_name: row.get(3)?,
        title: row.get(4)?,
        severity: row.get(5)?,
        cvss_score: row.get(6)?,
        owasp_category: row.get(7)?,
        cwe_id: row.get(8)?,
        overview: row.get(9)?,
        summary: row.get(10)?,
        affected_endpoints: parse_json(row.get::<_, String>(11)?.as_str()),
        evidence: parse_json(row.get::<_, String>(12)?.as_str()),
        impact_items: parse_json(row.get::<_, String>(13)?.as_str()),
        remediation_items: parse_json(row.get::<_, String>(14)?.as_str()),
        steps_to_reproduce: row.get(15)?,
        references: parse_json(row.get::<_, String>(16)?.as_str()),
        status: row.get(17)?,
        tags: parse_json(row.get::<_, String>(18)?.as_str()),
        notes: row.get(19)?,
        is_active: row.get::<_, i32>(20)? != 0,
        created_at: row.get(21)?,
        updated_at: row.get(22)?,
    })
}

// ─────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────

fn validate_severity(s: &str) -> Result<(), String> {
    match s {
        "critical" | "high" | "medium" | "low" | "informational" => Ok(()),
        _ => Err(
            "Invalid severity. Must be: critical, high, medium, low, or informational.".to_string(),
        ),
    }
}

fn validate_status(s: &str) -> Result<(), String> {
    match s {
        "draft" | "confirmed" | "reported" | "fixed" | "accepted" | "false_positive" | "wont_fix" => Ok(()),
        _ => Err("Invalid status. Must be: draft, confirmed, reported, fixed, accepted, false_positive, or wont_fix.".to_string()),
    }
}

fn validate_finding(input: &FindingInput) -> Result<(), String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err("Title is required.".to_string());
    }
    if title.len() > 500 {
        return Err("Title must be 500 characters or fewer.".to_string());
    }

    validate_severity(&input.severity)?;

    let overview = input.overview.trim();
    if overview.is_empty() {
        return Err("Overview is required.".to_string());
    }
    if overview.len() > 1_000 {
        return Err("Overview must be 1,000 characters or fewer.".to_string());
    }

    let summary = input.summary.trim();
    if summary.is_empty() {
        return Err("Summary is required.".to_string());
    }
    if summary.len() > 50_000 {
        return Err("Summary must be 50,000 characters or fewer.".to_string());
    }

    if input.affected_endpoints.is_empty() {
        return Err("At least one affected endpoint is required.".to_string());
    }
    if input.affected_endpoints.len() > 100 {
        return Err("Maximum 100 affected endpoints per finding.".to_string());
    }
    for ep in &input.affected_endpoints {
        if ep.method.len() > 32 {
            return Err("Endpoint method must be 32 characters or fewer.".to_string());
        }
        if ep.path.len() > 2_000 {
            return Err("Endpoint path must be 2,000 characters or fewer.".to_string());
        }
    }

    if input.evidence.is_empty() {
        return Err("At least one evidence entry is required.".to_string());
    }
    if input.evidence.len() > 50 {
        return Err("Maximum 50 evidence entries per finding.".to_string());
    }
    for ev in &input.evidence {
        let entry_len = ev.title.len() + ev.request.len() + ev.response.len();
        if entry_len > 100_000 {
            return Err("Evidence entry exceeds 100KB limit.".to_string());
        }
        if ev.title.len() > 255 {
            return Err("Evidence title must be 255 characters or fewer.".to_string());
        }
    }

    if input.impact_items.is_empty() {
        return Err("At least one impact item is required.".to_string());
    }
    if input.impact_items.len() > 20 {
        return Err("Maximum 20 impact items per finding.".to_string());
    }

    if input.remediation_items.is_empty() {
        return Err("At least one remediation item is required.".to_string());
    }
    if input.remediation_items.len() > 20 {
        return Err("Maximum 20 remediation items per finding.".to_string());
    }

    let steps = input.steps_to_reproduce.trim();
    if steps.is_empty() {
        return Err("Steps to reproduce are required.".to_string());
    }
    if steps.len() > 50_000 {
        return Err("Steps to reproduce must be 50,000 characters or fewer.".to_string());
    }

    if let Some(score) = input.cvss_score
        && !(0.0..=10.0).contains(&score)
    {
        return Err("CVSS score must be between 0.0 and 10.0.".to_string());
    }

    if let Some(ref cat) = input.owasp_category
        && cat.len() > 80
    {
        return Err("OWASP category must be 80 characters or fewer.".to_string());
    }

    if let Some(ref cwe) = input.cwe_id
        && cwe.len() > 20
    {
        return Err("CWE ID must be 20 characters or fewer.".to_string());
    }

    if let Some(ref refs) = input.references {
        if refs.len() > 50 {
            return Err("Maximum 50 references per finding.".to_string());
        }
        for r in refs {
            if r.title.len() > 255 {
                return Err("Reference title must be 255 characters or fewer.".to_string());
            }
            if r.url.len() > 4_000 {
                return Err("Reference URL must be 4,000 characters or fewer.".to_string());
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

    if let Some(ref notes) = input.notes
        && notes.len() > 20_000
    {
        return Err("Notes must be 20,000 characters or fewer.".to_string());
    }

    Ok(())
}

fn engagement_exists_and_active(conn: &Connection, id: u32) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM engagements WHERE id = ?1 AND is_active = 1",
        params![id],
        |_| Ok(true),
    )
    .optional()
    .map_err(|e| format!("Database error: {}", e))
    .map(|v| v.unwrap_or(false))
}

// ─────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────

fn do_create_finding(conn: &Connection, input: &FindingInput) -> Result<u32, String> {
    validate_finding(input)?;

    if !engagement_exists_and_active(conn, input.engagement_id)? {
        return Err("Engagement not found or has been archived.".to_string());
    }

    let endpoints_json = to_json(&input.affected_endpoints)?;
    let evidence_json = to_json(&input.evidence)?;
    let impact_json = to_json(&input.impact_items)?;
    let remediation_json = to_json(&input.remediation_items)?;
    let refs_json = to_json(&input.references.clone().unwrap_or_default())?;
    let tags_json = to_json(&input.tags.clone().unwrap_or_default())?;

    conn.execute(
        "INSERT INTO findings
         (engagement_id, title, severity, cvss_score, owasp_category, cwe_id,
          overview, summary, affected_endpoints, evidence, impact_items,
          remediation_items, steps_to_reproduce, references_json, status,
          tags, notes, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, strftime('%s', 'now'))",
        params![
            input.engagement_id,
            input.title.trim(),
            input.severity.trim(),
            input.cvss_score,
            input.owasp_category.as_deref(),
            input.cwe_id.as_deref(),
            input.overview.trim(),
            input.summary.trim(),
            endpoints_json,
            evidence_json,
            impact_json,
            remediation_json,
            input.steps_to_reproduce.trim(),
            refs_json,
            "draft",
            tags_json,
            input.notes.as_deref(),
        ],
    )
    .map_err(|e| format!("Failed to create finding: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    let new_finding = do_get_finding(conn, id)?;
    let new_json = serde_json::to_string(&new_finding)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "findings",
            "INSERT",
            &id.to_string(),
            None::<&str>,
            &new_json,
            "create_finding command",
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(id)
}

fn do_get_finding(conn: &Connection, id: u32) -> Result<Finding, String> {
    let finding: Option<Finding> = conn
        .query_row(
            "SELECT
                f.id, f.engagement_id, e.name as engagement_name, c.name as client_name,
                f.title, f.severity, f.cvss_score, f.owasp_category, f.cwe_id,
                f.overview, f.summary, f.affected_endpoints, f.evidence,
                f.impact_items, f.remediation_items, f.steps_to_reproduce,
                f.references_json, f.status, f.tags, f.notes, f.is_active,
                f.created_at, f.updated_at
             FROM findings f
             JOIN engagements e ON e.id = f.engagement_id
             JOIN clients c ON c.id = e.client_id
             WHERE f.id = ?1 AND f.is_active = 1",
            params![id],
            row_to_finding,
        )
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    finding.ok_or("Finding not found.".to_string())
}

fn do_update_finding(conn: &Connection, id: u32, input: &FindingInput) -> Result<(), String> {
    validate_finding(input)?;

    if !engagement_exists_and_active(conn, input.engagement_id)? {
        return Err("Engagement not found or has been archived.".to_string());
    }

    let old = do_get_finding(conn, id)?;

    let endpoints_json = to_json(&input.affected_endpoints)?;
    let evidence_json = to_json(&input.evidence)?;
    let impact_json = to_json(&input.impact_items)?;
    let remediation_json = to_json(&input.remediation_items)?;
    let refs_json = to_json(&input.references.clone().unwrap_or_default())?;
    let tags_json = to_json(&input.tags.clone().unwrap_or_default())?;

    conn.execute(
        "UPDATE findings SET
            engagement_id = ?1, title = ?2, severity = ?3, cvss_score = ?4,
            owasp_category = ?5, cwe_id = ?6, overview = ?7, summary = ?8,
            affected_endpoints = ?9, evidence = ?10, impact_items = ?11,
            remediation_items = ?12, steps_to_reproduce = ?13, references_json = ?14,
            tags = ?15, notes = ?16, updated_at = strftime('%s', 'now')
         WHERE id = ?17 AND is_active = 1",
        params![
            input.engagement_id,
            input.title.trim(),
            input.severity.trim(),
            input.cvss_score,
            input.owasp_category.as_deref(),
            input.cwe_id.as_deref(),
            input.overview.trim(),
            input.summary.trim(),
            endpoints_json,
            evidence_json,
            impact_json,
            remediation_json,
            input.steps_to_reproduce.trim(),
            refs_json,
            tags_json,
            input.notes.as_deref(),
            id,
        ],
    )
    .map_err(|e| format!("Failed to update finding: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_finding = do_get_finding(conn, id)?;
    let new_json = serde_json::to_string(&new_finding)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "findings",
            "UPDATE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_finding command",
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(())
}

fn do_update_finding_status(conn: &Connection, id: u32, status: &str) -> Result<(), String> {
    validate_status(status)?;

    let old = do_get_finding(conn, id)?;

    conn.execute(
        "UPDATE findings SET status = ?1, updated_at = strftime('%s', 'now') WHERE id = ?2 AND is_active = 1",
        params![status.trim(), id],
    )
    .map_err(|e| format!("Failed to update finding status: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_finding = do_get_finding(conn, id)?;
    let new_json = serde_json::to_string(&new_finding)
        .map_err(|e| format!("Failed to serialize new audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "findings",
            "STATUS_CHANGE",
            &id.to_string(),
            &old_json,
            &new_json,
            "update_finding_status command",
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    Ok(())
}

fn do_duplicate_finding(conn: &Connection, id: u32) -> Result<u32, String> {
    let original = do_get_finding(conn, id)?;

    let new_title = format!("[COPY] {}", original.title);
    let input = FindingInput {
        engagement_id: original.engagement_id,
        title: new_title,
        severity: original.severity.clone(),
        overview: original.overview.clone(),
        summary: original.summary.clone(),
        affected_endpoints: original.affected_endpoints.clone(),
        evidence: original.evidence.clone(),
        impact_items: original.impact_items.clone(),
        remediation_items: original.remediation_items.clone(),
        steps_to_reproduce: original.steps_to_reproduce.clone(),
        cvss_score: original.cvss_score,
        owasp_category: original.owasp_category.clone(),
        cwe_id: original.cwe_id.clone(),
        references: Some(original.references.clone()),
        tags: Some(original.tags.clone()),
        notes: original.notes.clone(),
    };

    do_create_finding(conn, &input)
}

fn do_archive_finding(conn: &Connection, id: u32) -> Result<(), String> {
    let old = do_get_finding(conn, id)?;

    conn.execute(
        "UPDATE findings SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?1",
        params![id],
    )
    .map_err(|e| format!("Failed to archive finding: {}", e))?;

    let old_json = serde_json::to_string(&old)
        .map_err(|e| format!("Failed to serialize old audit value: {}", e))?;
    let new_json = serde_json::json!({"id": id, "is_active": 0}).to_string();

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "findings",
            "ARCHIVE",
            &id.to_string(),
            &old_json,
            &new_json,
            "archive_finding command",
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Remove from global search index (PROP-028)
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = 'finding' AND entity_id = ?1",
        params![id],
    )
    .map_err(|e| format!("Search index removal failed: {}", e))?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn do_list_findings(
    conn: &Connection,
    engagement_id: Option<u32>,
    client_id: Option<u32>,
    search: Option<&str>,
    severity: Option<&str>,
    status: Option<&str>,
    owasp_category: Option<&str>,
    offset: u32,
    limit: u32,
) -> Result<FindingPage, String> {
    let max_limit = limit.clamp(1, 500);

    let mut count_sql = String::from(
        "SELECT COUNT(*) FROM findings f
         JOIN engagements e ON e.id = f.engagement_id
         JOIN clients c ON c.id = e.client_id
         WHERE f.is_active = 1",
    );
    let mut items_sql = String::from(
        "SELECT
            f.id, f.engagement_id, e.name as engagement_name, c.name as client_name,
            f.title, f.severity, f.cvss_score, f.owasp_category, f.cwe_id,
            f.overview, f.summary, f.affected_endpoints, f.evidence,
            f.impact_items, f.remediation_items, f.steps_to_reproduce,
            f.references_json, f.status, f.tags, f.notes, f.is_active,
            f.created_at, f.updated_at
         FROM findings f
         JOIN engagements e ON e.id = f.engagement_id
         JOIN clients c ON c.id = e.client_id
         WHERE f.is_active = 1",
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(eid) = engagement_id {
        count_sql.push_str(" AND f.engagement_id = ?");
        items_sql.push_str(" AND f.engagement_id = ?");
        params_vec.push(Box::new(eid as i64));
    }

    if let Some(cid) = client_id {
        count_sql.push_str(" AND e.client_id = ?");
        items_sql.push_str(" AND e.client_id = ?");
        params_vec.push(Box::new(cid as i64));
    }

    if let Some(s) = severity {
        count_sql.push_str(" AND f.severity = ?");
        items_sql.push_str(" AND f.severity = ?");
        params_vec.push(Box::new(s.to_string()));
    }

    if let Some(s) = status {
        count_sql.push_str(" AND f.status = ?");
        items_sql.push_str(" AND f.status = ?");
        params_vec.push(Box::new(s.to_string()));
    }

    if let Some(cat) = owasp_category {
        count_sql.push_str(" AND f.owasp_category = ?");
        items_sql.push_str(" AND f.owasp_category = ?");
        params_vec.push(Box::new(cat.to_string()));
    }

    if let Some(s) = search {
        let term = format!("%{}%", s.trim());
        count_sql.push_str(" AND (f.title LIKE ? OR f.owasp_category LIKE ? OR f.tags LIKE ? OR c.name LIKE ? OR e.name LIKE ?)");
        items_sql.push_str(" AND (f.title LIKE ? OR f.owasp_category LIKE ? OR f.tags LIKE ? OR c.name LIKE ? OR e.name LIKE ?)");
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term.clone()));
        params_vec.push(Box::new(term));
    }

    items_sql.push_str(" ORDER BY f.created_at DESC LIMIT ? OFFSET ?");

    // count_refs must be a separate collection since params_vec gets moved later
    let mut count_refs: Vec<&dyn rusqlite::ToSql> = Vec::new();
    for b in &params_vec {
        count_refs.push(b.as_ref());
    }

    let total: u32 = conn
        .query_row(&count_sql, &*count_refs, |row| row.get(0))
        .map_err(|e| format!("Database error: {}", e))?;

    params_vec.push(Box::new(max_limit as i64));
    params_vec.push(Box::new(offset as i64));

    let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn
        .prepare(&items_sql)
        .map_err(|e| format!("Database error: {}", e))?;
    let items = stmt
        .query_map(&*param_refs, row_to_finding)
        .map_err(|e| format!("Database error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(FindingPage {
        items,
        total,
        offset,
        limit: max_limit,
    })
}

fn do_get_finding_counts(conn: &Connection) -> Result<FindingCounts, String> {
    let total: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE is_active = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT severity, COUNT(*) FROM findings WHERE is_active = 1 GROUP BY severity")
        .map_err(|e| format!("Database error: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            let sev: String = row.get(0)?;
            let count: u32 = row.get(1)?;
            Ok((sev, count))
        })
        .map_err(|e| format!("Database error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Database error: {}", e))?;

    let by_severity = rows.into_iter().collect::<HashMap<String, u32>>();

    Ok(FindingCounts { total, by_severity })
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_finding(state: State<AppState>, input: FindingInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;
    do_create_finding(conn, &input)
}

#[tauri::command]
pub fn get_finding(state: State<AppState>, id: u32) -> Result<Finding, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;
    do_get_finding(conn, id)
}

#[tauri::command]
pub fn update_finding(state: State<AppState>, id: u32, input: FindingInput) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;
    do_update_finding(conn, id, &input)
}

#[tauri::command]
pub fn update_finding_status(
    state: State<AppState>,
    id: u32,
    status: String,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;
    do_update_finding_status(conn, id, &status)
}

#[tauri::command]
pub fn duplicate_finding(state: State<AppState>, id: u32) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;
    do_duplicate_finding(conn, id)
}

#[tauri::command]
pub fn archive_finding(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;
    do_archive_finding(conn, id)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn list_findings(
    state: State<AppState>,
    engagement_id: Option<u32>,
    client_id: Option<u32>,
    search: Option<String>,
    severity: Option<String>,
    status: Option<String>,
    owasp_category: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<FindingPage, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;
    do_list_findings(
        conn,
        engagement_id,
        client_id,
        search.as_deref(),
        severity.as_deref(),
        status.as_deref(),
        owasp_category.as_deref(),
        offset.unwrap_or(0),
        limit.unwrap_or(50),
    )
}

#[tauri::command]
pub fn get_finding_counts(state: State<AppState>) -> Result<FindingCounts, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;
    do_get_finding_counts(conn)
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

    fn make_minimal_finding_input(engagement_id: u32, title: &str) -> FindingInput {
        FindingInput {
            engagement_id,
            title: title.to_string(),
            severity: "high".to_string(),
            overview: "Overview".to_string(),
            summary: "Summary".to_string(),
            affected_endpoints: vec![AffectedEndpoint {
                method: "GET".to_string(),
                path: "/api/test".to_string(),
                description: "Test endpoint".to_string(),
            }],
            evidence: vec![Evidence {
                title: "Evidence 1".to_string(),
                request: "GET /api/test".to_string(),
                response: "200 OK".to_string(),
            }],
            impact_items: vec![ImpactItem {
                title: "Impact".to_string(),
                explanation: "Explanation".to_string(),
            }],
            remediation_items: vec![RemediationItem {
                action: "Fix".to_string(),
                fix: "Fix it".to_string(),
                code_snippet: None,
            }],
            steps_to_reproduce: "Step 1".to_string(),
            cvss_score: None,
            owasp_category: None,
            cwe_id: None,
            references: None,
            tags: None,
            notes: None,
        }
    }

    #[test]
    fn test_create_and_get_finding() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_minimal_finding_input(eid, "SQL Injection");
        let id = do_create_finding(&conn, &input).unwrap();

        let f = do_get_finding(&conn, id).unwrap();
        assert_eq!(f.title, "SQL Injection");
        assert_eq!(f.severity, "high");
        assert_eq!(f.status, "draft");
        assert_eq!(f.affected_endpoints.len(), 1);
        assert_eq!(f.evidence.len(), 1);
    }

    #[test]
    fn test_reject_missing_engagement() {
        let conn = test_conn();
        let input = make_minimal_finding_input(9999, "XSS");
        let result = do_create_finding(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("archived"));
    }

    #[test]
    fn test_invalid_severity() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let mut input = make_minimal_finding_input(eid, "Bad");
        input.severity = "bad".to_string();
        let result = do_create_finding(&conn, &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid severity"));
    }

    #[test]
    fn test_update_finding_status() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_minimal_finding_input(eid, "XSS");
        let id = do_create_finding(&conn, &input).unwrap();

        do_update_finding_status(&conn, id, "confirmed").unwrap();
        let f = do_get_finding(&conn, id).unwrap();
        assert_eq!(f.status, "confirmed");
    }

    #[test]
    fn test_duplicate_finding() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_minimal_finding_input(eid, "CSRF");
        let id = do_create_finding(&conn, &input).unwrap();

        let dup_id = do_duplicate_finding(&conn, id).unwrap();
        let dup = do_get_finding(&conn, dup_id).unwrap();
        assert_eq!(dup.title, "[COPY] CSRF");
        assert_eq!(dup.status, "draft");
    }

    #[test]
    fn test_archive_hides_from_list() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_minimal_finding_input(eid, "LFI");
        let id = do_create_finding(&conn, &input).unwrap();

        let page = do_list_findings(&conn, None, None, None, None, None, None, 0, 50).unwrap();
        assert_eq!(page.total, 1);

        do_archive_finding(&conn, id).unwrap();
        let after = do_list_findings(&conn, None, None, None, None, None, None, 0, 50).unwrap();
        assert_eq!(after.total, 0);
    }

    #[test]
    fn test_search_and_filter() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let mut input1 = make_minimal_finding_input(eid, "SQL Injection");
        input1.severity = "critical".to_string();
        input1.owasp_category = Some("A03:2021".to_string());
        let mut input2 = make_minimal_finding_input(eid, "XSS");
        input2.severity = "high".to_string();
        input2.owasp_category = Some("A03:2021".to_string());
        let mut input3 = make_minimal_finding_input(eid, "Info Leak");
        input3.severity = "low".to_string();

        do_create_finding(&conn, &input1).unwrap();
        do_create_finding(&conn, &input2).unwrap();
        do_create_finding(&conn, &input3).unwrap();

        let all = do_list_findings(&conn, None, None, None, None, None, None, 0, 50).unwrap();
        assert_eq!(all.total, 3);

        let by_sev =
            do_list_findings(&conn, None, None, None, Some("critical"), None, None, 0, 50).unwrap();
        assert_eq!(by_sev.total, 1);

        let by_search =
            do_list_findings(&conn, None, None, Some("SQL"), None, None, None, 0, 50).unwrap();
        assert_eq!(by_search.total, 1);

        let by_cat =
            do_list_findings(&conn, None, None, None, None, None, Some("A03:2021"), 0, 50).unwrap();
        assert_eq!(by_cat.total, 2);
    }

    #[test]
    fn test_pagination() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        for i in 0..5 {
            do_create_finding(
                &conn,
                &make_minimal_finding_input(eid, &format!("Finding {}", i)),
            )
            .unwrap();
        }

        let page1 = do_list_findings(&conn, None, None, None, None, None, None, 0, 2).unwrap();
        assert_eq!(page1.total, 5);
        assert_eq!(page1.items.len(), 2);

        let page2 = do_list_findings(&conn, None, None, None, None, None, None, 2, 2).unwrap();
        assert_eq!(page2.items.len(), 2);

        let page3 = do_list_findings(&conn, None, None, None, None, None, None, 4, 2).unwrap();
        assert_eq!(page3.items.len(), 1);
    }

    #[test]
    fn test_audit_snapshots() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let input = make_minimal_finding_input(eid, "Audit Test");
        let id = do_create_finding(&conn, &input).unwrap();
        do_update_finding_status(&conn, id, "confirmed").unwrap();
        do_archive_finding(&conn, id).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT action, old_value, new_value
                 FROM audit_log
                 WHERE table_name = 'findings'
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
        assert_eq!(rows[1].0, "STATUS_CHANGE");
        assert_eq!(rows[2].0, "ARCHIVE");
    }

    #[test]
    fn test_get_finding_counts() {
        let conn = test_conn();
        let cid = do_create_client(&conn, "Acme", None, None, None, None).unwrap();
        let eid = do_create_engagement(&conn, &make_engagement_input(cid, "Q1")).unwrap();

        let mut input1 = make_minimal_finding_input(eid, "Critical XSS");
        input1.severity = "critical".to_string();
        let mut input2 = make_minimal_finding_input(eid, "High SQLi");
        input2.severity = "high".to_string();

        do_create_finding(&conn, &input1).unwrap();
        do_create_finding(&conn, &input2).unwrap();

        let counts = do_get_finding_counts(&conn).unwrap();
        assert_eq!(counts.total, 2);
        assert_eq!(counts.by_severity.get("critical"), Some(&1));
        assert_eq!(counts.by_severity.get("high"), Some(&1));
    }
}
