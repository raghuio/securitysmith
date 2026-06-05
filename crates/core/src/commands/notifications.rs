use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct Notification {
    pub id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub entity_type: String,
    pub entity_id: u32,
    pub timestamp: i64,
    pub is_dismissed: bool,
}

fn is_dismissed(conn: &Connection, key: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM dismissed_notifications WHERE notification_key = ?",
            params![key],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;
    Ok(count > 0)
}

#[must_use]
pub fn do_get_notifications(conn: &Connection) -> crate::error::Result<Vec<Notification>> {
    let mut notifications = Vec::new();

    // Overdue invoices — treat sent/overdue/cancelled as "needs attention".
    // (No due_date column on invoices schema; surfaced by status alone.)
    let mut stmt = conn
        .prepare(
            "SELECT i.id, c.name, i.status
             FROM invoices i JOIN clients c ON c.id = i.client_id
             WHERE i.is_active = 1 AND i.status IN ('sent', 'overdue')",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(AppError::from)?;
    for row in rows {
        let (id, client_name, status) = row.map_err(AppError::from)?;
        let key = format!("overdue_invoice:{id}");
        if !is_dismissed(conn, &key)? {
            notifications.push(Notification {
                id: key,
                category: "overdue_invoice".to_string(),
                title: format!("Invoice {} for {}", status.to_uppercase(), client_name),
                description: format!("Status: {status}"),
                entity_type: "invoice".to_string(),
                entity_id: id,
                timestamp: 0,
                is_dismissed: false,
            });
        }
    }

    // Findings past deadline
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.title, f.fix_deadline, f.severity, e.name, c.name
             FROM findings f
             JOIN engagements e ON e.id = f.engagement_id
             JOIN clients c ON c.id = e.client_id
             WHERE f.is_active = 1 AND f.fix_deadline IS NOT NULL
             AND f.fix_deadline < date('now')
             AND f.client_response NOT IN ('fixed', 'accepted_risk')",
        )
        .map_err(AppError::from)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(AppError::from)?;
    for row in rows {
        let (id, title, deadline, severity, engagement_name, client_name) =
            row.map_err(AppError::from)?;
        let key = format!("overdue_fix:{id}");
        if !is_dismissed(conn, &key)? {
            notifications.push(Notification {
                id: key,
                category: "deadline".to_string(),
                title: format!("Overdue fix: {title}"),
                description: format!(
                    "{} · {} · {} · Deadline: {}",
                    client_name,
                    engagement_name,
                    severity,
                    deadline.unwrap_or_default()
                ),
                entity_type: "finding".to_string(),
                entity_id: id,
                timestamp: 0,
                is_dismissed: false,
            });
        }
    }

    // Awaiting retest
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.title, e.name, c.name
             FROM findings f
             JOIN engagements e ON e.id = f.engagement_id
             JOIN clients c ON c.id = e.client_id
             WHERE f.is_active = 1 AND f.status = 'reported' AND f.retest_result = 'not_tested'",
        )
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
    for row in rows {
        let (id, title, engagement_name, client_name) = row.map_err(AppError::from)?;
        let key = format!("retest_due:{id}");
        if !is_dismissed(conn, &key)? {
            notifications.push(Notification {
                id: key,
                category: "retest_due".to_string(),
                title: format!("Retest due: {title}"),
                description: format!("{client_name} · {engagement_name}"),
                entity_type: "finding".to_string(),
                entity_id: id,
                timestamp: 0,
                is_dismissed: false,
            });
        }
    }

    // Sort by urgency
    let order = |cat: &str| match cat {
        "deadline" => 0,
        "overdue_invoice" => 1,
        "retest_due" => 2,
        _ => 3,
    };
    notifications.sort_by_key(|a| order(&a.category));
    Ok(notifications.into_iter().take(50).collect())
}

#[must_use]
pub fn do_dismiss_notification(conn: &Connection, key: &str) -> crate::error::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO dismissed_notifications (notification_key) VALUES (?)",
        params![key],
    )
    .map_err(AppError::from)?;
    Ok(())
}

#[must_use]
pub fn do_get_notification_count(conn: &Connection) -> crate::error::Result<u32> {
    let notifs = do_get_notifications(conn)?;
    Ok(notifs.len() as u32)
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

    fn make_invoice(conn: &Connection, client_id: u32, status: &str) -> u32 {
        let inv_num = format!(
            "INV-{}",
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );
        conn.execute(
            "INSERT INTO invoices (client_id, document_type, invoice_number, status, currency, created_at, updated_at)
             VALUES (?1, 'invoice', ?2, ?3, 'USD', strftime('%s','now'), strftime('%s','now'))",
            params![client_id, inv_num, status],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    fn make_finding_with_deadline(conn: &Connection, eid: u32, days_ago: i64) -> u32 {
        let endpoint = serde_json::to_string(&Vec::<String>::new()).unwrap();
        let _deadline = format!("now-{days_ago} days");
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary,
                affected_endpoints, evidence, impact_items, remediation_items,
                steps_to_reproduce, status, client_response, fix_deadline)
             VALUES (?1, 'Test finding', 'high', 'o', 's', ?2, '[]', '[]', '[]', 's', 'reported', 'no_response', date('now', ?3))",
            params![eid, endpoint, format!("-{days_ago} days")],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    #[test]
    fn test_no_notifications_on_empty_vault() {
        let conn = test_conn();
        let notifs = do_get_notifications(&conn).unwrap();
        assert!(notifs.is_empty());
    }

    #[test]
    fn test_overdue_invoice_creates_notification() {
        let conn = test_conn();
        let _ = make_engagement(&conn);
        let cid = conn
            .query_row(
                "SELECT id FROM clients ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, u32>(0),
            )
            .unwrap();
        let _ = make_invoice(&conn, cid, "sent");

        let notifs = do_get_notifications(&conn).unwrap();
        assert!(notifs.iter().any(|n| n.category == "overdue_invoice"));
    }

    #[test]
    fn test_paid_invoice_no_notification() {
        let conn = test_conn();
        let _ = make_engagement(&conn);
        let cid = conn
            .query_row(
                "SELECT id FROM clients ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, u32>(0),
            )
            .unwrap();
        let _ = make_invoice(&conn, cid, "paid");

        let notifs = do_get_notifications(&conn).unwrap();
        assert!(!notifs.iter().any(|n| n.category == "overdue_invoice"));
    }

    #[test]
    fn test_finding_past_deadline_creates_notification() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        make_finding_with_deadline(&conn, eid, 5);

        let notifs = do_get_notifications(&conn).unwrap();
        assert!(notifs.iter().any(|n| n.category == "deadline"));
    }

    #[test]
    fn test_fixed_finding_no_deadline_notification() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        let endpoint = serde_json::to_string(&Vec::<String>::new()).unwrap();
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary,
                affected_endpoints, evidence, impact_items, remediation_items,
                steps_to_reproduce, status, client_response, fix_deadline)
             VALUES (?1, 'Fixed', 'high', 'o', 's', ?2, '[]', '[]', '[]', 's', 'reported', 'fixed', date('now', '-5 days'))",
            params![eid, endpoint],
        )
        .unwrap();

        let notifs = do_get_notifications(&conn).unwrap();
        assert!(!notifs.iter().any(|n| n.category == "deadline"));
    }

    #[test]
    fn test_dismiss_removes_notification() {
        let conn = test_conn();
        let _ = make_engagement(&conn);
        let cid = conn
            .query_row(
                "SELECT id FROM clients ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, u32>(0),
            )
            .unwrap();
        let _ = make_invoice(&conn, cid, "sent");

        let notifs_before = do_get_notifications(&conn).unwrap();
        assert!(!notifs_before.is_empty());

        // Dismiss
        do_dismiss_notification(&conn, "overdue_invoice:1").unwrap();

        let notifs_after = do_get_notifications(&conn).unwrap();
        assert!(notifs_after.is_empty());
    }

    #[test]
    fn test_dismiss_dedup_via_insert_ignore() {
        let conn = test_conn();
        do_dismiss_notification(&conn, "test_key").unwrap();
        // Re-dismiss should not error
        do_dismiss_notification(&conn, "test_key").unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dismissed_notifications WHERE notification_key = 'test_key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_notification_count_matches_list() {
        let conn = test_conn();
        let eid = make_engagement(&conn);
        make_finding_with_deadline(&conn, eid, 5);
        let notifs = do_get_notifications(&conn).unwrap();
        let count = do_get_notification_count(&conn).unwrap();
        assert_eq!(count as usize, notifs.len());
    }
}

// Tauri commands
#[tauri::command]
pub fn get_notifications(state: State<AppState>) -> Result<Vec<Notification>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_notifications(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dismiss_notification(state: State<AppState>, key: String) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection().map_err(|e| e.to_string())?;
    do_dismiss_notification(conn, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_notification_count(state: State<AppState>) -> Result<u32, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_notification_count(conn).map_err(|e| e.to_string())
}
