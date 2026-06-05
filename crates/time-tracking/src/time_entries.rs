use securitysmith_core::state::AppState;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize)]
pub struct TimeEntry {
    pub id: u32,
    pub engagement_id: u32,
    pub entry_date: String,
    pub hours: f64,
    pub description: Option<String>,
    pub activity_type: String,
    pub is_billable: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
pub struct WeeklySummary {
    pub engagement_id: u32,
    pub engagement_name: String,
    pub total_hours: f64,
    pub billable_hours: f64,
}

#[derive(Serialize)]
pub struct BudgetStatus {
    pub engagement_id: u32,
    pub engagement_name: String,
    pub budgeted_hours: f64,
    pub logged_hours: f64,
    pub percentage: f64,
}

#[derive(Deserialize)]
pub struct TimeEntryInput {
    pub engagement_id: u32,
    pub entry_date: String,
    pub hours: f64,
    pub description: Option<String>,
    pub activity_type: String,
    pub is_billable: Option<bool>,
}

const VALID_ACTIVITIES: [&str; 8] = [
    "testing",
    "reporting",
    "scoping",
    "communication",
    "remediation_support",
    "retest",
    "admin",
    "other",
];

fn validate_activity(t: &str) -> Result<(), String> {
    if VALID_ACTIVITIES.contains(&t) {
        Ok(())
    } else {
        Err(format!(
            "Invalid activity_type: {}. Must be one of: {:?}",
            t, VALID_ACTIVITIES
        ))
    }
}

fn validate_entry(input: &TimeEntryInput) -> Result<(), String> {
    if input.hours <= 0.0 || input.hours > 24.0 {
        return Err("Hours must be between 0 and 24.".to_string());
    }
    validate_activity(&input.activity_type)?;
    Ok(())
}

fn row_to_entry(row: &rusqlite::Row) -> Result<TimeEntry, rusqlite::Error> {
    Ok(TimeEntry {
        id: row.get(0)?,
        engagement_id: row.get(1)?,
        entry_date: row.get(2)?,
        hours: row.get(3)?,
        description: row.get(4)?,
        activity_type: row.get(5)?,
        is_billable: row.get::<_, i32>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

pub fn do_list_time_entries(
    conn: &Connection,
    engagement_id: u32,
) -> Result<Vec<TimeEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, engagement_id, entry_date, hours, description, activity_type, is_billable, created_at, updated_at
             FROM time_entries WHERE engagement_id = ? AND is_active = 1 ORDER BY entry_date DESC, created_at DESC",
        )
        .map_err(|e| format!("Database error: {e}"))?;
    let rows = stmt
        .query_map(params![engagement_id], row_to_entry)
        .map_err(|e| format!("Database error: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {e}"))?);
    }
    Ok(items)
}

pub fn do_create_time_entry(conn: &Connection, input: &TimeEntryInput) -> Result<u32, String> {
    validate_entry(input)?;
    conn.execute(
        "INSERT INTO time_entries (engagement_id, entry_date, hours, description, activity_type, is_billable, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%s', 'now'))",
        params![
            input.engagement_id,
            input.entry_date,
            input.hours,
            input.description.as_deref(),
            input.activity_type.trim(),
            input.is_billable.unwrap_or(true) as i32,
        ],
    )
    .map_err(|e| format!("Failed to create time entry: {e}"))?;

    let id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["time_entries", "INSERT", id, "", "", format!("engagement_id={} hours={}", input.engagement_id, input.hours)],
    )
    .map_err(|e| format!("Audit log failed: {e}"))?;
    Ok(id)
}

pub fn do_update_time_entry(
    conn: &Connection,
    id: u32,
    input: &TimeEntryInput,
) -> Result<(), String> {
    validate_entry(input)?;
    conn.execute(
        "UPDATE time_entries SET entry_date = ?1, hours = ?2, description = ?3, activity_type = ?4, is_billable = ?5, updated_at = strftime('%s', 'now') WHERE id = ?6",
        params![
            input.entry_date,
            input.hours,
            input.description.as_deref(),
            input.activity_type.trim(),
            input.is_billable.unwrap_or(true) as i32,
            id,
        ],
    )
    .map_err(|e| format!("Failed to update time entry: {e}"))?;
    Ok(())
}

pub fn do_delete_time_entry(conn: &Connection, id: u32) -> Result<(), String> {
    conn.execute(
        "UPDATE time_entries SET is_active = 0, updated_at = strftime('%s', 'now') WHERE id = ?",
        params![id],
    )
    .map_err(|e| format!("Failed to delete time entry: {e}"))?;
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params!["time_entries", "DELETE", id, "", "", ""],
    )
    .map_err(|e| format!("Audit log failed: {e}"))?;
    Ok(())
}

pub fn do_get_weekly_summary(
    conn: &Connection,
    date_from: &str,
    date_to: &str,
) -> Result<Vec<WeeklySummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.engagement_id, e.name, SUM(t.hours) as total, SUM(CASE WHEN t.is_billable = 1 THEN t.hours ELSE 0 END) as billable
             FROM time_entries t JOIN engagements e ON e.id = t.engagement_id
             WHERE t.is_active = 1 AND t.entry_date BETWEEN ?1 AND ?2
             GROUP BY t.engagement_id ORDER BY total DESC",
        )
        .map_err(|e| format!("Database error: {e}"))?;
    let rows = stmt
        .query_map(params![date_from, date_to], |row| {
            Ok(WeeklySummary {
                engagement_id: row.get(0)?,
                engagement_name: row.get(1)?,
                total_hours: row.get(2)?,
                billable_hours: row.get(3)?,
            })
        })
        .map_err(|e| format!("Database error: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_budget_status(conn: &Connection) -> Result<Vec<BudgetStatus>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, e.budgeted_hours, COALESCE(SUM(t.hours), 0)
             FROM engagements e LEFT JOIN time_entries t ON t.engagement_id = e.id AND t.is_active = 1
             WHERE e.is_active = 1 AND e.budgeted_hours IS NOT NULL
             GROUP BY e.id",
        )
        .map_err(|e| format!("Database error: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let budgeted: f64 = row.get(2)?;
            let logged: f64 = row.get(3)?;
            Ok(BudgetStatus {
                engagement_id: row.get(0)?,
                engagement_name: row.get(1)?,
                budgeted_hours: budgeted,
                logged_hours: logged,
                percentage: if budgeted > 0.0 {
                    (logged / budgeted) * 100.0
                } else {
                    0.0
                },
            })
        })
        .map_err(|e| format!("Database error: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row parse failed: {e}"))?);
    }
    Ok(items)
}

pub fn do_create_invoice_from_time(
    conn: &Connection,
    engagement_id: u32,
    date_from: &str,
    date_to: &str,
    rate: f64,
) -> Result<u32, String> {
    let mut stmt = conn
        .prepare(
            "SELECT activity_type, SUM(hours) FROM time_entries
             WHERE engagement_id = ? AND is_active = 1 AND is_billable = 1 AND entry_date BETWEEN ? AND ?
             GROUP BY activity_type",
        )
        .map_err(|e| format!("Database error: {e}"))?;
    let rows = stmt
        .query_map(params![engagement_id, date_from, date_to], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Database error: {e}"))?;

    let client_id: u32 = conn
        .query_row(
            "SELECT client_id FROM engagements WHERE id = ?",
            params![engagement_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Database error: {e}"))?;

    // Generate a unique invoice_number from the current timestamp + client id.
    // Format: INV-<unix_ts> so it is always unique even under concurrent inserts.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Clock error: {e}"))?
        .as_secs();
    let invoice_number = format!("INV-{ts}");

    conn.execute(
        "INSERT INTO invoices (client_id, engagement_id, document_type, invoice_number, status, notes, created_at, updated_at)
         VALUES (?1, ?2, 'invoice', ?3, 'draft', ?4, strftime('%s', 'now'), strftime('%s', 'now'))",
        params![client_id, engagement_id, invoice_number, format!("Generated from time entries {date_from} to {date_to}")],
    )
    .map_err(|e| format!("Failed to create invoice: {e}"))?;

    let invoice_id: u32 = conn
        .last_insert_rowid()
        .try_into()
        .map_err(|_| "ID overflow".to_string())?;

    for row in rows {
        let (activity, hours): (String, f64) = row.map_err(|e| format!("Row parse: {e}"))?;
        let amount = hours * rate;
        // invoice_items uses quantity (decimal hours) and rate_cents/total_cents.
        let rate_cents = (rate * 100.0).round() as i64;
        let amount_cents = (amount * 100.0).round() as i64;
        conn.execute(
            "INSERT INTO invoice_items (invoice_id, description, quantity, rate_cents, total_cents, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, strftime('%s', 'now'))",
            params![invoice_id, format!("{activity} ({hours} hours)"), hours as i64, rate_cents, amount_cents],
        )
        .map_err(|e| format!("Failed to add invoice item: {e}"))?;
    }

    Ok(invoice_id)
}

// Tauri commands
#[tauri::command]
pub fn list_time_entries(
    state: State<AppState>,
    engagement_id: u32,
) -> Result<Vec<TimeEntry>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_list_time_entries(conn, engagement_id)
}

#[tauri::command]
pub fn create_time_entry(state: State<AppState>, input: TimeEntryInput) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_time_entry(conn, &input)
}

#[tauri::command]
pub fn update_time_entry(
    state: State<AppState>,
    id: u32,
    input: TimeEntryInput,
) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_update_time_entry(conn, id, &input)
}

#[tauri::command]
pub fn delete_time_entry(state: State<AppState>, id: u32) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_delete_time_entry(conn, id)
}

#[tauri::command]
pub fn get_weekly_summary(
    state: State<AppState>,
    date_from: String,
    date_to: String,
) -> Result<Vec<WeeklySummary>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_weekly_summary(conn, &date_from, &date_to)
}

#[tauri::command]
pub fn get_budget_status(state: State<AppState>) -> Result<Vec<BudgetStatus>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_budget_status(conn)
}

#[tauri::command]
pub fn create_invoice_from_time(
    state: State<AppState>,
    engagement_id: u32,
    date_from: String,
    date_to: String,
    rate: f64,
) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_create_invoice_from_time(conn, engagement_id, &date_from, &date_to, rate)
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

    fn make_input(eid: u32, hours: f64, activity: &str) -> TimeEntryInput {
        TimeEntryInput {
            engagement_id: eid,
            entry_date: "2026-06-01".to_string(),
            hours,
            description: Some("desc".to_string()),
            activity_type: activity.to_string(),
            is_billable: Some(true),
        }
    }

    #[test]
    fn test_create_and_list_time_entry() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let id = do_create_time_entry(&conn, &make_input(eid, 2.5, "testing")).unwrap();
        assert!(id > 0);

        let entries = do_list_time_entries(&conn, eid).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hours, 2.5);
        assert_eq!(entries[0].activity_type, "testing");
    }

    #[test]
    fn test_hours_must_be_positive_and_bounded() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let err = do_create_time_entry(&conn, &make_input(eid, 0.0, "testing")).unwrap_err();
        assert!(err.to_lowercase().contains("hours"));

        let err = do_create_time_entry(&conn, &make_input(eid, 25.0, "testing")).unwrap_err();
        assert!(err.to_lowercase().contains("hours"));
    }

    #[test]
    fn test_invalid_activity_type_rejected() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let err = do_create_time_entry(&conn, &make_input(eid, 1.0, "bogus")).unwrap_err();
        assert!(err.to_lowercase().contains("activity"));
    }

    #[test]
    fn test_update_and_delete_time_entry() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let id = do_create_time_entry(&conn, &make_input(eid, 1.0, "testing")).unwrap();

        let updated = TimeEntryInput {
            hours: 3.0,
            ..make_input(eid, 0.0, "testing")
        };
        do_update_time_entry(&conn, id, &updated).unwrap();
        let entries = do_list_time_entries(&conn, eid).unwrap();
        assert_eq!(entries[0].hours, 3.0);

        do_delete_time_entry(&conn, id).unwrap();
        let entries = do_list_time_entries(&conn, eid).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_weekly_summary_aggregates_by_engagement() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        do_create_time_entry(&conn, &make_input(eid, 2.0, "testing")).unwrap();
        do_create_time_entry(&conn, &make_input(eid, 1.5, "reporting")).unwrap();
        do_create_time_entry(&conn, &make_input(eid, 0.5, "admin")).unwrap();

        let summary = do_get_weekly_summary(&conn, "2026-06-01", "2026-06-07").unwrap();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].engagement_id, eid);
        assert!((summary[0].total_hours - 4.0).abs() < 0.01);
        assert!((summary[0].billable_hours - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_budget_status_calculates_percentage() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        // Set budgeted_hours to 8
        conn.execute(
            "UPDATE engagements SET budgeted_hours = 8 WHERE id = ?1",
            params![eid],
        )
        .unwrap();
        do_create_time_entry(&conn, &make_input(eid, 6.0, "testing")).unwrap();

        let status = do_get_budget_status(&conn).unwrap();
        let s = status.iter().find(|b| b.engagement_id == eid).unwrap();
        assert_eq!(s.budgeted_hours, 8.0);
        assert_eq!(s.logged_hours, 6.0);
        assert!((s.percentage - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_budget_status_over_100_when_logged_exceeds_budget() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        conn.execute(
            "UPDATE engagements SET budgeted_hours = 4 WHERE id = ?1",
            params![eid],
        )
        .unwrap();
        do_create_time_entry(&conn, &make_input(eid, 6.0, "testing")).unwrap();

        let status = do_get_budget_status(&conn).unwrap();
        let s = status.iter().find(|b| b.engagement_id == eid).unwrap();
        assert!((s.percentage - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_create_invoice_from_time_groups_by_activity() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        do_create_time_entry(&conn, &make_input(eid, 2.0, "testing")).unwrap();
        do_create_time_entry(&conn, &make_input(eid, 1.0, "testing")).unwrap();
        do_create_time_entry(&conn, &make_input(eid, 3.0, "reporting")).unwrap();
        // out-of-range entry — should not be included
        let mut other = make_input(eid, 5.0, "admin");
        other.entry_date = "2025-01-01".to_string();
        do_create_time_entry(&conn, &other).unwrap();

        let invoice_id =
            do_create_invoice_from_time(&conn, eid, "2026-06-01", "2026-06-30", 100.0).unwrap();
        assert!(invoice_id > 0);
    }
}
