use securitysmith_core::state::AppState;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct ComplianceFramework {
    pub id: u32,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct ComplianceControl {
    pub id: u32,
    pub framework_id: u32,
    pub framework_name: String,
    pub control_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct FindingComplianceMapping {
    pub id: u32,
    pub finding_id: u32,
    pub control_id: u32,
    pub control: ComplianceControl,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct FrameworkInput {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct ControlInput {
    pub framework_id: u32,
    pub control_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Deserialize)]
pub struct MappingInput {
    pub finding_id: u32,
    pub control_id: u32,
    pub notes: Option<String>,
}

pub fn do_list_frameworks(conn: &Connection) -> Result<Vec<ComplianceFramework>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, version, description, is_builtin, is_active, created_at FROM compliance_frameworks WHERE is_active = 1 ORDER BY name")
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ComplianceFramework {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                description: row.get(3)?,
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

pub fn do_list_controls(
    conn: &Connection,
    framework_id: u32,
) -> Result<Vec<ComplianceControl>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT cc.id, cc.framework_id, cf.name, cc.control_id, cc.title, cc.description, cc.category, cc.sort_order
             FROM compliance_controls cc
             JOIN compliance_frameworks cf ON cf.id = cc.framework_id
             WHERE cc.framework_id = ? ORDER BY cc.sort_order",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(params![framework_id], |row| {
            Ok(ComplianceControl {
                id: row.get(0)?,
                framework_id: row.get(1)?,
                framework_name: row.get(2)?,
                control_id: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                category: row.get(6)?,
                sort_order: row.get(7)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_create_framework(conn: &Connection, input: &FrameworkInput) -> Result<u32, String> {
    conn.execute(
        "INSERT INTO compliance_frameworks (name, version, description, is_active, created_at) VALUES (?1, ?2, ?3, 1, strftime('%s', 'now'))",
        params![input.name.trim(), input.version.as_deref(), input.description.as_deref()],
    )
    .map_err(|e| format!("Failed to create framework: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    Ok(id)
}

pub fn do_create_control(conn: &Connection, input: &ControlInput) -> Result<u32, String> {
    conn.execute(
        "INSERT INTO compliance_controls (framework_id, control_id, title, description, category, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.framework_id,
            input.control_id.trim(),
            input.title.trim(),
            input.description.as_deref(),
            input.category.as_deref(),
            0,
        ],
    )
    .map_err(|e| format!("Failed to create control: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    Ok(id)
}

pub fn do_map_finding_to_control(conn: &Connection, input: &MappingInput) -> Result<u32, String> {
    conn.execute(
        "INSERT OR IGNORE INTO finding_compliance_mappings (finding_id, control_id, notes) VALUES (?1, ?2, ?3)",
        params![input.finding_id, input.control_id, input.notes.as_deref()],
    )
    .map_err(|e| format!("Failed to map: {e}"))?;
    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    Ok(id)
}

pub fn do_get_finding_mappings(
    conn: &Connection,
    finding_id: u32,
) -> Result<Vec<FindingComplianceMapping>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT fcm.id, fcm.finding_id, fcm.control_id, cc.id, cc.framework_id, cf.name, cc.control_id, cc.title, cc.description, cc.category, cc.sort_order, fcm.notes
             FROM finding_compliance_mappings fcm
             JOIN compliance_controls cc ON cc.id = fcm.control_id
             JOIN compliance_frameworks cf ON cf.id = cc.framework_id
             WHERE fcm.finding_id = ? ORDER BY cf.name, cc.sort_order",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(params![finding_id], |row| {
            Ok(FindingComplianceMapping {
                id: row.get(0)?,
                finding_id: row.get(1)?,
                control_id: row.get(2)?,
                control: ComplianceControl {
                    id: row.get(3)?,
                    framework_id: row.get(4)?,
                    framework_name: row.get(5)?,
                    control_id: row.get(6)?,
                    title: row.get(7)?,
                    description: row.get(8)?,
                    category: row.get(9)?,
                    sort_order: row.get(10)?,
                },
                notes: row.get(11)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_engagement_compliance_coverage(
    conn: &Connection,
    engagement_id: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT cf.name as framework, cc.control_id, cc.title, COUNT(fcm.id) as finding_count
             FROM compliance_frameworks cf
             JOIN compliance_controls cc ON cc.framework_id = cf.id
             LEFT JOIN finding_compliance_mappings fcm ON fcm.control_id = cc.id
             LEFT JOIN findings f ON f.id = fcm.finding_id AND f.engagement_id = ? AND f.is_active = 1
             GROUP BY cc.id ORDER BY cf.name, cc.sort_order",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(params![engagement_id], |row| {
            Ok(serde_json::json!({
                "framework": row.get::<_, String>(0)?,
                "control_id": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?,
                "finding_count": row.get::<_, i64>(3)?,
                "tested": row.get::<_, i64>(3)? > 0,
            }))
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_remove_mapping(conn: &Connection, mapping_id: u32) -> Result<(), String> {
    conn.execute(
        "DELETE FROM finding_compliance_mappings WHERE id = ?",
        params![mapping_id],
    )
    .map_err(|e| format!("Failed to remove mapping: {e}"))?;
    Ok(())
}

// Tauri commands
#[tauri::command]
pub fn list_frameworks(state: State<AppState>) -> Result<Vec<ComplianceFramework>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_frameworks(conn)
}

#[tauri::command]
pub fn list_controls(
    state: State<AppState>,
    framework_id: u32,
) -> Result<Vec<ComplianceControl>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_controls(conn, framework_id)
}

#[tauri::command]
pub fn create_framework(state: State<AppState>, input: FrameworkInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_framework(conn, &input)
}

#[tauri::command]
pub fn create_control(state: State<AppState>, input: ControlInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_control(conn, &input)
}

#[tauri::command]
pub fn map_finding_to_control(state: State<AppState>, input: MappingInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_map_finding_to_control(conn, &input)
}

#[tauri::command]
pub fn get_finding_mappings(
    state: State<AppState>,
    finding_id: u32,
) -> Result<Vec<FindingComplianceMapping>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_finding_mappings(conn, finding_id)
}

#[tauri::command]
pub fn get_engagement_compliance_coverage(
    state: State<AppState>,
    engagement_id: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_engagement_compliance_coverage(conn, engagement_id)
}

#[tauri::command]
pub fn remove_compliance_mapping(state: State<AppState>, mapping_id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_remove_mapping(conn, mapping_id)
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

    #[test]
    fn test_compliance_frameworks_list_seeded() {
        let conn = test_conn();
        let fws = do_list_frameworks(&conn).unwrap();
        // Seeded with PCI-DSS, OWASP Top 10, NIST CSF, ISO 27001
        assert!(fws.len() >= 1, "should have at least one seeded framework");
    }
}
