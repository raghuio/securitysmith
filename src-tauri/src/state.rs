use rusqlite::Connection;
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// Application-wide state shared between Tauri commands.
/// The vault connection is `None` before unlock and `Some` after successful unlock.
pub struct AppState {
    pub vault: Arc<Mutex<Option<Connection>>>,
    pub vault_key: Arc<Mutex<Option<[u8; 32]>>>,
    pub failed_attempts: Arc<Mutex<u32>>,
    /// Temporary recovery phrase held during creation/rotation until validated.
    pub pending_recovery: Arc<Mutex<Option<RecoveryState>>>,
}

/// Recovery phrase temporarily held during creation or rotation.
/// Serializes to the frontend as `RecoveryInfo` (see `src/api/auth.ts`).
#[derive(Serialize, Clone, Debug)]
pub struct RecoveryState {
    pub phrase: String,
    pub positions: Vec<u8>,
    pub is_rotation: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: Arc::new(Mutex::new(None)),
            vault_key: Arc::new(Mutex::new(None)),
            failed_attempts: Arc::new(Mutex::new(0)),
            pending_recovery: Arc::new(Mutex::new(None)),
        }
    }
}
