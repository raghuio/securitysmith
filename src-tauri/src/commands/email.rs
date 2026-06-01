use crate::state::AppState;
use lettre::AsyncTransport;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Message, Tokio1Executor};
use rusqlite::OptionalExtension;
use rusqlite::params;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct FollowUpReminder {
    pub engagement_id: u32,
    pub engagement_name: String,
    pub client_name: String,
    pub reminder_type: String,
    pub due_date: String,
    pub days_overdue: i32,
}

fn read_smtp_settings(
    conn: &rusqlite::Connection,
) -> Result<(String, u16, String, String, bool, String), String> {
    let host: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.smtp_host'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_default();
    let port_str: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.smtp_port'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_else(|| "587".to_string());
    let user: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.smtp_user'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_default();
    let password: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.smtp_password'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_default();
    let use_tls: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.smtp_tls'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_else(|| "true".to_string())
        == "true";
    let from: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.smtp_from'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_default();
    let port: u16 = port_str
        .parse()
        .map_err(|_| "Invalid SMTP port.".to_string())?;
    if host.is_empty() {
        return Err("SMTP host is not configured in Settings.".to_string());
    }
    Ok((host, port, user, password, use_tls, from))
}

#[tauri::command]
pub async fn test_smtp_connection(state: State<'_, AppState>) -> Result<bool, String> {
    let (host, port, user, password, _use_tls, _from) = {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
        read_smtp_settings(conn)?
    };
    let creds = Credentials::new(user, password);
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| format!("SMTP relay: {}", e))?
            .credentials(creds)
            .port(port)
            .build();
    mailer
        .test_connection()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;
    Ok(true)
}

#[tauri::command]
pub async fn send_email(
    state: State<'_, AppState>,
    to: String,
    subject: String,
    body: String,
    attachments: Vec<String>,
    client_id: Option<u32>,
    engagement_id: Option<u32>,
) -> Result<(), String> {
    let (host, port, user, password, _use_tls, from) = {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
        read_smtp_settings(conn)?
    };

    let mut multipart = MultiPart::mixed().singlepart(
        SinglePart::builder()
            .header(
                ContentType::parse("text/plain; charset=utf-8")
                    .map_err(|_| "Invalid content type.")?,
            )
            .body(body),
    );

    let attachment_count = attachments.len();
    for path in &attachments {
        let file_bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read attachment {}: {}", path, e))?;
        let filename = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();
        let content_type = ContentType::parse(&infer_mime_type(&filename))
            .map_err(|_| format!("Invalid content type for {}", filename))?;
        let attachment = Attachment::new(filename.clone()).body(file_bytes, content_type);
        multipart = multipart.singlepart(attachment);
    }

    let email = Message::builder()
        .from(from.parse().map_err(|_| "Invalid from address.")?)
        .to(to.parse().map_err(|_| "Invalid to address.")?)
        .subject(&subject)
        .multipart(multipart)
        .map_err(|e| format!("Build email: {}", e))?;

    let creds = Credentials::new(user, password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
        .map_err(|e| format!("SMTP relay: {}", e))?
        .credentials(creds)
        .port(port)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;

    {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
        conn.execute(
            "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["system", "EMAIL_SENT", 0, "", "", format!("to={}, subject={}, attachments={}, client_id={:?}, engagement_id={:?}", to, subject, attachment_count, client_id, engagement_id)],
        )
        .map_err(|e| format!("Audit failed: {}", e))?;
    }
    Ok(())
}

fn infer_mime_type(filename: &str) -> String {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

#[tauri::command]
pub fn get_follow_up_reminders(state: State<AppState>) -> Result<Vec<FollowUpReminder>, String> {
    let guard = state
        .vault
        .lock()
        .map_err(|_| "State poisoned".to_string())?;
    let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;

    let feedback_days: i64 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.followup_feedback_days'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_else(|| "7".to_string())
        .parse()
        .unwrap_or(7);

    let retest_days: i64 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'email.followup_retest_days'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("DB: {}", e))?
        .unwrap_or_else(|| "90".to_string())
        .parse()
        .unwrap_or(90);

    let today = chrono::Local::now().date_naive();
    let mut reminders: Vec<FollowUpReminder> = Vec::new();

    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, c.name, e.updated_at
             FROM engagements e JOIN clients c ON e.client_id = c.id
             WHERE e.is_active = 1 AND e.status = 'completed'",
        )
        .map_err(|e| format!("Prepare: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| format!("Query: {}", e))?
        .filter_map(|r| r.ok());

    for (eid, ename, cname, updated_at) in rows {
        let completed_date = chrono::DateTime::from_timestamp(updated_at, 0)
            .map(|dt| dt.date_naive())
            .unwrap_or(today);
        let feedback_due = completed_date + chrono::Duration::days(feedback_days);
        let retest_due = completed_date + chrono::Duration::days(retest_days);

        if today >= feedback_due {
            let key = format!("followup_feedback:{}", eid);
            let dismissed: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM dismissed_reminders WHERE reminder_key = ?",
                    [&key],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if dismissed == 0 {
                reminders.push(FollowUpReminder {
                    engagement_id: eid,
                    engagement_name: ename.clone(),
                    client_name: cname.clone(),
                    reminder_type: "feedback".to_string(),
                    due_date: feedback_due.to_string(),
                    days_overdue: (today - feedback_due).num_days() as i32,
                });
            }
        }
        if today >= retest_due {
            let key = format!("followup_retest:{}", eid);
            let dismissed: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM dismissed_reminders WHERE reminder_key = ?",
                    [&key],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if dismissed == 0 {
                reminders.push(FollowUpReminder {
                    engagement_id: eid,
                    engagement_name: ename.clone(),
                    client_name: cname.clone(),
                    reminder_type: "retest".to_string(),
                    due_date: retest_due.to_string(),
                    days_overdue: (today - retest_due).num_days() as i32,
                });
            }
        }
    }
    Ok(reminders)
}

#[tauri::command]
pub async fn send_test_email(
    state: State<'_, AppState>,
    to: String,
    subject: String,
    body: String,
) -> Result<(), String> {
    let (host, port, user, password, _use_tls, from) = {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
        read_smtp_settings(conn)?
    };

    let email = Message::builder()
        .from(from.parse().map_err(|_| "Invalid from address.")?)
        .to(to.parse().map_err(|_| "Invalid to address.")?)
        .subject(&subject)
        .header(
            ContentType::parse("text/plain; charset=utf-8").map_err(|_| "Invalid content type.")?,
        )
        .body(body)
        .map_err(|e| format!("Build email: {}", e))?;

    let creds = Credentials::new(user, password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
        .map_err(|e| format!("SMTP relay: {}", e))?
        .credentials(creds)
        .port(port)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;

    {
        let guard = state
            .vault
            .lock()
            .map_err(|_| "State poisoned".to_string())?;
        let conn = guard.as_ref().ok_or("Vault not unlocked.".to_string())?;
        conn.execute(
            "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["system", "EMAIL_SENT", 0, "", "", format!("to={}, subject={}", to, subject)],
        )
        .map_err(|e| format!("Audit failed: {}", e))?;
    }

    Ok(())
}
