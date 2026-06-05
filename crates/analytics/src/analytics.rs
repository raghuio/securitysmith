use securitysmith_core::state::AppState;
use rusqlite::{Connection, params};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct DataPoint {
    pub label: String,
    pub value: u32,
}

#[derive(Serialize)]
pub struct TimeSeriesPoint {
    pub period: String,
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub informational: u32,
}

#[derive(Serialize)]
pub struct RemediationRate {
    pub total: u32,
    pub fixed_on_time: u32,
    pub overdue: u32,
    pub rate_percent: f64,
}

#[derive(Serialize)]
pub struct TimelineEntry {
    pub engagement_id: u32,
    pub name: String,
    pub client_name: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: String,
}

#[derive(Serialize)]
pub struct BudgetComparison {
    pub engagement_id: u32,
    pub name: String,
    pub budgeted: f64,
    pub actual: f64,
}

pub fn do_get_severity_distribution(
    conn: &Connection,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<Vec<DataPoint>, String> {
    let mut sql = String::from("SELECT severity, COUNT(*) FROM findings WHERE is_active = 1");
    let mut ps: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(d) = date_from {
        sql.push_str(" AND created_at >= strftime('%s', ?1)");
        ps.push(Box::new(d));
    }
    if let Some(d) = date_to {
        sql.push_str(" AND created_at <= strftime('%s', ?2)");
        ps.push(Box::new(d));
    }
    sql.push_str(" GROUP BY severity");
    let p_refs: Vec<&dyn rusqlite::ToSql> = ps.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(p_refs), |row| {
            Ok(DataPoint {
                label: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_top_categories(conn: &Connection, limit: u32) -> Result<Vec<DataPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT owasp_category, COUNT(*) as c FROM findings WHERE is_active = 1 AND owasp_category IS NOT NULL AND owasp_category != '' GROUP BY owasp_category ORDER BY c DESC LIMIT ?",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(DataPoint {
                label: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_findings_over_time(
    conn: &Connection,
    interval: &str,
) -> Result<Vec<TimeSeriesPoint>, String> {
    let format = match interval {
        "weekly" => "%Y-%W",
        _ => "%Y-%m",
    };
    let mut stmt = conn
        .prepare(&format!(
            "SELECT strftime('{}', datetime(created_at, 'unixepoch')) as period, severity, COUNT(*)
                 FROM findings WHERE is_active = 1 GROUP BY period, severity ORDER BY period",
            format
        ))
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(|e| format!("DB: {e}"))?;

    use std::collections::HashMap;
    let mut map: HashMap<String, TimeSeriesPoint> = HashMap::new();
    for row in rows {
        let (period, severity, count) = row.map_err(|e| format!("Row: {e}"))?;
        let entry = map.entry(period.clone()).or_insert(TimeSeriesPoint {
            period,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            informational: 0,
        });
        match severity.as_str() {
            "critical" => entry.critical += count,
            "high" => entry.high += count,
            "medium" => entry.medium += count,
            "low" => entry.low += count,
            _ => entry.informational += count,
        }
    }
    let mut items: Vec<TimeSeriesPoint> = map.into_values().collect();
    items.sort_by(|a, b| a.period.cmp(&b.period));
    Ok(items)
}

pub fn do_get_remediation_rate(conn: &Connection) -> Result<RemediationRate, String> {
    let total: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE is_active = 1 AND status = 'reported'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("DB: {e}"))?;

    let fixed_on_time: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE is_active = 1 AND status = 'reported' AND client_response = 'fixed' AND (fix_deadline IS NULL OR fix_deadline >= date('now') OR retested_at IS NOT NULL)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("DB: {e}"))?;

    let overdue: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM findings WHERE is_active = 1 AND status = 'reported' AND fix_deadline < date('now') AND client_response NOT IN ('fixed', 'accepted_risk')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("DB: {e}"))?;

    let rate = if total > 0 {
        (fixed_on_time as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(RemediationRate {
        total,
        fixed_on_time,
        overdue,
        rate_percent: rate,
    })
}

pub fn do_get_revenue_by_client(conn: &Connection) -> Result<Vec<DataPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT c.name, COALESCE(SUM(ii.amount), 0)
             FROM clients c
             LEFT JOIN invoices i ON i.client_id = c.id AND i.is_active = 1 AND i.status = 'paid'
             LEFT JOIN invoice_items ii ON ii.invoice_id = i.id
             WHERE c.is_active = 1
             GROUP BY c.id ORDER BY SUM(ii.amount) DESC",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DataPoint {
                label: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_engagement_timeline(conn: &Connection) -> Result<Vec<TimelineEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, c.name, e.start_date, e.end_date, e.status
             FROM engagements e JOIN clients c ON c.id = e.client_id
             WHERE e.is_active = 1 AND e.status IN ('active', 'scheduled', 'planned')
             ORDER BY e.start_date",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TimelineEntry {
                engagement_id: row.get(0)?,
                name: row.get(1)?,
                client_name: row.get(2)?,
                start_date: row.get(3)?,
                end_date: row.get(4)?,
                status: row.get(5)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_time_by_activity(conn: &Connection) -> Result<Vec<DataPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT activity_type, SUM(hours) FROM time_entries WHERE is_active = 1 GROUP BY activity_type",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(DataPoint {
                label: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

pub fn do_get_budget_vs_actual(conn: &Connection) -> Result<Vec<BudgetComparison>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, e.budgeted_hours, COALESCE(SUM(t.hours), 0)
             FROM engagements e LEFT JOIN time_entries t ON t.engagement_id = e.id AND t.is_active = 1
             WHERE e.is_active = 1 AND e.budgeted_hours IS NOT NULL
             GROUP BY e.id",
        )
        .map_err(|e| format!("DB: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(BudgetComparison {
                engagement_id: row.get(0)?,
                name: row.get(1)?,
                budgeted: row.get(2)?,
                actual: row.get(3)?,
            })
        })
        .map_err(|e| format!("DB: {e}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("Row: {e}"))?);
    }
    Ok(items)
}

// Tauri commands
#[tauri::command]
pub fn get_severity_distribution(
    state: State<AppState>,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<Vec<DataPoint>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_severity_distribution(conn, date_from, date_to)
}

#[tauri::command]
pub fn get_top_categories(state: State<AppState>, limit: u32) -> Result<Vec<DataPoint>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_top_categories(conn, limit)
}

#[tauri::command]
pub fn get_findings_over_time(
    state: State<AppState>,
    interval: String,
) -> Result<Vec<TimeSeriesPoint>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_findings_over_time(conn, &interval)
}

#[tauri::command]
pub fn get_remediation_rate(state: State<AppState>) -> Result<RemediationRate, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_remediation_rate(conn)
}

#[tauri::command]
pub fn get_revenue_by_client(state: State<AppState>) -> Result<Vec<DataPoint>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_revenue_by_client(conn)
}

#[tauri::command]
pub fn get_engagement_timeline(state: State<AppState>) -> Result<Vec<TimelineEntry>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_engagement_timeline(conn)
}

#[tauri::command]
pub fn get_time_by_activity(state: State<AppState>) -> Result<Vec<DataPoint>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_time_by_activity(conn)
}

#[tauri::command]
pub fn get_budget_vs_actual(state: State<AppState>) -> Result<Vec<BudgetComparison>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;
    do_get_budget_vs_actual(conn)
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

    fn seed_finding(
        conn: &Connection,
        engagement_id: u32,
        severity: &str,
        status: &str,
        owasp: Option<&str>,
    ) {
        let endpoint = serde_json::to_string(&Vec::<String>::new()).unwrap();
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary, affected_endpoints,
                evidence, impact_items, remediation_items, steps_to_reproduce, owasp_category, status)
             VALUES (?1, ?2, ?3, 'ov', 'su', ?4, '[]', '[]', '[]', 's', ?5, ?6)",
            params![engagement_id, format!("Finding {engagement_id}"), severity, endpoint, owasp, status],
        )
        .unwrap();
    }

    fn seed_engagement(conn: &Connection, name: &str) -> u32 {
        // Use a unique client name per call to avoid UNIQUE collisions
        let client_name = format!("Client-{}", uuid_like());
        conn.execute(
            "INSERT INTO clients (name, contact_email, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, NULL, NULL, '[]', 1, strftime('%s','now'), strftime('%s','now'))",
            params![client_name],
        )
        .unwrap();
        let client_id = conn.last_insert_rowid() as u32;
        conn.execute(
            "INSERT INTO engagements (client_id, name, target_area, assessment_kind, access_model,
                engagement_type, status, objectives, notes, tags, is_active, created_at, updated_at)
             VALUES (?1, ?2, 'web', 'pentest', 'auth', 'pentest', 'active', '[]', NULL, '[]', 1,
                strftime('%s','now'), strftime('%s','now'))",
            params![client_id, name],
        )
        .unwrap();
        conn.last_insert_rowid() as u32
    }

    // Simple counter-based "unique" generator for test names
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    fn uuid_like() -> u32 {
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn test_severity_distribution_groups_by_severity() {
        let conn = test_conn();
        let eid = seed_engagement(&conn, "TestEng");
        seed_finding(&conn, eid, "critical", "draft", None);
        seed_finding(&conn, eid, "critical", "draft", None);
        seed_finding(&conn, eid, "high", "draft", None);
        seed_finding(&conn, eid, "low", "draft", None);
        seed_finding(&conn, eid, "low", "draft", None);
        seed_finding(&conn, eid, "low", "draft", None);

        let dist = do_get_severity_distribution(&conn, None, None).unwrap();
        let map: std::collections::HashMap<String, u32> =
            dist.iter().map(|d| (d.label.clone(), d.value)).collect();
        assert_eq!(map.get("critical"), Some(&2));
        assert_eq!(map.get("high"), Some(&1));
        assert_eq!(map.get("low"), Some(&3));
        assert_eq!(map.get("medium"), None);
    }

    #[test]
    fn test_top_categories_respects_limit() {
        let conn = test_conn();
        let eid = seed_engagement(&conn, "TestEng");
        // Create findings with 11 different owasp categories
        for i in 0..11 {
            seed_finding(
                &conn,
                eid,
                "high",
                "draft",
                Some(&format!("A{:02}:2025", i + 1)),
            );
        }
        // Add an extra to category A01 so it has 2 hits (sorts highest)
        seed_finding(&conn, eid, "high", "draft", Some("A01:2025"));

        let top5 = do_get_top_categories(&conn, 5).unwrap();
        assert_eq!(top5.len(), 5);
        // First should be A01 (has 2 findings)
        assert_eq!(top5[0].label, "A01:2025");
        assert_eq!(top5[0].value, 2);

        let top3 = do_get_top_categories(&conn, 3).unwrap();
        assert_eq!(top3.len(), 3);
    }

    #[test]
    fn test_findings_over_time_groups_by_month() {
        let conn = test_conn();
        let eid = seed_engagement(&conn, "TestEng");
        seed_finding(&conn, eid, "high", "draft", None);
        seed_finding(&conn, eid, "low", "draft", None);

        let monthly = do_get_findings_over_time(&conn, "monthly").unwrap();
        assert!(!monthly.is_empty());
        // All severities in one period — should sum to 2
        let total: u32 = monthly[0].critical
            + monthly[0].high
            + monthly[0].medium
            + monthly[0].low
            + monthly[0].informational;
        assert_eq!(total, 2);
    }

    #[test]
    fn test_remediation_rate_calculates_correctly() {
        let conn = test_conn();
        let eid = seed_engagement(&conn, "TestEng");

        // Helper: create a finding with both status and client_response
        let endpoint = serde_json::to_string(&Vec::<String>::new()).unwrap();
        let make = |conn: &Connection, eid: u32, status: &str, cr: &str, deadline: Option<&str>| {
            conn.execute(
                "INSERT INTO findings (engagement_id, title, severity, overview, summary,
                    affected_endpoints, evidence, impact_items, remediation_items,
                    steps_to_reproduce, status, client_response, fix_deadline)
                 VALUES (?1, ?2, 'high', 'o', 's', ?3, '[]', '[]', '[]', 's', ?4, ?5, ?6)",
                params![eid, format!("F-{status}"), endpoint, status, cr, deadline],
            )
            .unwrap();
        };

        // 3 reported + client_response=fixed (fixed_on_time)
        make(&conn, eid, "reported", "fixed", None);
        make(&conn, eid, "reported", "fixed", None);
        make(&conn, eid, "reported", "fixed", None);
        // 1 reported + no_response + past deadline (overdue)
        make(&conn, eid, "reported", "no_response", Some("2020-01-01"));
        // 1 reported + accepted_risk + past deadline (NOT overdue per the query)
        make(&conn, eid, "reported", "accepted_risk", Some("2020-01-01"));
        // 1 draft
        make(&conn, eid, "draft", "no_response", None);

        let r = do_get_remediation_rate(&conn).unwrap();
        // total counts reported findings only (5: 3 fixed + 1 overdue + 1 accepted_risk)
        assert_eq!(r.total, 5);
        assert_eq!(r.fixed_on_time, 3, "3 with client_response=fixed");
        // 1 overdue: only the no_response + past-deadline one
        assert_eq!(r.overdue, 1);
        // 3/5 = 60%
        assert!((r.rate_percent - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_engagement_timeline_returns_active_engagements() {
        let conn = test_conn();
        seed_engagement(&conn, "Eng1");
        seed_engagement(&conn, "Eng2");
        let tl = do_get_engagement_timeline(&conn).unwrap();
        assert_eq!(tl.len(), 2);
        assert!(tl.iter().any(|e| e.name == "Eng1"));
    }

    #[test]
    fn test_budget_vs_actual_returns_zero_when_no_time_entries() {
        let conn = test_conn();
        seed_engagement(&conn, "Eng1");
        let budget = do_get_budget_vs_actual(&conn).unwrap();
        // No time entries -> no rows
        assert!(budget.is_empty());
    }

    #[test]
    fn test_date_range_filter_narrows_severity_distribution() {
        let conn = test_conn();
        let eid = seed_engagement(&conn, "TestEng");
        // Backdate all but one finding by 2 years
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary, affected_endpoints,
                evidence, impact_items, remediation_items, steps_to_reproduce, status)
             SELECT engagement_id, title, severity, overview, summary, affected_endpoints,
                evidence, impact_items, remediation_items, steps_to_reproduce, status
             FROM findings WHERE 1=0",
            params![],
        )
        .unwrap();
        // First: create 2 backdated findings by directly inserting with explicit created_at
        let endpoint = serde_json::to_string(&Vec::<String>::new()).unwrap();
        let old_ts = now_ts() - (2 * 365 * 24 * 60 * 60);
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary,
                affected_endpoints, evidence, impact_items, remediation_items,
                steps_to_reproduce, status, created_at)
             VALUES (?1, 'old1', 'high', 'o', 's', ?2, '[]', '[]', '[]', 's', 'draft', ?3)",
            params![eid, endpoint, old_ts],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO findings (engagement_id, title, severity, overview, summary,
                affected_endpoints, evidence, impact_items, remediation_items,
                steps_to_reproduce, status)
             VALUES (?1, 'new1', 'critical', 'o', 's', ?2, '[]', '[]', '[]', 's', 'draft')",
            params![eid, endpoint],
        )
        .unwrap();

        // Without filter: 2 findings total
        let all = do_get_severity_distribution(&conn, None, None).unwrap();
        let total_all: u32 = all.iter().map(|d| d.value).sum();
        assert_eq!(total_all, 2);

        // With "from 1 year ago" filter: only the recent finding
        let one_year_ago_ts = now_ts() - (365 * 24 * 60 * 60);
        let one_year_ago_date = ts_to_iso(one_year_ago_ts);
        let recent = do_get_severity_distribution(&conn, Some(one_year_ago_date), None).unwrap();
        let total_recent: u32 = recent.iter().map(|d| d.value).sum();
        assert_eq!(total_recent, 1);
    }

    fn now_ts() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    fn ts_to_iso(ts: i64) -> String {
        let days = ts / 86400;
        let (y, m, d) = civil_from_days(days);
        format!("{:04}-{:02}-{:02}", y, m, d)
    }

    fn civil_from_days(z: i64) -> (i32, u32, u32) {
        let z = z + 719468;
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = (z - era * 146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
        let y = (if m <= 2 { y + 1 } else { y }) as i32;
        (y, m, d)
    }
}
