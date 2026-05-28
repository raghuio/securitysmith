use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Application-wide state shared between Tauri commands.
/// The vault connection is `None` before unlock and `Some` after successful unlock.
pub struct AppState {
    pub vault: Arc<Mutex<Option<Connection>>>,
    pub failed_attempts: Arc<Mutex<u32>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: Arc::new(Mutex::new(None)),
            failed_attempts: Arc::new(Mutex::new(0)),
        }
    }
}
