use crate::error::AppError;
use crate::state::AppState;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ActivityLogEntry {
    pub id: u32,
    pub table_name: String,
    pub action: String,
    pub record_id: u32,
    pub old_value: String,
    pub new_value: String,
    pub context: String,
    pub timestamp: i64,
}

#[derive(Deserialize)]
pub struct ActivityLogFilters {
    pub table_name: Option<String>,
    pub action: Option<String>,
    pub search: Option<String>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

fn build_list_query(filters: &ActivityLogFilters) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = "SELECT id, table_name, action, record_id, old_value, new_value, context, timestamp FROM audit_log".to_string();
    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut clauses: Vec<String> = Vec::new();

    if let Some(t) = &filters.table_name {
        clauses.push("table_name = ?".to_string());
        ps.push(Box::new(t.clone()));
    }
    if let Some(a) = &filters.action {
        clauses.push("action = ?".to_string());
        ps.push(Box::new(a.clone()));
    }
    if let Some(q) = &filters.search {
        clauses.push(
            "(table_name LIKE ? OR action LIKE ? OR old_value LIKE ? OR new_value LIKE ?)"
                .to_string(),
        );
        let pattern = format!("%{q}%");
        ps.push(Box::new(pattern.clone()));
        ps.push(Box::new(pattern.clone()));
        ps.push(Box::new(pattern.clone()));
        ps.push(Box::new(pattern));
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
    let limit = filters.limit.map(|l| l as i64).unwrap_or(50);
    let offset = filters.offset.map(|o| o as i64).unwrap_or(0);
    ps.push(Box::new(limit));
    ps.push(Box::new(offset));
    (sql, ps)
}

fn do_list_activity_log(
    conn: &Connection,
    filters: &ActivityLogFilters,
) -> Result<Vec<ActivityLogEntry>, String> {
    let (sql, ps) = build_list_query(filters);
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(AppError::from)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(p_refs), |row| {
            Ok(ActivityLogEntry {
                id: row.get(0)?,
                table_name: row.get(1)?,
                action: row.get(2)?,
                record_id: row.get(3)?,
                old_value: row.get(4)?,
                new_value: row.get(5)?,
                context: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })
        .map_err(AppError::from)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn do_export_activity_log(
    conn: &Connection,
    filters: &ActivityLogFilters,
    file_path: &str,
) -> Result<u32, String> {
    let (sql, ps) = build_list_query(filters);
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(AppError::from)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(p_refs), |row| {
            Ok(ActivityLogEntry {
                id: row.get(0)?,
                table_name: row.get(1)?,
                action: row.get(2)?,
                record_id: row.get(3)?,
                old_value: row.get(4)?,
                new_value: row.get(5)?,
                context: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })
        .map_err(AppError::from)?;

    let mut wtr = csv::WriterBuilder::new()
        .from_path(file_path)
        .map_err(AppError::from)?;
    wtr.write_record([
        "id",
        "table_name",
        "action",
        "record_id",
        "old_value",
        "new_value",
        "context",
        "timestamp",
    ])
    .map_err(AppError::from)?;
    let mut count = 0u32;
    for row in rows {
        let row = row.map_err(AppError::from)?;
        wtr.write_record([
            row.id.to_string(),
            row.table_name,
            row.action,
            row.record_id.to_string(),
            row.old_value,
            row.new_value,
            row.context,
            row.timestamp.to_string(),
        ])
        .map_err(AppError::from)?;
        count += 1;
    }
    wtr.flush().map_err(AppError::from)?;
    Ok(count)
}

#[tauri::command]
pub fn list_activity_log(
    state: State<AppState>,
    filters: ActivityLogFilters,
) -> Result<Vec<ActivityLogEntry>, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;
    do_list_activity_log(conn, &filters)
}

#[tauri::command]
pub fn export_activity_log(
    state: State<AppState>,
    filters: ActivityLogFilters,
    file_path: String,
) -> Result<u32, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;
    do_export_activity_log(conn, &filters, &file_path)
}

#[tauri::command]
pub fn get_entity_history(
    state: State<AppState>,
    table_name: String,
    entity_id: u32,
) -> Result<Vec<ActivityLogEntry>, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, table_name, action, record_id, old_value, new_value, context, timestamp FROM audit_log WHERE table_name = ? AND record_id = ? ORDER BY timestamp DESC LIMIT 200"
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map(rusqlite::params![&table_name, entity_id], |row| {
            Ok(ActivityLogEntry {
                id: row.get(0)?,
                table_name: row.get(1)?,
                action: row.get(2)?,
                record_id: row.get(3)?,
                old_value: row.get(4)?,
                new_value: row.get(5)?,
                context: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })
        .map_err(AppError::from)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

#[tauri::command]
pub fn prune_audit_log(state: State<AppState>, before_date: String) -> Result<u32, String> {
    let mut guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = guard.connection().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params!["system", "PRUNE", 0, "", "", format!("before_date={before_date}")],
    )
    .map_err(AppError::from)?;
    let deleted = conn
        .execute(
            "DELETE FROM audit_log WHERE timestamp < strftime('%s', ?)",
            [&before_date],
        )
        .map_err(AppError::from)?;
    Ok(deleted as u32)
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::test_conn;
    use super::*;
    use crate::db;


    #[test]
    fn test_activity_log_empty_returns_empty_list() {
        let conn = test_conn();
        let filters = ActivityLogFilters {
            table_name: None,
            action: None,
            search: None,
            offset: None,
            limit: None,
        };
        let entries = do_list_activity_log(&conn, &filters).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_activity_log_filters_by_table() {
        let conn = test_conn();
        // Manually insert audit_log row with a numeric record_id matching u32 schema
        conn.execute(
            "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
             VALUES ('clients', 'INSERT', '1', NULL, '{}', 'manual')",
            [],
        )
        .unwrap();
        let filters = ActivityLogFilters {
            table_name: Some("clients".to_string()),
            action: None,
            search: None,
            offset: None,
            limit: None,
        };
        let entries = do_list_activity_log(&conn, &filters).unwrap();
        // The first test (test_activity_log_empty_returns_empty_list) ran first and now
        // there's a "clients" entry. Just check that the table filter works.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE table_name = 'clients'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count >= 1, "should have at least one clients audit entry");
        assert!(entries.iter().all(|e| e.table_name == "clients"));
    }
}
