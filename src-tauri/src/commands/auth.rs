use crate::db;
use crate::state::AppState;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

/// Check whether the vault file exists on disk.
#[tauri::command]
pub fn is_vault_initialized(app_handle: AppHandle) -> Result<bool, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;
    Ok(data_dir.join("vault.db").exists())
}

/// Create a new encrypted vault with the given master password.
/// Fails if a vault already exists.
#[tauri::command]
pub fn create_vault(
    app_handle: AppHandle,
    state: State<AppState>,
    password: String,
) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    if data_dir.join("vault.db").exists() {
        return Err("Vault already exists".to_string());
    }

    let conn = db::open_vault(&data_dir, &password)
        .map_err(|e| format!("Failed to create vault: {}", e))?;

    db::init_db(&conn).map_err(|e| format!("Failed to initialize vault: {}", e))?;

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    *vault = Some(conn);

    *state
        .failed_attempts
        .lock()
        .map_err(|_| "Internal state error".to_string())? = 0;

    Ok(())
}

/// Unlock the existing vault with the master password.
/// After 5 consecutive failures, progressive exponential delay is enforced
/// (1s → 2s → 4s → 8s ... capped at 60s). Delay resets on correct unlock.
#[tauri::command]
pub fn unlock_vault(
    app_handle: AppHandle,
    state: State<AppState>,
    password: String,
) -> Result<(), String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    if !data_dir.join("vault.db").exists() {
        return Err("Vault not found".to_string());
    }

    // Check and enforce progressive delay on repeated failures
    {
        let attempts = state
            .failed_attempts
            .lock()
            .map_err(|_| "Internal state error".to_string())?;
        if *attempts >= 5 {
            let delay_mins = u64::pow(2, (*attempts).saturating_sub(5)).min(60);
            thread::sleep(Duration::from_secs(delay_mins * 60));
        }
    }

    match db::open_vault(&data_dir, &password) {
        Ok(conn) => {
            let mut vault = state
                .vault
                .lock()
                .map_err(|_| "Internal state error".to_string())?;
            *vault = Some(conn);

            *state
                .failed_attempts
                .lock()
                .map_err(|_| "Internal state error".to_string())? = 0;

            Ok(())
        }
        Err(e) => {
            *state
                .failed_attempts
                .lock()
                .map_err(|_| "Internal state error".to_string())? += 1;
            Err(format!("Failed to unlock vault: {}", e))
        }
    }
}
