use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::State;

const VALID_CATEGORIES: [&str; 6] = [
    "finding",
    "requirements",
    "checklist",
    "email",
    "status_report",
    "engagement_status",
];

#[derive(Serialize)]
pub struct TemplateSummary {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub subcategory: String,
    pub tags: Vec<String>,
    pub is_builtin: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
pub struct Template {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub subcategory: String,
    pub content: String,
    pub tags: Vec<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
pub struct TemplateInput {
    pub name: String,
    pub category: String,
    pub subcategory: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

fn parse_tags(tag_str: &str) -> Vec<String> {
    serde_json::from_str(tag_str).unwrap_or_default()
}

fn row_to_summary(row: &rusqlite::Row) -> Result<TemplateSummary, rusqlite::Error> {
    let tags_str: String = row.get(5)?;
    Ok(TemplateSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        subcategory: row.get(3)?,
        tags: parse_tags(&tags_str),
        is_builtin: row.get(4)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_template(row: &rusqlite::Row) -> Result<Template, rusqlite::Error> {
    let tags_str: String = row.get(5)?;
    Ok(Template {
        id: row.get(0)?,
        name: row.get(1)?,
        category: row.get(2)?,
        subcategory: row.get(3)?,
        content: row.get(4)?,
        tags: parse_tags(&tags_str),
        is_builtin: row.get(6)?,
        is_active: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn validate_category(category: &str) -> Result<(), String> {
    if VALID_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(format!(
            "Invalid category: {}. Must be one of: {:?}",
            category, VALID_CATEGORIES
        ))
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Template name is required.".to_string());
    }
    if name.len() > 255 {
        return Err("Template name exceeds 255 characters.".to_string());
    }
    Ok(())
}

fn validate_content(content: &str) -> Result<(), String> {
    if content.len() > 100_000 {
        return Err("Template content exceeds maximum size of 100KB.".to_string());
    }
    Ok(())
}

fn build_list_query(
    category: Option<&str>,
    subcategory: Option<&str>,
    search: Option<&str>,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = String::from(
        "SELECT id, name, category, subcategory, is_builtin, tags, created_at, updated_at
         FROM templates WHERE is_active = 1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(c) = category {
        sql.push_str(" AND category = ?");
        params.push(Box::new(c.to_string()));
    }
    if let Some(s) = subcategory {
        sql.push_str(" AND subcategory = ?");
        params.push(Box::new(s.to_string()));
    }
    if let Some(q) = search {
        sql.push_str(" AND (name LIKE ? OR tags LIKE ?)");
        let pattern = format!("%{}%", q);
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern));
    }

    sql.push_str(" ORDER BY category, name");
    (sql, params)
}

// ─────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────

pub fn do_list_templates(
    conn: &Connection,
    category: Option<&str>,
    subcategory: Option<&str>,
    search: Option<&str>,
) -> Result<Vec<TemplateSummary>, String> {
    let (sql, ps) = build_list_query(category, subcategory, search);
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare list: {}", e))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(p_refs), row_to_summary)
        .map_err(|e| format!("Failed to list templates: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {}", e))?);
    }
    Ok(items)
}

pub fn do_get_template(conn: &Connection, id: u32) -> Result<Template, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, category, subcategory, content, tags, is_builtin, is_active, created_at, updated_at
             FROM templates WHERE id = ?",
        )
        .map_err(|e| format!("Failed to prepare get: {}", e))?;
    let item: Option<Template> = stmt
        .query_map(params![id], row_to_template)
        .map_err(|e| format!("Failed to get template: {}", e))?
        .next()
        .transpose()
        .map_err(|e| format!("Row parse failed: {}", e))?;

    item.ok_or_else(|| "Template not found.".to_string())
}

pub fn do_create_template(conn: &Connection, input: &TemplateInput) -> Result<u32, String> {
    validate_name(&input.name)?;
    validate_category(&input.category)?;
    validate_content(&input.content)?;
    let tags_json = serde_json::to_string(&input.tags.clone().unwrap_or_default())
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;

    conn.execute(
        "INSERT INTO templates (name, category, subcategory, content, tags, is_builtin, is_active, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, strftime('%s', 'now'))",
        params![input.name, input.category, input.subcategory, input.content, tags_json],
    )
    .map_err(|e| format!("Failed to create template: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    let new_tmpl = do_get_template(conn, id)?;
    let new_json = serde_json::to_string(&new_tmpl)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["templates", "CREATE", id, "", new_json, ""],
    )
    .map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(id)
}

pub fn do_update_template(
    conn: &Connection,
    id: u32,
    name: Option<&str>,
    content: Option<&str>,
    tags: Option<Vec<String>>,
) -> Result<(), String> {
    let existing = do_get_template(conn, id)?;
    if existing.is_builtin {
        return Err(
            "Built-in templates are read-only. Duplicate to create an editable copy.".to_string(),
        );
    }

    let mut updates: Vec<(&str, Box<dyn rusqlite::ToSql>)> = Vec::new();

    if let Some(n) = name {
        validate_name(n)?;
        updates.push(("name = ?", Box::new(n.to_string())));
    }
    if let Some(c) = content {
        validate_content(c)?;
        updates.push(("content = ?", Box::new(c.to_string())));
    }
    if let Some(t) = tags {
        let tags_json =
            serde_json::to_string(&t).map_err(|e| format!("Failed to serialize tags: {}", e))?;
        updates.push(("tags = ?", Box::new(tags_json)));
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
        "UPDATE templates SET {}, updated_at = strftime('%s', 'now') WHERE id = ?",
        set_clause
    );

    let old_json = serde_json::to_string(&existing)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = updates.into_iter().map(|(_, v)| v).collect();
    ps.push(Box::new(id));
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();

    conn.execute(&sql, rusqlite::params_from_iter(p_refs))
        .map_err(|e| format!("Failed to update template: {}", e))?;

    let new_tmpl = do_get_template(conn, id)?;
    let new_json = serde_json::to_string(&new_tmpl)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["templates", "UPDATE", id, old_json, new_json, ""],
    )
    .map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(())
}

pub fn do_duplicate_template(conn: &Connection, id: u32) -> Result<u32, String> {
    let existing = do_get_template(conn, id)?;
    let new_name = format!("[COPY] {}", existing.name);
    validate_name(&new_name)?;
    let tags_json = serde_json::to_string(&existing.tags)
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;

    conn.execute(
        "INSERT INTO templates (name, category, subcategory, content, tags, is_builtin, is_active, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, strftime('%s', 'now'))",
        params![new_name, existing.category, existing.subcategory, existing.content, tags_json],
    )
    .map_err(|e| format!("Failed to duplicate template: {}", e))?;

    let new_id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    let new_tmpl = do_get_template(conn, new_id)?;
    let new_json = serde_json::to_string(&new_tmpl)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "templates",
            "DUPLICATE",
            new_id,
            "",
            new_json,
            format!("source_id={}", id)
        ],
    )
    .map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(new_id)
}

pub fn do_delete_template(conn: &Connection, id: u32) -> Result<(), String> {
    let existing = do_get_template(conn, id)?;
    if existing.is_builtin {
        return Err("Built-in templates cannot be deleted.".to_string());
    }

    conn.execute("DELETE FROM templates WHERE id = ?", params![id])
        .map_err(|e| format!("Failed to delete template: {}", e))?;

    let old_json = serde_json::to_string(&existing)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["templates", "DELETE", id, old_json, "", ""],
    )
    .map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(())
}

pub fn do_save_finding_as_template(
    conn: &Connection,
    finding_id: u32,
    name: &str,
    tags: Option<Vec<String>>,
) -> Result<u32, String> {
    validate_name(name)?;

    #[derive(serde::Deserialize)]
    struct FindingData {
        title: String,
        severity: String,
        overview: String,
        summary: String,
        affected_endpoints: String,
        evidence: String,
        impact_items: String,
        remediation_items: String,
        steps_to_reproduce: String,
        references_json: Option<String>,
        cvss_score: Option<f64>,
        owasp_category: Option<String>,
        cwe_id: Option<String>,
    }

    let row: Option<FindingData> = conn
        .query_row(
            "SELECT title, severity, overview, summary, affected_endpoints, evidence, impact_items, remediation_items, steps_to_reproduce, references_json, cvss_score, owasp_category, cwe_id
             FROM findings WHERE id = ? AND is_active = 1",
            params![finding_id],
            |row| {
                Ok(FindingData {
                    title: row.get(0)?,
                    severity: row.get(1)?,
                    overview: row.get(2)?,
                    summary: row.get(3)?,
                    affected_endpoints: row.get(4)?,
                    evidence: row.get(5)?,
                    impact_items: row.get(6)?,
                    remediation_items: row.get(7)?,
                    steps_to_reproduce: row.get(8)?,
                    references_json: row.get(9)?,
                    cvss_score: row.get(10)?,
                    owasp_category: row.get(11)?,
                    cwe_id: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Failed to load finding: {}", e))?;

    let finding = row.ok_or_else(|| "Finding not found.".to_string())?;

    let content = serde_json::json!({
        "title": finding.title,
        "severity": finding.severity,
        "overview": finding.overview,
        "summary": finding.summary,
        "affected_endpoints": serde_json::from_str::<serde_json::Value>(&finding.affected_endpoints).unwrap_or(serde_json::Value::Null),
        "evidence": serde_json::from_str::<serde_json::Value>(&finding.evidence).unwrap_or(serde_json::Value::Null),
        "impact_items": serde_json::from_str::<serde_json::Value>(&finding.impact_items).unwrap_or(serde_json::Value::Null),
        "remediation_items": serde_json::from_str::<serde_json::Value>(&finding.remediation_items).unwrap_or(serde_json::Value::Null),
        "steps_to_reproduce": finding.steps_to_reproduce,
        "references": serde_json::from_str::<serde_json::Value>(finding.references_json.as_deref().unwrap_or("[]")).unwrap_or(serde_json::Value::Null),
        "cvss_score": finding.cvss_score,
        "owasp_category": finding.owasp_category,
        "cwe_id": finding.cwe_id,
    });

    let content_str = content.to_string();
    validate_content(&content_str)?;
    let tags_json = serde_json::to_string(&tags.unwrap_or_default())
        .map_err(|e| format!("Failed to serialize tags: {}", e))?;

    conn.execute(
        "INSERT INTO templates (name, category, subcategory, content, tags, is_builtin, is_active, updated_at)
         VALUES (?1, 'finding', ?2, ?3, ?4, 0, 1, strftime('%s', 'now'))",
        params![name, finding.owasp_category.as_deref().unwrap_or(""), content_str, tags_json],
    )
    .map_err(|e| format!("Failed to save finding as template: {}", e))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    let new_tmpl = do_get_template(conn, id)?;
    let new_json = serde_json::to_string(&new_tmpl)
        .map_err(|e| format!("Failed to serialize audit value: {}", e))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "templates",
            "CREATE",
            id,
            "",
            new_json,
            format!("source_finding_id={}", finding_id)
        ],
    )
    .map_err(|e| format!("Failed to write audit log: {}", e))?;

    Ok(id)
}

// ─────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_templates(
    state: State<AppState>,
    category: Option<String>,
    subcategory: Option<String>,
    search: Option<String>,
) -> Result<Vec<TemplateSummary>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_list_templates(
        conn,
        category.as_deref(),
        subcategory.as_deref(),
        search.as_deref(),
    )
}

#[tauri::command]
pub fn get_template(state: State<AppState>, id: u32) -> Result<Template, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_get_template(conn, id)
}

#[tauri::command]
pub fn create_template(state: State<AppState>, input: TemplateInput) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_create_template(conn, &input)
}

#[tauri::command]
pub fn update_template(
    state: State<AppState>,
    id: u32,
    name: Option<String>,
    content: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_update_template(conn, id, name.as_deref(), content.as_deref(), tags)
}

#[tauri::command]
pub fn duplicate_template(state: State<AppState>, id: u32) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_duplicate_template(conn, id)
}

#[tauri::command]
pub fn delete_template(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_delete_template(conn, id)
}

#[tauri::command]
pub fn save_finding_as_template(
    state: State<AppState>,
    finding_id: u32,
    name: String,
    tags: Option<Vec<String>>,
) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
    do_save_finding_as_template(conn, finding_id, &name, tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let key = [7u8; 32];
        let conn = db::open_vault(tmp.path(), &key).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_and_get() {
        let conn = test_conn();
        let input = TemplateInput {
            name: "Test Template".to_string(),
            category: "checklist".to_string(),
            subcategory: "pre".to_string(),
            content: "{\"title\":\"test\"}".to_string(),
            tags: Some(vec!["test".to_string()]),
        };
        let id = do_create_template(&conn, &input).unwrap();
        let tmpl = do_get_template(&conn, id).unwrap();
        assert_eq!(tmpl.name, "Test Template");
        assert_eq!(tmpl.category, "checklist");
        assert!(!tmpl.is_builtin);
    }

    #[test]
    fn test_builtin_template_seeded() {
        let conn = test_conn();
        let items = do_list_templates(&conn, Some("finding"), None, None).unwrap();
        assert!(
            items.len() >= 10,
            "Expected at least 10 finding templates, got {}",
            items.len()
        );
        assert!(items.iter().any(|t| t.is_builtin));
    }

    #[test]
    fn test_built_in_read_only() {
        let conn = test_conn();
        let items = do_list_templates(&conn, Some("finding"), None, None).unwrap();
        let builtin = items.iter().find(|t| t.is_builtin).unwrap();
        let result = do_update_template(&conn, builtin.id, Some("Changed"), None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only"));
    }

    #[test]
    fn test_duplicate_template() {
        let conn = test_conn();
        let items = do_list_templates(&conn, Some("finding"), None, None).unwrap();
        let builtin = items.iter().find(|t| t.is_builtin).unwrap();
        let new_id = do_duplicate_template(&conn, builtin.id).unwrap();
        let copy = do_get_template(&conn, new_id).unwrap();
        assert!(!copy.is_builtin);
        assert!(copy.name.starts_with("[COPY]"));
    }

    #[test]
    fn test_search() {
        let conn = test_conn();
        let items = do_list_templates(&conn, None, None, Some("Injection")).unwrap();
        assert!(items.iter().any(|t| t.name.contains("Injection")));
    }
}
