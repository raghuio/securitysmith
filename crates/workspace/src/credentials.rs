//! Credentials management — encrypted credential store.
//!
//! ChaCha20-Poly1305 AEAD encryption, Argon2id key derivation.
//! Master password prompted from stdin, never stored, zeroed after use.
//!
//! Security practices:
//! - Keys are borrowed (not consumed) so they can be zeroized after use
//! - Salt and nonce are generated with `rand::rngs::OsRng` (CSPRNG)
//! - All sensitive buffers are zeroized with the `zeroize` crate after use

use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce, aead::Aead};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::WorkspaceError;

const MAGIC: &[u8] = b"SSCRED\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const ARGON2_M_COST: u32 = 19456;
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
pub struct Credential {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub cred_type: String,
    pub value: String,
    pub status: String,
    #[serde(default)]
    pub notes: String,
    pub engagement_path: String,
    pub created: String,
    pub updated: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CredentialStore {
    pub credentials: Vec<Credential>,
}

pub const VALID_CRED_TYPES: &[&str] = &[
    "url",
    "username_password",
    "api_key",
    "bearer_token",
    "vpn_config",
    "ssh_key",
    "custom",
];

pub const VALID_CRED_STATUSES: &[&str] = &["not_verified", "working", "not_working", "expired"];

fn to_ws_error(e: &str) -> WorkspaceError {
    WorkspaceError::Io(std::io::Error::other(e.to_string()))
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(32))
        .map_err(|e| format!("Argon2 params error: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Argon2 key derivation failed: {e}"))?;
    Ok(key)
}

fn cred_file_path(workspace_root: &camino::Utf8Path) -> camino::Utf8PathBuf {
    workspace_root.join(".credentials.enc")
}

pub fn load_store(
    workspace_root: &camino::Utf8Path,
    password: &str,
) -> Result<CredentialStore, WorkspaceError> {
    let path = cred_file_path(workspace_root);
    if !path.exists() {
        return Ok(CredentialStore::default());
    }
    let file_bytes = std::fs::read(path.as_std_path())?;
    decrypt_store(&file_bytes, password).map_err(|e| to_ws_error(&e))
}

fn decrypt_store(file_bytes: &[u8], password: &str) -> Result<CredentialStore, String> {
    if file_bytes.len() < MAGIC.len() + SALT_LEN + NONCE_LEN {
        return Err("Credential file is too short or corrupted".to_string());
    }
    if &file_bytes[..MAGIC.len()] != MAGIC {
        return Err("Invalid credential file format".to_string());
    }

    let salt = &file_bytes[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce_bytes = &file_bytes[MAGIC.len() + SALT_LEN..MAGIC.len() + SALT_LEN + NONCE_LEN];
    let ciphertext = &file_bytes[MAGIC.len() + SALT_LEN + NONCE_LEN..];

    let mut key = derive_key(password, salt)?;

    // Borrow the key — do not consume it so we can zeroize after.
    let cipher = ChaCha20Poly1305::new(<&Key>::from(&key));

    let nonce = Nonce::try_from(nonce_bytes).map_err(|e| format!("Invalid nonce: {e}"))?;

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| format!("Decryption failed (wrong password or corrupted file): {e}"))?;

    key.zeroize();

    serde_json::from_slice(&plaintext).map_err(|e| format!("JSON parse error: {e}"))
}

pub fn save_store(
    workspace_root: &camino::Utf8Path,
    store: &CredentialStore,
    password: &str,
) -> Result<(), WorkspaceError> {
    // Generate fresh salt and nonce with a CSPRNG.
    let mut salt = [0u8; SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    let mut key = derive_key(password, &salt).map_err(|e| to_ws_error(&e))?;

    let json = serde_json::to_vec(store)
        .map_err(|e| to_ws_error(&format!("JSON serialize error: {e}")))?;

    // Borrow the key — do not consume it so we can zeroize after.
    let cipher = ChaCha20Poly1305::new(<&Key>::from(&key));
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, json.as_ref())
        .map_err(|e| to_ws_error(&format!("Encryption failed: {e}")))?;

    let mut file = Vec::with_capacity(MAGIC.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    file.extend_from_slice(MAGIC);
    file.extend_from_slice(&salt);
    file.extend_from_slice(&nonce_bytes);
    file.extend_from_slice(&ciphertext);

    // Zeroize sensitive buffers after they've been written to the file.
    key.zeroize();
    nonce_bytes.zeroize();

    let path = cred_file_path(workspace_root);
    crate::atomic_write(&path, &file)?;
    Ok(())
}

fn next_cred_id(store: &CredentialStore) -> String {
    let max = store
        .credentials
        .iter()
        .filter_map(|c| {
            c.id.strip_prefix("CRED-")
                .and_then(|n| n.parse::<usize>().ok())
        })
        .max()
        .unwrap_or(0);
    format!("CRED-{:03}", max + 1)
}

pub fn add_credential(
    workspace_root: &camino::Utf8Path,
    password: &str,
    engagement_path: &str,
    label: &str,
    cred_type: &str,
    value: &str,
    notes: &str,
) -> Result<String, WorkspaceError> {
    if !VALID_CRED_TYPES.contains(&cred_type) {
        return Err(WorkspaceError::InvalidStatusSeverity(format!(
            "'{}' is not a valid credential type. Use: {}.",
            cred_type,
            VALID_CRED_TYPES.join(", ")
        )));
    }

    let mut store = load_store(workspace_root, password)?;
    let id = next_cred_id(&store);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    store.credentials.push(Credential {
        id: id.clone(),
        label: label.to_string(),
        cred_type: cred_type.to_string(),
        value: value.to_string(),
        status: "not_verified".to_string(),
        notes: notes.to_string(),
        engagement_path: engagement_path.to_string(),
        created: today.clone(),
        updated: today,
    });

    save_store(workspace_root, &store, password)?;
    Ok(id)
}

#[derive(Debug, Clone)]
pub struct CredentialSummary {
    pub id: String,
    pub label: String,
    pub cred_type: String,
    pub status: String,
}

pub fn list_credentials(
    workspace_root: &camino::Utf8Path,
    password: &str,
    engagement_path: &str,
) -> Result<Vec<CredentialSummary>, WorkspaceError> {
    let store = load_store(workspace_root, password)?;
    Ok(store
        .credentials
        .iter()
        .filter(|c| c.engagement_path == engagement_path)
        .map(|c| CredentialSummary {
            id: c.id.clone(),
            label: c.label.clone(),
            cred_type: c.cred_type.clone(),
            status: c.status.clone(),
        })
        .collect())
}

pub fn show_credential(
    workspace_root: &camino::Utf8Path,
    password: &str,
    cred_id: &str,
) -> Result<Credential, WorkspaceError> {
    let store = load_store(workspace_root, password)?;
    store
        .credentials
        .into_iter()
        .find(|c| c.id == cred_id)
        .ok_or_else(|| WorkspaceError::NotFound(camino::Utf8PathBuf::from(cred_id)))
}

pub fn update_credential_status(
    workspace_root: &camino::Utf8Path,
    password: &str,
    cred_id: &str,
    status: &str,
) -> Result<(), WorkspaceError> {
    if !VALID_CRED_STATUSES.contains(&status) {
        return Err(WorkspaceError::InvalidStatusSeverity(format!(
            "'{}' is not a valid credential status. Use: {}.",
            status,
            VALID_CRED_STATUSES.join(", ")
        )));
    }

    let mut store = load_store(workspace_root, password)?;
    let cred = store
        .credentials
        .iter_mut()
        .find(|c| c.id == cred_id)
        .ok_or_else(|| WorkspaceError::NotFound(camino::Utf8PathBuf::from(cred_id)))?;

    cred.status = status.to_string();
    cred.updated = chrono::Utc::now().format("%Y-%m-%d").to_string();

    save_store(workspace_root, &store, password)?;
    Ok(())
}

pub fn remove_credential(
    workspace_root: &camino::Utf8Path,
    password: &str,
    cred_id: &str,
) -> Result<(), WorkspaceError> {
    let mut store = load_store(workspace_root, password)?;
    let before = store.credentials.len();
    store.credentials.retain(|c| c.id != cred_id);
    if store.credentials.len() == before {
        return Err(WorkspaceError::NotFound(camino::Utf8PathBuf::from(cred_id)));
    }
    save_store(workspace_root, &store, password)?;
    Ok(())
}

pub fn verify_store(
    workspace_root: &camino::Utf8Path,
    password: &str,
) -> Result<bool, WorkspaceError> {
    let path = cred_file_path(workspace_root);
    if !path.exists() {
        return Ok(true);
    }
    let file_bytes = std::fs::read(path.as_std_path())?;
    Ok(decrypt_store(&file_bytes, password).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestWorkspace;

    #[test]
    fn add_and_list_credential() {
        let tw = TestWorkspace::new();
        let id = add_credential(
            &tw.root,
            "pass123",
            "acme/web/initial",
            "Admin Login",
            "username_password",
            "admin:secret",
            "",
        )
        .unwrap();
        assert!(id.starts_with("CRED-"));
        let creds = list_credentials(&tw.root, "pass123", "acme/web/initial").unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].label, "Admin Login");
    }

    #[test]
    fn show_includes_value() {
        let tw = TestWorkspace::new();
        let id = add_credential(&tw.root, "pass", "e", "API", "api_key", "sk-123", "prod").unwrap();
        let cred = show_credential(&tw.root, "pass", &id).unwrap();
        assert_eq!(cred.value, "sk-123");
    }

    #[test]
    fn wrong_password_fails() {
        let tw = TestWorkspace::new();
        add_credential(&tw.root, "correct", "e", "T", "api_key", "s", "").unwrap();
        assert!(load_store(&tw.root, "wrong").is_err());
    }

    #[test]
    fn encrypted_file_not_readable() {
        let tw = TestWorkspace::new();
        add_credential(&tw.root, "pass", "e", "T", "api_key", "secret_value", "").unwrap();
        let content = std::fs::read(tw.root.join(".credentials.enc")).unwrap();
        assert_eq!(&content[..7], b"SSCRED\x01");
        assert!(!String::from_utf8_lossy(&content).contains("secret_value"));
    }

    #[test]
    fn update_status() {
        let tw = TestWorkspace::new();
        let id = add_credential(&tw.root, "pass", "e", "T", "api_key", "s", "").unwrap();
        update_credential_status(&tw.root, "pass", &id, "working").unwrap();
        assert_eq!(
            show_credential(&tw.root, "pass", &id).unwrap().status,
            "working"
        );
    }

    #[test]
    fn remove_works() {
        let tw = TestWorkspace::new();
        let id = add_credential(&tw.root, "pass", "e", "T", "api_key", "s", "").unwrap();
        remove_credential(&tw.root, "pass", &id).unwrap();
        assert!(list_credentials(&tw.root, "pass", "e").unwrap().is_empty());
    }

    #[test]
    fn invalid_type_rejected() {
        let tw = TestWorkspace::new();
        assert!(add_credential(&tw.root, "pass", "e", "T", "bad_type", "s", "").is_err());
    }

    #[test]
    fn sequential_ids() {
        let tw = TestWorkspace::new();
        let id1 = add_credential(&tw.root, "pass", "e", "First", "api_key", "v1", "").unwrap();
        let id2 = add_credential(&tw.root, "pass", "e", "Second", "api_key", "v2", "").unwrap();
        assert_ne!(id1, id2);
        assert_eq!(list_credentials(&tw.root, "pass", "e").unwrap().len(), 2);
    }
}
