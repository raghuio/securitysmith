use crate::crypto;
use crate::db;
use crate::state::{AppState, RecoveryState};
use rusqlite::params;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

const VAULT_VERSION: u8 = 2;
const VAULT_VERSION_FILENAME: &str = "vault_version.txt";

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
/// Uses Argon2id key derivation. Fails if a vault already exists.
/// Returns recovery phrase and validation positions for the user to record.
#[tauri::command]
pub fn create_vault(
    app_handle: AppHandle,
    state: State<AppState>,
    password: String,
) -> Result<RecoveryState, String> {
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

    let salt = crypto::generate_salt();
    let key = crypto::derive_key(&password, &salt)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    // Write version file: "2:<hex_salt>"
    let version_data = format!("{}:{}", VAULT_VERSION, hex::encode(salt));
    std::fs::write(data_dir.join(VAULT_VERSION_FILENAME), version_data)
        .map_err(|e| format!("Failed to write vault version: {}", e))?;

    let conn =
        db::open_vault(&data_dir, &key).map_err(|e| format!("Failed to create vault: {}", e))?;

    db::init_db(&conn).map_err(|e| format!("Failed to initialize vault: {}", e))?;

    // Store salt in vault_meta for internal reference
    conn.execute(
        "INSERT INTO vault_meta (id, key_salt, key_version, created_at) VALUES (1, ?1, 1, strftime('%s', 'now'))",
        params![&salt[..]],
    )
    .map_err(|e| format!("Failed to store vault metadata: {}", e))?;

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    *vault = Some(conn);

    *state
        .vault_key
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(key);

    *state
        .failed_attempts
        .lock()
        .map_err(|_| "Internal state error".to_string())? = 0;

    // Generate recovery phrase and validation positions
    let phrase = crypto::generate_recovery_phrase();
    let positions = generate_positions();

    let recovery = RecoveryState {
        phrase,
        positions: positions.clone(),
        is_rotation: false,
    };

    *state
        .pending_recovery
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(recovery.clone());

    Ok(recovery)
}

/// Unlock the existing vault with the master password.
/// After 5 consecutive failures, progressive exponential delay is enforced
/// (1m → 2m → 4m → 8m ... capped at 60m). Delay resets on correct unlock.
///
/// Returns `Ok(true)` when the vault was created before PROP-002 and has
/// no recovery envelope yet — the frontend should call
/// `rotate_recovery_phrase` to bootstrap recovery. Returns `Ok(false)`
/// when recovery is already configured (new vaults, post-rotation,
/// or after a successful `change_master_password`).
#[tauri::command]
pub fn unlock_vault(
    app_handle: AppHandle,
    state: State<AppState>,
    password: String,
) -> Result<bool, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    if !data_dir.join("vault.db").exists() {
        return Err("Vault not found".to_string());
    }

    // Verify vault version
    let version_content = std::fs::read_to_string(data_dir.join(VAULT_VERSION_FILENAME))
        .map_err(|_| "Vault format not supported. Please create a new vault.".to_string())?;
    let parts: Vec<&str> = version_content.trim().split(':').collect();
    if parts.len() != 2 || parts[0] != "2" {
        return Err("Vault format not supported. Please create a new vault.".to_string());
    }

    let salt_hex = parts[1];
    let salt_vec = hex::decode(salt_hex).map_err(|_| "Vault metadata corrupted".to_string())?;
    if salt_vec.len() != 16 {
        return Err("Vault metadata corrupted: invalid salt".to_string());
    }
    let salt: [u8; 16] = salt_vec
        .try_into()
        .map_err(|_| "Vault metadata corrupted: invalid salt length".to_string())?;

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

    let key = crypto::derive_key(&password, &salt)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    let conn = match db::open_vault(&data_dir, &key) {
        Ok(conn) => conn,
        Err(e) => {
            *state
                .failed_attempts
                .lock()
                .map_err(|_| "Internal state error".to_string())? += 1;
            return Err(format!("Failed to unlock vault: {}", e));
        }
    };

    // Ensure all migrations are applied (e.g. recovery table added after vault creation)
    db::init_db(&conn).map_err(|e| format!("Failed to apply migrations: {}", e))?;

    // Detect whether the recovery envelope is already configured. For
    // vaults created before PROP-002, the in-vault `recovery` table is
    // present (added by migration) but the singleton row is missing —
    // the user must bootstrap recovery by calling `rotate_recovery_phrase`
    // from the UI. The external `recovery_envelope.bin` is the source
    // of truth; if it does not exist, recovery is not configured.
    let envelope_path = data_dir.join("recovery_envelope.bin");
    let recovery_setup_required = !envelope_path.exists();

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    *vault = Some(conn);

    *state
        .vault_key
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(key);

    *state
        .failed_attempts
        .lock()
        .map_err(|_| "Internal state error".to_string())? = 0;

    Ok(recovery_setup_required)
}

/// Result of validating recovery words.
#[derive(serde::Serialize)]
pub struct ValidationResult {
    pub success: bool,
    pub new_phrase: Option<String>,
    pub new_positions: Option<Vec<u8>>,
}

/// Validate the 3 randomly selected recovery words.
/// On success, encrypts and stores the recovery envelope.
/// On failure, generates a completely new phrase and returns it.
#[tauri::command]
pub fn validate_recovery_words(
    app_handle: AppHandle,
    state: State<AppState>,
    phrase: String,
    positions: Vec<u8>,
    words: Vec<String>,
) -> Result<ValidationResult, String> {
    let pending = {
        let guard = state
            .pending_recovery
            .lock()
            .map_err(|_| "Internal state error".to_string())?;
        guard
            .clone()
            .ok_or("No recovery phrase pending validation.".to_string())?
    };

    if phrase != pending.phrase {
        return Err("Provided phrase does not match pending recovery state.".to_string());
    }

    if positions != pending.positions {
        return Err("Validation positions do not match pending recovery state.".to_string());
    }

    if positions.len() != 3 || words.len() != 3 {
        return Err("Exactly 3 words must be provided.".to_string());
    }

    let phrase_words = crypto::parse_phrase(&pending.phrase);

    let all_correct = positions.iter().zip(words.iter()).all(|(pos, word)| {
        let trimmed = word.trim().to_lowercase();
        if trimmed.is_empty() {
            return false;
        }
        phrase_words
            .get(*pos as usize)
            .map(|w| w == &trimmed)
            .unwrap_or(false)
    });

    if !all_correct {
        let new_phrase = crypto::generate_recovery_phrase();
        let new_positions = generate_positions();

        let new_recovery = RecoveryState {
            phrase: new_phrase.clone(),
            positions: new_positions.clone(),
            is_rotation: pending.is_rotation,
        };

        *state
            .pending_recovery
            .lock()
            .map_err(|_| "Internal state error".to_string())? = Some(new_recovery);

        return Ok(ValidationResult {
            success: false,
            new_phrase: Some(new_phrase),
            new_positions: Some(new_positions),
        });
    }

    // Success — encrypt vault key into recovery envelope
    let vault_key = {
        let guard = state
            .vault_key
            .lock()
            .map_err(|_| "Internal state error".to_string())?;
        guard.ok_or("Vault key not available".to_string())?
    };

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    let salt = crypto::generate_salt();
    let recovery_key = crypto::derive_recovery_key(&pending.phrase, &salt)
        .map_err(|e| format!("Recovery key derivation failed: {}", e))?;

    let (nonce, ciphertext) = crypto::encrypt_envelope(&vault_key, &recovery_key)
        .map_err(|e| format!("Envelope encryption failed: {}", e))?;

    let mut envelope = nonce.to_vec();
    envelope.extend_from_slice(&ciphertext);

    // Persist to external file and internal vault table
    db::save_recovery_envelope(&data_dir, &salt, &envelope)?;

    {
        let mut vault_guard = state
            .vault
            .lock()
            .map_err(|_| "Internal state error".to_string())?;
        let conn = vault_guard
            .as_mut()
            .ok_or("Vault not unlocked".to_string())?;
        db::store_recovery(conn, &envelope, &salt)?;

        // Audit log actions: "INSERT" for first-time setup, "UPDATE" for
        // rotation, "RECOVERY_UNLOCK" for recovery-phrase unlocks.
        // `context` distinguishes the three: "vault_creation",
        // "recovery_phrase_rotation", "recovery_unlock".
        let action = if pending.is_rotation {
            "UPDATE"
        } else {
            "INSERT"
        };
        let context = if pending.is_rotation {
            "recovery_phrase_rotation"
        } else {
            "vault_creation"
        };
        conn.execute(
            "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "recovery",
                action,
                "recovery",
                if pending.is_rotation {
                    Some("old phrase invalidated")
                } else {
                    None::<&str>
                },
                "new recovery envelope stored",
                context
            ],
        )
        .map_err(|e| format!("Audit log failed: {}", e))?;
    }

    // Clear pending state
    *state
        .pending_recovery
        .lock()
        .map_err(|_| "Internal state error".to_string())? = None;

    Ok(ValidationResult {
        success: true,
        new_phrase: None,
        new_positions: None,
    })
}

/// Recover the vault using a 12-word BIP-39 recovery phrase.
/// Protected by the same progressive delay as password unlock.
#[tauri::command]
pub fn recover_vault(
    app_handle: AppHandle,
    state: State<AppState>,
    phrase: String,
) -> Result<(), String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    if !data_dir.join("vault.db").exists() {
        return Err("Vault not found".to_string());
    }

    if !crypto::is_valid_recovery_phrase(&phrase) {
        return Err("Recovery phrase must be exactly 12 words.".to_string());
    }

    // Progressive delay (shared with password unlock; counter is shared)
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

    // Load recovery envelope from external file
    let load_result = (|| -> Result<[u8; 32], String> {
        let (envelope, salt) = db::load_recovery_envelope(&data_dir)?;
        let recovery_key = crypto::derive_recovery_key(&phrase, &salt)
            .map_err(|e| format!("Recovery key derivation failed: {}", e))?;
        crypto::decrypt_envelope(&envelope, &recovery_key)
            .map_err(|_| "Incorrect recovery phrase. Please try again.".to_string())
    })();

    let vault_key = match load_result {
        Ok(key) => key,
        Err(e) => {
            // Wrong phrase, missing file, decryption failure — all count as
            // a failed unlock attempt and tick the shared counter.
            *state
                .failed_attempts
                .lock()
                .map_err(|_| "Internal state error".to_string())? += 1;
            return Err(e);
        }
    };

    let conn = db::open_vault(&data_dir, &vault_key)
        .map_err(|e| format!("Failed to unlock vault: {}", e))?;

    // Ensure all migrations are applied
    db::init_db(&conn).map_err(|e| format!("Failed to apply migrations: {}", e))?;

    // Log recovery unlock to audit_log
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "recovery",
            "RECOVERY_UNLOCK",
            "recovery",
            None::<&str>,
            None::<&str>,
            "recovery_unlock"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    *vault = Some(conn);

    *state
        .vault_key
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(vault_key);

    *state
        .failed_attempts
        .lock()
        .map_err(|_| "Internal state error".to_string())? = 0;

    Ok(())
}

/// Rotate the recovery phrase. Requires current password for verification.
/// Returns a new recovery phrase and validation positions.
/// The envelope is NOT stored until `validate_recovery_words` succeeds.
#[tauri::command]
pub fn rotate_recovery_phrase(
    app_handle: AppHandle,
    state: State<AppState>,
    password: String,
) -> Result<RecoveryState, String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    if !data_dir.join("vault.db").exists() {
        return Err("Vault not found".to_string());
    }

    // Verify vault version
    let version_content = std::fs::read_to_string(data_dir.join(VAULT_VERSION_FILENAME))
        .map_err(|_| "Vault format not supported. Please create a new vault.".to_string())?;
    let parts: Vec<&str> = version_content.trim().split(':').collect();
    if parts.len() != 2 || parts[0] != "2" {
        return Err("Vault format not supported. Please create a new vault.".to_string());
    }

    let salt_hex = parts[1];
    let salt_vec = hex::decode(salt_hex).map_err(|_| "Vault metadata corrupted".to_string())?;
    if salt_vec.len() != 16 {
        return Err("Vault metadata corrupted: invalid salt".to_string());
    }
    let salt: [u8; 16] = salt_vec
        .try_into()
        .map_err(|_| "Vault metadata corrupted: invalid salt length".to_string())?;

    let key = crypto::derive_key(&password, &salt)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    // Verify password by opening vault; wrong password ticks the shared
    // brute-force counter (consistent with `unlock_vault`).
    let _conn = match db::open_vault(&data_dir, &key) {
        Ok(conn) => conn,
        Err(_) => {
            *state
                .failed_attempts
                .lock()
                .map_err(|_| "Internal state error".to_string())? += 1;
            return Err("Incorrect password.".to_string());
        }
    };
    drop(_conn);

    // Generate new recovery phrase
    let phrase = crypto::generate_recovery_phrase();
    let positions = generate_positions();

    let recovery = RecoveryState {
        phrase: phrase.clone(),
        positions: positions.clone(),
        is_rotation: true,
    };

    *state
        .pending_recovery
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(recovery.clone());

    Ok(recovery)
}

// ─────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────

/// Change the master password. Re-keys the SQLCipher vault with a new
/// Argon2id-derived key and forces regeneration of the recovery phrase.
/// On success, returns a new recovery phrase + positions. The caller must
/// display the new phrase to the user and complete validation via
/// `validate_recovery_words` before the change is finalised: the recovery
/// envelope from the old password is invalidated up-front, so until the
/// user validates the new phrase, recovery is unavailable (but the new
/// password still works).
#[tauri::command]
pub fn change_master_password(
    app_handle: AppHandle,
    state: State<AppState>,
    old_password: String,
    new_password: String,
) -> Result<RecoveryState, String> {
    if new_password.len() < 8 {
        return Err("New password must be at least 8 characters".to_string());
    }
    if old_password == new_password {
        return Err("New password must differ from the current password.".to_string());
    }

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data directory: {}", e))?;

    if !data_dir.join("vault.db").exists() {
        return Err("Vault not found".to_string());
    }

    // Verify old password by reading the current vault metadata.
    let version_content = std::fs::read_to_string(data_dir.join(VAULT_VERSION_FILENAME))
        .map_err(|_| "Vault format not supported. Please create a new vault.".to_string())?;
    let parts: Vec<&str> = version_content.trim().split(':').collect();
    if parts.len() != 2 || parts[0] != "2" {
        return Err("Vault format not supported. Please create a new vault.".to_string());
    }
    let salt_vec = hex::decode(parts[1]).map_err(|_| "Vault metadata corrupted".to_string())?;
    if salt_vec.len() != 16 {
        return Err("Vault metadata corrupted: invalid salt".to_string());
    }
    let old_salt: [u8; 16] = salt_vec
        .try_into()
        .map_err(|_| "Vault metadata corrupted: invalid salt length".to_string())?;

    let old_key = crypto::derive_key(&old_password, &old_salt)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    // Open the vault with the OLD key to verify the password and grab a
    // connection we can re-key in place. A wrong password here ticks the
    // shared brute-force counter.
    let conn = match db::open_vault(&data_dir, &old_key) {
        Ok(c) => c,
        Err(_) => {
            *state
                .failed_attempts
                .lock()
                .map_err(|_| "Internal state error".to_string())? += 1;
            return Err("Incorrect current password.".to_string());
        }
    };

    // Derive the new key and re-key the SQLCipher database.
    let new_salt = crypto::generate_salt();
    let new_key = crypto::derive_key(&new_password, &new_salt)
        .map_err(|e| format!("Key derivation failed: {}", e))?;

    db::rekey_vault(&conn, &old_key, &new_key)
        .map_err(|e| format!("Failed to re-key vault: {}", e))?;

    // Persist the new salt and bump key version. SQLCipher PRAGMA rekey
    // rewrites every page of the DB; the in-vault vault_meta is now
    // encrypted with the new key and the UPDATE works.
    conn.execute(
        "UPDATE vault_meta SET key_salt = ?1, key_version = key_version + 1 WHERE id = 1",
        params![&new_salt[..]],
    )
    .map_err(|e| format!("Failed to update vault metadata: {}", e))?;

    // Write the new version file: "2:<hex_new_salt>". The file is the
    // source of truth used by `unlock_vault` to reproduce the new key.
    let version_data = format!("{}:{}", VAULT_VERSION, hex::encode(new_salt));
    std::fs::write(data_dir.join(VAULT_VERSION_FILENAME), version_data)
        .map_err(|e| format!("Failed to write vault version: {}", e))?;

    // Invalidate the old recovery envelope (file + in-vault row). The
    // user MUST set up a new recovery phrase via the returned
    // `RecoveryState` + `validate_recovery_words` flow.
    let envelope_path = data_dir.join("recovery_envelope.bin");
    if envelope_path.exists() {
        let _ = std::fs::remove_file(&envelope_path);
    }
    conn.execute("DELETE FROM recovery WHERE id = 1", params![])
        .map_err(|e| format!("Failed to clear recovery envelope: {}", e))?;

    // Audit log: record the password change.
    conn.execute(
        "INSERT INTO audit_log (table_name, action, record_id, old_value, new_value, context)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "vault_meta",
            "UPDATE",
            "1",
            "password changed (old key rotated)",
            "new salt written, key_version bumped",
            "change_master_password"
        ],
    )
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // Swap the live vault connection to the re-keyed one and update
    // `state.vault_key`. The vault is now re-keyed; recovery is
    // unavailable until the user validates a new recovery phrase.
    let mut vault_guard = state
        .vault
        .lock()
        .map_err(|_| "Internal state error".to_string())?;
    *vault_guard = Some(conn);

    *state
        .vault_key
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(new_key);

    *state
        .failed_attempts
        .lock()
        .map_err(|_| "Internal state error".to_string())? = 0;

    drop(vault_guard);

    // Generate a fresh recovery phrase and stage it for validation.
    let phrase = crypto::generate_recovery_phrase();
    let positions = generate_positions();

    let recovery = RecoveryState {
        phrase,
        positions,
        is_rotation: true,
    };

    *state
        .pending_recovery
        .lock()
        .map_err(|_| "Internal state error".to_string())? = Some(recovery.clone());

    Ok(recovery)
}

/// Generate 3 distinct random positions from [0..11], sorted.
fn generate_positions() -> Vec<u8> {
    let mut positions: Vec<u8> = (0..12).collect();
    for i in (1..12).rev() {
        let j = (rand::random::<u8>() % ((i + 1) as u8)) as usize;
        positions.swap(i, j);
    }
    positions.truncate(3);
    positions.sort();
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_positions_returns_three_distinct_sorted() {
        // Generate many times; every result must be 3 distinct values in 0..12.
        for _ in 0..50 {
            let p = generate_positions();
            assert_eq!(p.len(), 3);
            assert!(
                p.windows(2).all(|w| w[0] < w[1]),
                "positions must be sorted"
            );
            assert!(p.iter().all(|&x| x < 12));
        }
    }

    #[test]
    fn test_validate_positions_valid_input() {
        let phrase = crypto::generate_recovery_phrase();
        assert!(crypto::is_valid_recovery_phrase(&phrase));
    }

    #[test]
    fn test_validate_positions_invalid_word_count() {
        assert!(!crypto::is_valid_recovery_phrase("abandon ability able"));
        assert!(!crypto::is_valid_recovery_phrase(""));
    }
}
