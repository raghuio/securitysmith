use rusqlite::OptionalExtension;
use rusqlite::{Connection, params};
use serde::Serialize;
use ss_core::state::AppState;
use tauri::State;

#[derive(Serialize)]
pub struct Document {
    pub id: u32,
    pub client_id: u32,
    pub client_name: String,
    pub engagement_id: Option<u32>,
    pub engagement_name: Option<String>,
    pub name: String,
    pub document_type: String,
    pub content: String,
    pub status: String,
    pub template_id: Option<u32>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_to_doc(row: &rusqlite::Row) -> Result<Document, rusqlite::Error> {
    Ok(Document {
        id: row.get(0)?,
        client_id: row.get(1)?,
        client_name: row.get(2)?,
        engagement_id: row.get(3)?,
        engagement_name: row.get(4)?,
        name: row.get(5)?,
        document_type: row.get(6)?,
        content: row.get(7)?,
        status: row.get(8)?,
        template_id: row.get(9)?,
        is_active: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn do_list_docs(
    conn: &Connection,
    client_id: Option<u32>,
    engagement_id: Option<u32>,
) -> Result<Vec<Document>, String> {
    let mut sql = "SELECT d.id, d.client_id, c.short_name, d.engagement_id, e.name, d.name, d.document_type, d.content, d.status, d.template_id, d.is_active, d.created_at, d.updated_at
     FROM documents d JOIN clients c ON d.client_id = c.id LEFT JOIN engagements e ON d.engagement_id = e.id
     WHERE d.is_active = 1".to_string();
    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(cid) = client_id {
        sql.push_str(" AND d.client_id = ?");
        ps.push(Box::new(cid));
    }
    if let Some(eid) = engagement_id {
        sql.push_str(" AND d.engagement_id = ?");
        ps.push(Box::new(eid));
    }
    sql.push_str(" ORDER BY d.updated_at DESC");
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(p_refs), row_to_doc)
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn do_get_doc(conn: &Connection, id: u32) -> Result<Document, String> {
    let mut stmt = conn
        .prepare(
            "SELECT d.id, d.client_id, c.short_name, d.engagement_id, e.name, d.name, d.document_type, d.content, d.status, d.template_id, d.is_active, d.created_at, d.updated_at
             FROM documents d JOIN clients c ON d.client_id = c.id LEFT JOIN engagements e ON d.engagement_id = e.id
             WHERE d.id = ? AND d.is_active = 1"
        )
        .map_err(|e| format!("Prepare failed: {e}"))?;
    let item: Option<Document> = stmt
        .query_map(params![id], row_to_doc)
        .map_err(|e| format!("Query failed: {e}"))?
        .next()
        .transpose()
        .map_err(|e| format!("Row failed: {e}"))?;
    item.ok_or_else(|| "Document not found.".to_string())
}

fn do_create_doc(
    conn: &Connection,
    client_id: u32,
    engagement_id: Option<u32>,
    name: &str,
    document_type: &str,
    content: &str,
    template_id: Option<u32>,
) -> Result<u32, String> {
    conn.execute(
        "INSERT INTO documents (client_id, engagement_id, name, document_type, content, status, template_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6, strftime('%s', 'now'), strftime('%s', 'now'))",
        params![client_id, engagement_id, name, document_type, content, template_id],
    )
    .map_err(|e| format!("Failed to create document: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    let new = do_get_doc(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["documents", "CREATE", id, "", new_json],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;
    Ok(id)
}

fn do_update_doc(
    conn: &Connection,
    id: u32,
    name: Option<&str>,
    content: Option<&str>,
    status: Option<&str>,
) -> Result<(), String> {
    let old = do_get_doc(conn, id)?;
    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {e}"))?;
    let mut updates: Vec<(&str, Box<dyn rusqlite::ToSql>)> = Vec::new();
    if let Some(n) = name {
        updates.push(("name = ?", Box::new(n.to_string())));
    }
    if let Some(c) = content {
        updates.push(("content = ?", Box::new(c.to_string())));
    }
    if let Some(s) = status {
        updates.push(("status = ?", Box::new(s.to_string())));
    }
    if updates.is_empty() {
        return Ok(());
    }
    let set_clause = updates
        .iter()
        .map(|(c, _)| *c)
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE documents SET {}, updated_at = strftime('%s', 'now') WHERE id = ?",
        set_clause
    );
    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = updates.into_iter().map(|(_, v)| v).collect();
    ps.push(Box::new(id));
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, rusqlite::params_from_iter(p_refs))
        .map_err(|e| format!("Update failed: {e}"))?;
    let new = do_get_doc(conn, id)?;
    let new_json = serde_json::to_string(&new).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["documents", "UPDATE", id, old_json, new_json],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;
    Ok(())
}

fn do_archive_doc(conn: &Connection, id: u32) -> Result<(), String> {
    let old = do_get_doc(conn, id)?;
    let old_json = serde_json::to_string(&old).map_err(|e| format!("Serialize failed: {e}"))?;
    conn.execute(
        "UPDATE documents SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Archive failed: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["documents", "ARCHIVE", id, old_json, ""],
    )
    .map_err(|e| format!("Audit failed: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn list_documents(
    state: State<AppState>,
    client_id: Option<u32>,
    engagement_id: Option<u32>,
) -> Result<Vec<Document>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_list_docs(conn, client_id, engagement_id)
}

#[tauri::command]
pub fn get_document(state: State<AppState>, id: u32) -> Result<Document, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_get_doc(conn, id)
}

#[tauri::command]
pub fn create_document(
    state: State<AppState>,
    client_id: u32,
    engagement_id: Option<u32>,
    name: String,
    document_type: String,
    content: String,
    template_id: Option<u32>,
) -> Result<u32, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_create_doc(
        conn,
        client_id,
        engagement_id,
        &name,
        &document_type,
        &content,
        template_id,
    )
}

#[tauri::command]
pub fn update_document(
    state: State<AppState>,
    id: u32,
    name: Option<String>,
    content: Option<String>,
    status: Option<String>,
) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_update_doc(
        conn,
        id,
        name.as_deref(),
        content.as_deref(),
        status.as_deref(),
    )
}

#[tauri::command]
pub fn archive_document(state: State<AppState>, id: u32) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_archive_doc(conn, id)
}

#[tauri::command]
pub fn render_document_placeholders(
    state: State<AppState>,
    content: String,
    client_id: u32,
    engagement_id: Option<u32>,
) -> Result<String, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;

    let client_name: String = conn
        .query_row(
            "SELECT name FROM clients WHERE id = ?",
            [client_id],
            |row| row.get(0),
        )
        .map_err(|_| "Client not found.".to_string())?;

    let engagement_name: Option<String> = if let Some(eid) = engagement_id {
        conn.query_row("SELECT name FROM engagements WHERE id = ?", [eid], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| format!("DB: {e}"))?
    } else {
        None
    };

    let company_name: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'profile.company_name'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {e}"))?
        .unwrap_or_default();

    let contact_email: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'profile.contact_email'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {e}"))?
        .unwrap_or_default();

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut result = content;
    result = result.replace("{{client_name}}", &client_name);
    result = result.replace("{{company_name}}", &company_name);
    result = result.replace("{{contact_email}}", &contact_email);
    result = result.replace("{{date}}", &today);
    if let Some(en) = &engagement_name {
        result = result.replace("{{engagement_name}}", en);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss_core::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn make_client(conn: &Connection) -> u32 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conn.execute(
            "INSERT INTO clients (short_name, registered_business_name, email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![format!("Client-{n}")],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    #[test]
    fn test_documents_crud() {
        let conn = test_conn();
        let cid = make_client(&conn);
        let docs = do_list_docs(&conn, None, None).unwrap();
        assert_eq!(docs.len(), 0);
        // Insert manually
        conn.execute(
            "INSERT INTO documents (client_id, name, document_type, content, status)
             VALUES (?1, 'SOW', 'sow', 'test content', 'draft')",
            params![cid],
        )
        .unwrap();
        let docs = do_list_docs(&conn, None, None).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].document_type, "sow");
    }
}
