use crate::error::AppError;
use crate::state::AppState;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use std::time::Duration;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
}

/// Read a single setting value by key. Returns `None` if the key does not exist.
/// Requires the vault to be unlocked.
#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    let value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(AppError::from)?;

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
    let conn = vault.connection().map_err(|e| e.to_string())?;

    let old_value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [&key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(AppError::from)?;

    conn.execute(
        "INSERT INTO settings (key, value, updated_at)
         VALUES (?1, ?2, strftime('%s', 'now'))
         ON CONFLICT(key) DO UPDATE SET
             value = excluded.value,
             updated_at = strftime('%s', 'now')",
        [&key, &value],
    )
    .map_err(AppError::from)?;

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
    .map_err(AppError::from)?;

    Ok(())
}

/// List all settings as key/value pairs.
/// Requires the vault to be unlocked.
#[tauri::command]
pub fn list_settings(state: State<AppState>) -> Result<Vec<SettingEntry>, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(AppError::from)?;

    let results: Vec<SettingEntry> = stmt
        .query_map([], |row| {
            Ok(SettingEntry {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;

    Ok(results)
}

/// Read the stored appearance theme for boot-time provider sync.
/// Returns "light" or "dark". Defaults to "light" if missing or invalid.
/// Requires the vault to be unlocked.
#[tauri::command]
pub fn get_boot_theme(state: State<AppState>) -> Result<String, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    let conn = vault.connection_ref().map_err(|e| e.to_string())?;

    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'appearance.theme'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)?;

    match value.as_deref() {
        Some("dark") => Ok("dark".to_string()),
        Some("light") => Ok("light".to_string()),
        Some(other) => {
            eprintln!(
                "Warning: invalid theme value '{}', defaulting to light",
                other
            );
            Ok("light".to_string())
        }
        None => Ok("light".to_string()),
    }
}

/// Test connectivity to a local Ollama instance.
/// Validates the URL scheme and performs a lightweight GET to `/api/tags`.
#[tauri::command]
pub async fn test_ollama_connection(url: String) -> Result<bool, String> {
    let parsed =
        reqwest::Url::parse(&url).map_err(|e| AppError::Generic(format!("Invalid URL: {e}")))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("URL must use http or https scheme".to_string());
    }

    let api_url = parsed
        .join("/api/tags")
        .map_err(|e| AppError::Generic(format!("Invalid URL path: {e}")))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(AppError::from)?;

    let resp = client.get(api_url).send().await.map_err(AppError::from)?;

    Ok(resp.status().is_success())
}
