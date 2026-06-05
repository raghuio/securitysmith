use securitysmith_core::state::AppState;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct Checklist {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct ChecklistItem {
    pub id: u32,
    pub checklist_id: u32,
    pub category: String,
    pub test_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct EngagementChecklistItem {
    pub id: u32,
    pub engagement_id: u32,
    pub checklist_item_id: u32,
    pub status: String,
    pub linked_finding_id: Option<u32>,
    pub notes: Option<String>,
    pub updated_at: i64,
    pub checklist_item: ChecklistItem,
}

#[derive(Deserialize)]
pub struct ChecklistInput {
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize)]
pub struct ChecklistItemInput {
    pub checklist_id: u32,
    pub category: String,
    pub test_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct EngagementChecklistUpdate {
    pub engagement_id: u32,
    pub checklist_item_id: u32,
    pub status: String,
    pub linked_finding_id: Option<u32>,
    pub notes: Option<String>,
}

fn validate_status(s: &str) -> Result<(), String> {
    match s {
        "not_started" | "in_progress" | "tested" | "not_applicable" | "finding_created"
        | "deferred" => Ok(()),
        _ => Err(format!("Invalid status: {s}")),
    }
}

pub fn do_list_checklists(conn: &Connection) -> Result<Vec<Checklist>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, description, version, is_builtin, is_active, created_at FROM checklists WHERE is_active = 1 ORDER BY name")
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Checklist {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                version: row.get(3)?,
                is_builtin: row.get::<_, i32>(4)? != 0,
                is_active: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_checklist_items(
    conn: &Connection,
    checklist_id: u32,
) -> Result<Vec<ChecklistItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id, checklist_id, category, test_id, name, description, sort_order FROM checklist_items WHERE checklist_id = ? ORDER BY sort_order")
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(params![checklist_id], |row| {
            Ok(ChecklistItem {
                id: row.get(0)?,
                checklist_id: row.get(1)?,
                category: row.get(2)?,
                test_id: row.get(3)?,
                name: row.get(4)?,
                description: row.get(5)?,
                sort_order: row.get(6)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_create_checklist(conn: &Connection, input: &ChecklistInput) -> Result<u32, String> {
    conn.execute(
        "INSERT INTO checklists (name, description, version, is_active, created_at) VALUES (?1, ?2, ?3, 1, strftime('%s', 'now'))",
        params![input.name.trim(), input.description.as_deref(), input.version.as_deref()],
    )
    .map_err(|e| format!("Failed to create checklist: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    Ok(id)
}

pub fn do_create_checklist_item(
    conn: &Connection,
    input: &ChecklistItemInput,
) -> Result<u32, String> {
    conn.execute(
        "INSERT INTO checklist_items (checklist_id, category, test_id, name, description, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.checklist_id,
            input.category.trim(),
            input.test_id.as_deref(),
            input.name.trim(),
            input.description.as_deref(),
            0,
        ],
    )
    .map_err(|e| format!("Failed to create checklist item: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    Ok(id)
}

pub fn do_assign_checklist_to_engagement(
    conn: &Connection,
    engagement_id: u32,
    checklist_id: u32,
) -> Result<(), String> {
    let items = do_get_checklist_items(conn, checklist_id)?;
    for item in items {
        conn.execute(
            "INSERT OR IGNORE INTO engagement_checklist_items (engagement_id, checklist_item_id, status, updated_at) VALUES (?1, ?2, 'not_started', strftime('%s', 'now'))",
            params![engagement_id, item.id],
        )
        .map_err(|e| format!("Failed to assign checklist item: {e}"))?;
    }
    Ok(())
}

pub fn do_get_engagement_checklist(
    conn: &Connection,
    engagement_id: u32,
) -> Result<Vec<EngagementChecklistItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT eci.id, eci.engagement_id, eci.checklist_item_id, eci.status, eci.linked_finding_id, eci.notes, eci.updated_at,
             ci.id, ci.checklist_id, ci.category, ci.test_id, ci.name, ci.description, ci.sort_order
             FROM engagement_checklist_items eci
             JOIN checklist_items ci ON ci.id = eci.checklist_item_id
             WHERE eci.engagement_id = ? ORDER BY ci.sort_order",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(params![engagement_id], |row| {
            Ok(EngagementChecklistItem {
                id: row.get(0)?,
                engagement_id: row.get(1)?,
                checklist_item_id: row.get(2)?,
                status: row.get(3)?,
                linked_finding_id: row.get(4)?,
                notes: row.get(5)?,
                updated_at: row.get(6)?,
                checklist_item: ChecklistItem {
                    id: row.get(7)?,
                    checklist_id: row.get(8)?,
                    category: row.get(9)?,
                    test_id: row.get(10)?,
                    name: row.get(11)?,
                    description: row.get(12)?,
                    sort_order: row.get(13)?,
                },
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_update_engagement_checklist_item(
    conn: &Connection,
    id: u32,
    status: &str,
    linked_finding_id: Option<u32>,
    notes: Option<&str>,
) -> Result<(), String> {
    validate_status(status)?;
    conn.execute(
        "UPDATE engagement_checklist_items SET status = ?1, linked_finding_id = ?2, notes = ?3, updated_at = strftime('%s', 'now') WHERE id = ?4",
        params![status, linked_finding_id, notes, id],
    )
    .map_err(|e| format!("Failed to update: {e}"))?;
    Ok(())
}

pub fn do_get_checklist_coverage(
    conn: &Connection,
    engagement_id: u32,
) -> Result<(f64, u32, u32), String> {
    let total: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM engagement_checklist_items WHERE engagement_id = ?",
            params![engagement_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("DB: {e}"))?;
    if total == 0 {
        return Ok((0.0, 0, 0));
    }
    let completed: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM engagement_checklist_items WHERE engagement_id = ? AND status IN ('tested', 'finding_created', 'not_applicable')",
            params![engagement_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("DB: {e}"))?;
    let pct = (completed as f64 / total as f64) * 100.0;
    Ok((pct, completed, total))
}

// Tauri commands
#[tauri::command]
pub fn list_checklists(state: State<AppState>) -> Result<Vec<Checklist>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_checklists(conn)
}

#[tauri::command]
pub fn get_checklist_items(
    state: State<AppState>,
    checklist_id: u32,
) -> Result<Vec<ChecklistItem>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_checklist_items(conn, checklist_id)
}

#[tauri::command]
pub fn create_checklist(state: State<AppState>, input: ChecklistInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_checklist(conn, &input)
}

#[tauri::command]
pub fn create_checklist_item(
    state: State<AppState>,
    input: ChecklistItemInput,
) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_checklist_item(conn, &input)
}

#[tauri::command]
pub fn assign_checklist_to_engagement(
    state: State<AppState>,
    engagement_id: u32,
    checklist_id: u32,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_assign_checklist_to_engagement(conn, engagement_id, checklist_id)
}

#[tauri::command]
pub fn get_engagement_checklist(
    state: State<AppState>,
    engagement_id: u32,
) -> Result<Vec<EngagementChecklistItem>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_engagement_checklist(conn, engagement_id)
}

#[tauri::command]
pub fn update_engagement_checklist_item(
    state: State<AppState>,
    id: u32,
    status: String,
    linked_finding_id: Option<u32>,
    notes: Option<String>,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_update_engagement_checklist_item(conn, id, &status, linked_finding_id, notes.as_deref())
}

#[tauri::command]
pub fn get_checklist_coverage(
    state: State<AppState>,
    engagement_id: u32,
) -> Result<(f64, u32, u32), String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_checklist_coverage(conn, engagement_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use securitysmith_core::db;

    fn test_conn() -> Connection {
        let tmp = tempfile::tempdir().unwrap();
        let conn = db::open_vault(tmp.path(), &[0u8; 32]).unwrap();
        db::init_db(&conn).unwrap();
        conn
    }

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

    fn make_checklist(conn: &Connection, name: &str) -> u32 {
        let id = do_create_checklist(
            conn,
            &ChecklistInput {
                name: name.to_string(),
                description: Some("desc".to_string()),
                version: Some("1.0".to_string()),
            },
        )
        .unwrap();
        id
    }

    fn add_item(conn: &Connection, checklist_id: u32, name: &str) -> u32 {
        do_create_checklist_item(
            conn,
            &ChecklistItemInput {
                checklist_id,
                category: "Test".to_string(),
                test_id: Some(format!("TST-{name}")),
                name: name.to_string(),
                description: Some("d".to_string()),
            },
        )
        .unwrap()
    }

    #[test]
    fn test_create_and_list_checklist() {
        let conn = test_conn();
        let id = make_checklist(&conn, "My Custom Checklist");
        let list = do_list_checklists(&conn).unwrap();
        // Includes the seeded OWASP WSTG + our new one
        assert!(list.iter().any(|c| c.id == id));
    }

    #[test]
    fn test_add_items_and_get_them() {
        let conn = test_conn();
        let cid = make_checklist(&conn, "Items Test");
        let i1 = add_item(&conn, cid, "Item One");
        let i2 = add_item(&conn, cid, "Item Two");
        let items = do_get_checklist_items(&conn, cid).unwrap();
        assert_eq!(items.len(), 2);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"Item One"));
        assert!(names.contains(&"Item Two"));
        // ids should match
        assert!(items.iter().any(|i| i.id == i1));
        assert!(items.iter().any(|i| i.id == i2));
    }

    #[test]
    fn test_assign_checklist_to_engagement() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let cid = make_checklist(&conn, "Assign Test");
        add_item(&conn, cid, "A");
        add_item(&conn, cid, "B");

        do_assign_checklist_to_engagement(&conn, eid, cid).unwrap();
        let items = do_get_engagement_checklist(&conn, eid).unwrap();
        assert_eq!(items.len(), 2);
        // All items default to not_started
        assert!(items.iter().all(|i| i.status == "not_started"));
    }

    #[test]
    fn test_update_checklist_item_status() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let cid = make_checklist(&conn, "Status Test");
        let iid = add_item(&conn, cid, "X");
        do_assign_checklist_to_engagement(&conn, eid, cid).unwrap();

        let items = do_get_engagement_checklist(&conn, eid).unwrap();
        let target = items.iter().find(|i| i.checklist_item_id == iid).unwrap();

        do_update_engagement_checklist_item(&conn, target.id, "tested", None, Some("done"))
            .unwrap();
        let items = do_get_engagement_checklist(&conn, eid).unwrap();
        let updated = items.iter().find(|i| i.id == target.id).unwrap();
        assert_eq!(updated.status, "tested");
        assert_eq!(updated.notes.as_deref(), Some("done"));
    }

    #[test]
    fn test_invalid_status_rejected() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let cid = make_checklist(&conn, "Invalid Status");
        let _iid = add_item(&conn, cid, "Y");
        do_assign_checklist_to_engagement(&conn, eid, cid).unwrap();
        let items = do_get_engagement_checklist(&conn, eid).unwrap();

        let err = do_update_engagement_checklist_item(&conn, items[0].id, "bogus", None, None)
            .unwrap_err();
        assert!(err.to_lowercase().contains("invalid"));
    }

    #[test]
    fn test_checklist_coverage_calculation() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let cid = make_checklist(&conn, "Coverage Test");
        for n in &["A", "B", "C", "D"] {
            add_item(&conn, cid, n);
        }
        do_assign_checklist_to_engagement(&conn, eid, cid).unwrap();
        let items = do_get_engagement_checklist(&conn, eid).unwrap();

        // 0/4 completed at start
        let (_, completed, total) = do_get_checklist_coverage(&conn, eid).unwrap();
        assert_eq!(total, 4);
        assert_eq!(completed, 0);

        // Mark 2 as tested, 1 as not_applicable
        for (i, status) in items.iter().take(2).zip(["tested", "tested"].iter()) {
            do_update_engagement_checklist_item(&conn, i.id, status, None, None).unwrap();
        }
        do_update_engagement_checklist_item(&conn, items[2].id, "not_applicable", None, None)
            .unwrap();

        // 3/4 covered
        let (pct, completed, total) = do_get_checklist_coverage(&conn, eid).unwrap();
        assert_eq!(total, 4);
        assert_eq!(completed, 3);
        assert!((pct - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_owasp_wstg_seed_present() {
        let conn = test_conn();
        let list = do_list_checklists(&conn).unwrap();
        let wstg = list.iter().find(|c| c.name == "OWASP WSTG v4.2");
        assert!(wstg.is_some(), "OWASP WSTG v4.2 must be seeded");
        let items = do_get_checklist_items(&conn, wstg.unwrap().id).unwrap();
        assert!(
            items.len() >= 12,
            "Should have at least 12 categories of items"
        );
    }
}
