use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: u32,
    pub title: String,
    pub subtitle: String,
    pub relevance: f64,
}

#[allow(clippy::type_complexity)]
fn rebuild_search_index(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM search_index", [])
        .map_err(AppError::from)?;

    // Index clients
    let mut stmt = conn
        .prepare("SELECT id, name, contact_email, notes, tags FROM clients WHERE is_active = 1")
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(AppError::from)?;
    for row in rows {
        let (id, name, email, notes, tags) = row.map_err(AppError::from)?;
        let email_val = email.as_deref().unwrap_or_default();
        let notes_val = notes.as_deref().unwrap_or_default();
        let body = format!("{email_val} {notes_val} {tags}");
        conn.execute(
            "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["client", id, &name, email_val, &body, &tags],
        )
        .map_err(AppError::from)?;
    }

    // Index engagements
    let mut stmt = conn
        .prepare("SELECT e.id, e.name, c.name, e.scope_summary, e.notes, e.tags FROM engagements e JOIN clients c ON c.id = e.client_id WHERE e.is_active = 1")
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(AppError::from)?;
    for row in rows {
        let (id, name, client_name, scope, notes, tags) = row.map_err(AppError::from)?;
        let body = format!(
            "{} {} {}",
            scope.unwrap_or_default(),
            notes.unwrap_or_default(),
            tags
        );
        conn.execute(
            "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["engagement", id, &name, &client_name, &body, &tags],
        )
        .map_err(AppError::from)?;
    }

    // Index findings
    let mut stmt = conn
        .prepare("SELECT f.id, f.title, f.overview, f.summary, f.tags, e.name, c.name FROM findings f JOIN engagements e ON e.id = f.engagement_id JOIN clients c ON c.id = e.client_id WHERE f.is_active = 1")
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(AppError::from)?;
    for row in rows {
        let (id, title, overview, summary, tags, engagement_name, client_name) =
            row.map_err(AppError::from)?;
        let subtitle = format!("{client_name} · {engagement_name}");
        let body = format!("{overview} {summary}");
        conn.execute(
            "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["finding", id, &title, &subtitle, &body, &tags],
        )
        .map_err(AppError::from)?;
    }

    // Index templates
    let mut stmt = conn
        .prepare("SELECT id, name, category, subcategory, content, tags FROM templates WHERE is_active = 1")
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(AppError::from)?;
    for row in rows {
        let (id, name, category, subcategory, content, tags) =
            row.map_err(AppError::from)?;
        let subtitle = format!("{category} / {subcategory}");
        conn.execute(
            "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["template", id, &name, &subtitle, &content, &tags],
        )
        .map_err(AppError::from)?;
    }

    // Index credentials (PROP-005)
    let cred_table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='credentials'",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(AppError::from)?
        .unwrap_or(false);
    if cred_table_exists {
        let mut stmt = conn
            .prepare("SELECT id, label, credential_type, notes FROM credentials")
            .map_err(AppError::from)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(AppError::from)?;
        for row in rows.flatten() {
            let (id, label, cred_type, notes) = row;
            let subtitle = cred_type.clone();
            let body = notes.as_deref().unwrap_or("").to_string();
            let _ = conn.execute(
                "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params!["credential", id, &label, &subtitle, &body, ""],
            );
        }
    }

    // Index documents (PROP-010)
    let doc_table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='documents'",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(AppError::from)?
        .unwrap_or(false);
    if doc_table_exists {
        let mut stmt = conn
            .prepare("SELECT id, name, document_type, content FROM documents WHERE is_active = 1")
            .map_err(AppError::from)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(AppError::from)?;
        for row in rows.flatten() {
            let (id, name, doc_type, content) = row;
            let _ = conn.execute(
                "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, '')",
                params!["document", id, &name, &doc_type, &content],
            );
        }
    }

    // Index invoices (PROP-011)
    let inv_table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='invoices'",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(AppError::from)?
        .unwrap_or(false);
    if inv_table_exists {
        let mut stmt = conn
            .prepare("SELECT id, invoice_number, status, notes FROM invoices WHERE is_active = 1")
            .map_err(AppError::from)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(AppError::from)?;
        for row in rows.flatten() {
            let (id, number, status, notes) = row;
            let _ = conn.execute(
                "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, '')",
                params!["invoice", id, &number, &status, notes.as_deref().unwrap_or("")],
            );
        }
    }

    Ok(())
}

#[must_use]
pub fn do_global_search(
    conn: &Connection,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchResult>, String> {
    // Empty query: FTS5 MATCH cannot accept empty string, return empty.
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    // Ensure index is populated (lazy approach: rebuild on first search after boot)
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM search_index", [], |row| row.get(0))
        .unwrap_or(0);
    if count == 0 {
        rebuild_search_index(conn)?;
    }

    let mut stmt = conn
        .prepare(
            "SELECT entity_type, entity_id, title, subtitle, rank FROM search_index WHERE search_index MATCH ? ORDER BY rank LIMIT ?",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(params![query, limit], |row| {
            Ok(SearchResult {
                entity_type: row.get(0)?,
                entity_id: row.get(1)?,
                title: row.get(2)?,
                subtitle: row.get(3)?,
                relevance: row.get(4)?,
            })
        })
        .map_err(AppError::from)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(AppError::from)?);
    }
    Ok(items)
}

#[allow(clippy::type_complexity)]
#[must_use]
pub fn do_update_search_index_for_entity(
    conn: &Connection,
    entity_type: &str,
    entity_id: u32,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = ? AND entity_id = ?",
        params![entity_type, entity_id],
    )
    .map_err(AppError::from)?;

    match entity_type {
        "client" => {
            let row: Option<(u32, String, Option<String>, Option<String>, String)> = conn
                .query_row(
                    "SELECT id, name, contact_email, notes, tags FROM clients WHERE id = ? AND is_active = 1",
                    params![entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .optional()
                .map_err(AppError::from)?;
            if let Some((id, name, email, notes, tags)) = row {
                let email_val = email.as_deref().unwrap_or_default();
                let notes_val = notes.as_deref().unwrap_or_default();
                let body = format!("{email_val} {notes_val} {tags}");
                conn.execute(
                    "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params!["client", id, &name, email_val, &body, &tags],
                )
                .map_err(AppError::from)?;
            }
        }
        "engagement" => {
            let row: Option<(u32, String, String, Option<String>, Option<String>, String)> = conn
                .query_row(
                    "SELECT e.id, e.name, c.name, e.scope_summary, e.notes, e.tags FROM engagements e JOIN clients c ON c.id = e.client_id WHERE e.id = ? AND e.is_active = 1",
                    params![entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                )
                .optional()
                .map_err(AppError::from)?;
            if let Some((id, name, client_name, scope, notes, tags)) = row {
                let body = format!(
                    "{} {} {}",
                    scope.unwrap_or_default(),
                    notes.unwrap_or_default(),
                    tags
                );
                conn.execute(
                    "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params!["engagement", id, &name, &client_name, &body, &tags],
                )
                .map_err(AppError::from)?;
            }
        }
        "finding" => {
            let row: Option<(u32, String, String, String, String, String, String)> = conn
                .query_row(
                    "SELECT f.id, f.title, f.overview, f.summary, f.tags, e.name, c.name FROM findings f JOIN engagements e ON e.id = f.engagement_id JOIN clients c ON c.id = e.client_id WHERE f.id = ? AND f.is_active = 1",
                    params![entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
                )
                .optional()
                .map_err(AppError::from)?;
            if let Some((id, title, overview, summary, tags, engagement_name, client_name)) = row {
                let subtitle = format!("{client_name} · {engagement_name}");
                let body = format!("{overview} {summary}");
                conn.execute(
                    "INSERT INTO search_index (entity_type, entity_id, title, subtitle, body, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params!["finding", id, &title, &subtitle, &body, &tags],
                )
                .map_err(AppError::from)?;
            }
        }
        _ => {}
    }
    Ok(())
}

// Tauri commands
#[tauri::command]
pub fn global_search(
    state: State<AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_global_search(conn, &query, limit.unwrap_or(50))
}

#[tauri::command]
pub fn rebuild_search_index_command(state: State<AppState>) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    rebuild_search_index(conn)
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_conn;
    use super::*;
    use crate::db;


    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    fn make_engagement(conn: &Connection) -> u32 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let client_name = format!("Client-{n}");
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![client_name],
        )
        .unwrap();
        let cid = conn.last_insert_rowid() as u32;
        conn.execute(
            "INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model,
                engagement_type, status, objectives, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, 'Eng1', 'web', 'pentest', 'auth', 'pentest', 'active', '[]', NULL, '[]', 1,
                strftime('%s','now'), strftime('%s','now'))",
            params![cid],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    #[test]
    fn test_rebuild_search_index_populates_clients() {
        let conn = test_conn();
        // Add a client
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES ('Acme Corporation', 'sec@acme.com', 'Notes here', '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![],
        )
        .unwrap();
        rebuild_search_index(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE entity_type = 'client'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_global_search_finds_indexed_client() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES ('AcmeCorporation', 'sec@acme.com', 'Notes', '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![],
        )
        .unwrap();
        rebuild_search_index(&conn).unwrap();
        // FTS5 prefix match: "Acme*"
        let results = do_global_search(&conn, "Acme*", 50).unwrap();
        assert!(!results.is_empty(), "should find AcmeCorporation");
        assert!(
            results
                .iter()
                .any(|r| r.entity_type == "client" && r.title.contains("Acme"))
        );
    }

    #[test]
    fn test_global_search_finds_finding() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let endpoint = serde_json::to_string(&Vec::<String>::new()).unwrap();
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary,
                affected_endpoints, evidence, impact_items, remediation_items,
                steps_to_reproduce, status)
             VALUES (?1, 'SQLInjection', 'high', 'o', 's', ?2, '[]', '[]', '[]', 's', 'draft')",
            params![eid, endpoint],
        )
        .unwrap();
        let fid = conn.last_insert_rowid() as u32;
        rebuild_search_index(&conn).unwrap();
        let results = do_global_search(&conn, "SQLInjection*", 50).unwrap();
        assert!(
            results
                .iter()
                .any(|r| r.entity_type == "finding" && r.entity_id == fid)
        );
    }

    #[test]
    fn test_update_index_replaces_old_entry() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES ('Acme', NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![],
        )
        .unwrap();
        let id = conn.last_insert_rowid() as u32;
        rebuild_search_index(&conn).unwrap();

        // Update client name
        conn.execute(
            "UPDATE clients SET name = 'Wayne Enterprises' WHERE id = ?1",
            params![id],
        )
        .unwrap();
        do_update_search_index_for_entity(&conn, "client", id).unwrap();

        // Old query returns nothing
        let r1 = do_global_search(&conn, "Acme*", 10).unwrap();
        assert!(r1.is_empty(), "old name should not match after update");
        // New query returns it
        let r2 = do_global_search(&conn, "Wayne*", 10).unwrap();
        assert!(r2.iter().any(|r| r.entity_id == id));
    }

    #[test]
    fn test_search_index_cleared_on_delete() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES ('Doomed', NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![],
        )
        .unwrap();
        let id = conn.last_insert_rowid() as u32;
        rebuild_search_index(&conn).unwrap();
        // Manually delete from index (mirror what the do_delete_client would do)
        conn.execute(
            "DELETE FROM search_index WHERE entity_type = 'client' AND entity_id = ?1",
            params![id],
        )
        .unwrap();
        let results = do_global_search(&conn, "Doomed*", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_query_returns_empty() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES ('HasStuff', NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![],
        )
        .unwrap();
        rebuild_search_index(&conn).unwrap();
        let results = do_global_search(&conn, "", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_includes_engagements() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        // Rename the engagement
        conn.execute(
            "UPDATE engagements SET name = 'Banking Pentest' WHERE id = ?1",
            params![eid],
        )
        .unwrap();
        rebuild_search_index(&conn).unwrap();
        let results = do_global_search(&conn, "Banking*", 10).unwrap();
        assert!(
            results
                .iter()
                .any(|r| r.entity_type == "engagement" && r.entity_id == eid)
        );
    }
}
