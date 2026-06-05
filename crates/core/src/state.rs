use rusqlite::Connection;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Vault connection state.
pub enum VaultState {
    Locked,
    Unlocked {
        conn: Connection,
        key: [u8; 32],
    },
}

impl VaultState {
    /// Returns a mutable reference to the SQLite connection if the vault is
    /// unlocked. Returns `AppError::VaultLocked` otherwise.
    #[inline]
    pub fn connection(&mut self) -> crate::error::Result<&mut Connection> {
        match self {
            VaultState::Locked => Err(crate::error::AppError::VaultLocked),
            VaultState::Unlocked { conn, .. } => Ok(conn),
        }
    }

    /// Returns an immutable reference to the SQLite connection if the vault is
    /// unlocked.
    #[allow(dead_code)]
    pub fn connection_ref(&self) -> crate::error::Result<&Connection> {
        match self {
            VaultState::Locked => Err(crate::error::AppError::VaultLocked),
            VaultState::Unlocked { conn, .. } => Ok(conn),
        }
    }

    /// Returns the vault encryption key if the vault is unlocked.
    pub fn key(&self) -> crate::error::Result<&[u8; 32]> {
        match self {
            VaultState::Locked => Err(crate::error::AppError::VaultLocked),
            VaultState::Unlocked { key, .. } => Ok(key),
        }
    }
}

/// Application-wide state shared between Tauri commands.
/// The vault starts in `VaultState::Locked` and transitions to `Unlocked`
/// after successful authentication.
pub struct AppState {
    pub vault: Arc<Mutex<VaultState>>,
    pub failed_attempts: Arc<Mutex<u32>>,
    /// Timestamp of the last failed unlock attempt for cooldown calculation.
    pub last_failed_at: Arc<Mutex<Option<Instant>>>,
    /// Temporary recovery phrase held during creation/rotation until validated.
    pub pending_recovery: Arc<Mutex<Option<RecoveryState>>>,
}

/// Recovery phrase temporarily held during creation or rotation.
/// Serializes to the frontend as RecoveryInfo (see src/api/auth.ts).
#[derive(Serialize, Clone, Debug)]
pub struct RecoveryState {
    pub phrase: String,
    pub positions: Vec<u8>,
    pub is_rotation: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            vault: Arc::new(Mutex::new(VaultState::Locked)),
            failed_attempts: Arc::new(Mutex::new(0)),
            last_failed_at: Arc::new(Mutex::new(None)),
            pending_recovery: Arc::new(Mutex::new(None)),
        }
    }
}

impl AppState {
    /// Calculate remaining cooldown seconds based on failed attempts.
    /// Returns 0 if no cooldown is needed.
    pub fn cooldown_secs(&self) -> u64 {
        let attempts = *self.failed_attempts.lock().unwrap();
        if attempts < 5 {
            return 0;
        }
        let delay_mins = u64::pow(2, attempts.saturating_sub(5)).min(60);
        let delay_secs = delay_mins * 60;

        let last_failed = *self.last_failed_at.lock().unwrap();
        match last_failed {
            Some(instant) => {
                let elapsed = instant.elapsed().as_secs();
                delay_secs.saturating_sub(elapsed)
            }
            None => 0,
        }
    }

    /// Record a failed attempt, incrementing counter and updating timestamp.
    pub fn record_failed_attempt(&self) {
        let mut attempts = self.failed_attempts.lock().unwrap();
        *attempts += 1;
        drop(attempts);
        let mut last = self.last_failed_at.lock().unwrap();
        *last = Some(Instant::now());
    }

    /// Reset failed attempts after successful unlock.
    pub fn reset_failed_attempts(&self) {
        let mut attempts = self.failed_attempts.lock().unwrap();
        *attempts = 0;
        drop(attempts);
        let mut last = self.last_failed_at.lock().unwrap();
        *last = None;
    }
}
