use crate::state::AppState;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::time::Duration;
use tauri::State;

#[derive(Serialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
}

/// Read a single setting value by key. Returns `None` if the key does not exist.
/// Requires the vault to be unlocked.
#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;

    let value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(value)
}

/// Write (or update) a setting value and log the change to `audit_log`.
/// Requires the vault to be unlocked.
#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_mut().ok_or("Vault not unlocked")?;

    let old_value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| format!("Database error: {}", e))?;

    conn.execute(
        "INSERT INTO settings (key, value, updated_at)
         VALUES (?1, ?2, strftime('%s', 'now'))
         ON CONFLICT(key) DO UPDATE SET
             value = excluded.value,
             updated_at = strftime('%s', 'now')",
        [&key, &value],
    )
    .map_err(|e| format!("Database error: {}", e))?;

    let action = if old_value.is_some() {
        "UPDATE"
    } else {
        "INSERT"
    };

    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "settings",
            action,
            &key,
            old_value.as_deref(),
            &value,
            "set_setting command"
        ],
    )
    .map_err(|e| format!("Database error: {}", e))?;

    Ok(())
}

/// List all settings as key/value pairs.
/// Requires the vault to be unlocked.
#[tauri::command]
pub fn list_settings(state: State<AppState>) -> Result<Vec<SettingEntry>, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.as_ref().ok_or("Vault not unlocked")?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| format!("Database error: {}", e))?;

    let results: Vec<SettingEntry> = stmt
        .query_map([], |row| {
            Ok(SettingEntry {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| format!("Database error: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(results)
}

/// Test connectivity to a local Ollama instance.
/// Validates the URL scheme and performs a lightweight GET to `/api/tags`.
#[tauri::command]
pub async fn test_ollama_connection(url: String) -> Result<bool, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|e| format!("Invalid URL: {}", e))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("URL must use http or https scheme".to_string());
    }

    let api_url = parsed
        .join("/api/tags")
        .map_err(|e| format!("Invalid URL path: {}", e))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| format!("Ollama connection failed: {}", e))?;

    Ok(resp.status().is_success())
}
