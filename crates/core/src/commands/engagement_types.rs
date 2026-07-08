use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct EngagementTypeLabel {
    pub id: u32,
    pub label: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
}

// ─────────────────────────────────────────────────────────────
// Core logic
// ─────────────────────────────────────────────────────────────

pub fn do_create_engagement_type(
    conn: &Connection,
    label: &str,
    description: Option<&str>,
) -> crate::error::Result<u32> {
    conn.execute(
        "INSERT INTO engagement_type_labels (label, description, created_at)
         VALUES (?1, ?2, strftime('%s', 'now'))",
        params![label, description],
    )
    .map_err(AppError::from)?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| AppError::Generic("ID overflow".to_string()))?;
    Ok(id)
}

pub fn do_list_engagement_types(
    conn: &Connection,
    include_inactive: bool,
) -> crate::error::Result<Vec<EngagementTypeLabel>> {
    let sql = if include_inactive {
        "SELECT id, label, description, is_active, created_at FROM engagement_type_labels ORDER BY label"
    } else {
        "SELECT id, label, description, is_active, created_at FROM engagement_type_labels WHERE is_active = 1 ORDER BY label"
    };

    let mut stmt = conn.prepare(sql).map_err(AppError::from)?;
    let results = stmt
        .query_map([], |row| {
            Ok(EngagementTypeLabel {
                id: row.get(0)?,
                label: row.get(1)?,
                description: row.get(2)?,
                is_active: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    Ok(results)
}

pub fn do_update_engagement_type(
    conn: &Connection,
    id: u32,
    label: Option<&str>,
    description: Option<&str>,
) -> crate::error::Result<()> {
    conn.execute(
        "UPDATE engagement_type_labels SET
            label = COALESCE(?1, label),
            description = COALESCE(?2, description)
         WHERE id = ?3",
        params![label, description, id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

pub fn do_delete_engagement_type(conn: &Connection, id: u32) -> crate::error::Result<()> {
    conn.execute(
        "UPDATE engagement_type_labels SET is_active = 0 WHERE id = ?1",
        params![id],
    )
    .map_err(AppError::from)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────

fn validate_label(label: &str) -> Result<(), AppError> {
    if label.trim().is_empty() {
        return Err(AppError::Validation(
            "Engagement type label is required.".to_string(),
        ));
    }
    if label.len() > 100 {
        return Err(AppError::Validation(
            "Label must be 100 characters or fewer.".to_string(),
        ));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Tauri command wrappers
// ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_engagement_type(
    state: State<AppState>,
    label: String,
    description: Option<String>,
) -> Result<u32, String> {
    validate_label(&label)?;
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_create_engagement_type(conn, label.trim(), description.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_engagement_types(
    state: State<AppState>,
    include_inactive: bool,
) -> Result<Vec<EngagementTypeLabel>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    do_list_engagement_types(conn, include_inactive).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_engagement_type(
    state: State<AppState>,
    id: u32,
    label: Option<String>,
    description: Option<String>,
) -> Result<(), String> {
    if let Some(ref l) = label {
        validate_label(l)?;
    }
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_update_engagement_type(conn, id, label.as_deref(), description.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_engagement_type(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;

    do_delete_engagement_type(conn, id).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_conn;

    #[test]
    fn test_create_and_list_types() {
        let conn = test_conn();
        let id1 =
            do_create_engagement_type(&conn, "Pentest", Some("Standard penetration test")).unwrap();
        let id2 = do_create_engagement_type(&conn, "VAPT", Some("Vulnerability assessment + PT"))
            .unwrap();

        let list = do_list_engagement_types(&conn, false).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|t| t.label == "Pentest"));
        assert!(list.iter().any(|t| t.id == id2));
    }

    #[test]
    fn test_delete_type_is_soft() {
        let conn = test_conn();
        let id = do_create_engagement_type(&conn, "Retest", None).unwrap();

        do_delete_engagement_type(&conn, id).unwrap();
        let list_active = do_list_engagement_types(&conn, false).unwrap();
        assert_eq!(list_active.len(), 0);

        let list_all = do_list_engagement_types(&conn, true).unwrap();
        assert_eq!(list_all.len(), 1);
    }

    #[test]
    fn test_duplicate_label_rejected() {
        let conn = test_conn();
        do_create_engagement_type(&conn, "Pentest", None).unwrap();
        let result = do_create_engagement_type(&conn, "Pentest", None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("UNIQUE constraint")
        );
    }
}
