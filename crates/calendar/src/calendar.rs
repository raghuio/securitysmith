use securitysmith_core::state::AppState;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct CalendarEvent {
    pub id: u32,
    pub client_id: u32,
    pub client_name: String,
    pub name: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: String,
}

#[derive(Serialize)]
pub struct Reminder {
    pub reminder_key: String,
    pub reminder_type: String,
    pub entity_id: u32,
    pub entity_name: String,
    pub client_name: String,
    pub due_date: String,
    pub days_until: i32,
    pub urgency: String,
}

fn do_list_calendar_events(conn: &Connection) -> Result<Vec<CalendarEvent>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.client_id, c.name, e.name, e.start_date, e.end_date, e.status
         FROM engagements e JOIN clients c ON e.client_id = c.id
         WHERE e.is_active = 1 AND e.status IN ('scheduled', 'active', 'paused')
         ORDER BY e.start_date",
        )
        .map_err(|e| format!("Prepare: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CalendarEvent {
                id: row.get(0)?,
                client_id: row.get(1)?,
                client_name: row.get(2)?,
                name: row.get(3)?,
                start_date: row.get(4)?,
                end_date: row.get(5)?,
                status: row.get(6)?,
            })
        })
        .map_err(|e| format!("Query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn do_get_active_reminders(conn: &Connection) -> Result<Vec<Reminder>, String> {
    let mut reminders: Vec<Reminder> = Vec::new();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Engagement start reminders (default 3 days before)
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, c.name, e.start_date
             FROM engagements e JOIN clients c ON e.client_id = c.id
             WHERE e.is_active = 1 AND e.status = 'scheduled' AND e.start_date IS NOT NULL
               AND e.start_date >= ? AND julianday(e.start_date) - julianday(?) <= 3",
        )
        .map_err(|e| format!("Prepare start: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![&today, &today], |row| {
            let start_date: String = row.get(3)?;
            let days = julianday_diff(&start_date, &today);
            Ok(Reminder {
                reminder_key: format!("engagement_start:{}", row.get::<_, u32>(0)?),
                reminder_type: "engagement_start".to_string(),
                entity_id: row.get(0)?,
                entity_name: row.get(1)?,
                client_name: row.get(2)?,
                due_date: start_date.clone(),
                days_until: days,
                urgency: if days <= 0 {
                    "today".to_string()
                } else {
                    "upcoming".to_string()
                },
            })
        })
        .map_err(|e| format!("Query start: {e}"))?;
    for r in rows.filter_map(|r| r.ok()) {
        if !is_dismissed(conn, &r.reminder_key)? {
            reminders.push(r);
        }
    }

    // Engagement end reminders (default 1 day before)
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, c.name, e.end_date
             FROM engagements e JOIN clients c ON e.client_id = c.id
             WHERE e.is_active = 1 AND e.status = 'active' AND e.end_date IS NOT NULL
               AND e.end_date >= ? AND julianday(e.end_date) - julianday(?) <= 1",
        )
        .map_err(|e| format!("Prepare end: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![&today, &today], |row| {
            let end_date: String = row.get(3)?;
            let days = julianday_diff(&end_date, &today);
            Ok(Reminder {
                reminder_key: format!("engagement_end:{}", row.get::<_, u32>(0)?),
                reminder_type: "engagement_end".to_string(),
                entity_id: row.get(0)?,
                entity_name: row.get(1)?,
                client_name: row.get(2)?,
                due_date: end_date.clone(),
                days_until: days,
                urgency: if days <= 0 {
                    "today".to_string()
                } else {
                    "upcoming".to_string()
                },
            })
        })
        .map_err(|e| format!("Query end: {e}"))?;
    for r in rows.filter_map(|r| r.ok()) {
        if !is_dismissed(conn, &r.reminder_key)? {
            reminders.push(r);
        }
    }

    // Overdue invoices (status-based; invoices schema has no due_date column)
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.invoice_number, c.name, i.updated_at
             FROM invoices i JOIN clients c ON i.client_id = c.id
             WHERE i.is_active = 1 AND i.status IN ('sent', 'overdue')",
        )
        .map_err(|e| format!("Prepare invoice: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(Reminder {
                reminder_key: format!("invoice_due:{}", row.get::<_, u32>(0)?),
                reminder_type: "invoice_due".to_string(),
                entity_id: row.get(0)?,
                entity_name: row.get(1)?,
                client_name: row.get(2)?,
                due_date: row.get::<_, String>(3)?,
                days_until: 0,
                urgency: "overdue".to_string(),
            })
        })
        .map_err(|e| format!("Query invoice: {e}"))?;
    for r in rows.filter_map(|r| r.ok()) {
        if !is_dismissed(conn, &r.reminder_key)? {
            reminders.push(r);
        }
    }

    // Prune old dismissals (> 1 year)
    let _ = conn.execute(
        "DELETE FROM dismissed_reminders WHERE dismissed_at < strftime('%s', 'now', '-1 year')",
        [],
    );

    // Sort: overdue first, then today, then upcoming
    reminders.sort_by(|a, b| {
        let order = |r: &Reminder| match r.urgency.as_str() {
            "overdue" => 0,
            "today" => 1,
            _ => 2,
        };
        order(a)
            .cmp(&order(b))
            .then(a.days_until.cmp(&b.days_until))
    });

    // Cap at 10
    reminders.truncate(10);
    Ok(reminders)
}

fn julianday_diff(date: &str, today: &str) -> i32 {
    // Simple day difference for ISO dates YYYY-MM-DD
    if let (Ok(d1), Ok(d2)) = (
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d"),
        chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d"),
    ) {
        (d1 - d2).num_days() as i32
    } else {
        0
    }
}

fn is_dismissed(conn: &Connection, key: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM dismissed_reminders WHERE reminder_key = ?",
            [key],
            |row| row.get(0),
        )
        .map_err(|e| format!("Dismiss check: {e}"))?;
    Ok(count > 0)
}

#[tauri::command]
pub fn list_calendar_events(state: State<AppState>) -> Result<Vec<CalendarEvent>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_list_calendar_events(conn)
}

#[tauri::command]
pub fn get_active_reminders(state: State<AppState>) -> Result<Vec<Reminder>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    do_get_active_reminders(conn)
}

#[tauri::command]
pub fn dismiss_reminder(state: State<AppState>, reminder_key: String) -> Result<(), String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.connection_ref().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR IGNORE INTO dismissed_reminders (reminder_key) VALUES (?)",
        [&reminder_key],
    )
    .map_err(|e| format!("Dismiss failed: {e}"))?;
    Ok(())
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

    fn make_engagement(
        conn: &Connection,
        status: &str,
        start: Option<&str>,
        end: Option<&str>,
    ) -> u32 {
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            rusqlite::params![format!("Client-{n}")],
        )
        .unwrap();
        let cid = conn.last_insert_rowid() as u32;
        conn.execute(
            "INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model,
                engagement_type, status, start_date, end_date, objectives, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, 'Eng', 'web', 'pentest', 'auth', 'pentest', ?2, ?3, ?4, '[]', NULL, '[]', 1,
                strftime('%s','now'), strftime('%s','now'))",
            rusqlite::params![cid, status, start, end],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    #[test]
    fn test_calendar_lists_active_and_scheduled_engagements() {
        let conn = test_conn();
        make_engagement(&conn, "active", Some("2026-01-01"), Some("2026-12-31"));
        make_engagement(&conn, "scheduled", Some("2026-06-01"), Some("2026-07-01"));
        make_engagement(&conn, "completed", Some("2025-01-01"), Some("2025-12-31")); // excluded

        let events = do_list_calendar_events(&conn).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_dismiss_reminder_dedup() {
        let conn = test_conn();
        let key = "test_key";
        conn.execute(
            "INSERT INTO dismissed_reminders (reminder_key) VALUES (?1)",
            [key],
        )
        .unwrap();
        // Re-dismiss should be idempotent
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dismissed_reminders WHERE reminder_key = ?1",
                [key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
