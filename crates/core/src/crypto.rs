use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::{Algorithm, Argon2, Params, Version};
use bip39::Mnemonic;
use serde::{Deserialize, Serialize};

const MEMORY_KIB: u32 = 65536; // 64 MB
const ITERATIONS: u32 = 3;
const PARALLELISM: u32 = 4;
const HASH_LENGTH: usize = 32;

/// Derive a 32-byte key from a password and 16-byte salt using Argon2id v1.3.
pub fn derive_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 32], String> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, PARALLELISM, Some(HASH_LENGTH))
        .map_err(|e| format!("Argon2id params error: {e}"))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut output = [0u8; HASH_LENGTH];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|e| format!("Argon2id derivation failed: {e}"))?;

    Ok(output)
}

/// Async wrapper that runs Argon2id on a blocking thread pool.
pub async fn derive_key_async(password: String, salt: [u8; 16]) -> Result<[u8; 32], String> {
    tokio::task::spawn_blocking(move || derive_key(&password, &salt))
        .await
        .map_err(|e| format!("Key derivation task failed: {e}"))?
}

/// Generate a random 16-byte salt.
pub fn generate_salt() -> [u8; 16] {
    rand::random()
}

/// Generate a random 12-byte nonce for AES-256-GCM.
pub fn generate_nonce() -> [u8; 12] {
    rand::random()
}

/// Generate a 12-word BIP-39 recovery phrase from 128 bits of entropy.
pub fn generate_recovery_phrase() -> String {
    let entropy: [u8; 16] = rand::random();
    let mnemonic = Mnemonic::from_entropy(&entropy).expect("valid entropy for 12 words");
    mnemonic.to_string()
}

/// Derive a recovery key from a BIP-39 phrase and a salt using Argon2id.
pub fn derive_recovery_key(phrase: &str, salt: &[u8; 16]) -> Result<[u8; 32], String> {
    derive_key(phrase, salt)
}

/// Async wrapper that runs Argon2id recovery key derivation on a blocking thread pool.
pub async fn derive_recovery_key_async(phrase: String, salt: [u8; 16]) -> Result<[u8; 32], String> {
    tokio::task::spawn_blocking(move || derive_key(&phrase, &salt))
        .await
        .map_err(|e| format!("Recovery key derivation task failed: {e}"))?
}

/// Envelope plaintext containing the vault's encryption key.
#[derive(Serialize, Deserialize, Debug)]
pub struct EnvelopePayload {
    pub vault_key_hex: String,
    pub version: u32,
}

/// Parse a BIP-39 phrase into individual words (space-separated).
pub fn parse_phrase(phrase: &str) -> Vec<String> {
    phrase
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect()
}

/// Validate that a phrase contains exactly 12 valid BIP-39 English words.
pub fn is_valid_recovery_phrase(phrase: &str) -> bool {
    let words = parse_phrase(phrase);
    if words.len() != 12 {
        return false;
    }
    if let Ok(mnemonic) = Mnemonic::parse_in(bip39::Language::English, phrase.trim()) {
        mnemonic.words().count() == 12
    } else {
        false
    }
}

/// Encrypt the vault key into a recovery envelope using the recovery key.
/// Returns `(nonce, ciphertext)` where `ciphertext` includes the GCM tag.
pub fn encrypt_envelope(
    vault_key: &[u8; 32],
    recovery_key: &[u8; 32],
) -> Result<([u8; 12], Vec<u8>), String> {
    let payload = EnvelopePayload {
        vault_key_hex: hex::encode(vault_key),
        version: 1,
    };
    let plaintext = serde_json::to_vec(&payload)
        .map_err(|e| format!("Envelope JSON serialization failed: {e}"))?;

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(recovery_key);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes: [u8; 12] = generate_nonce();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("Envelope encryption failed: {:?}", e))?;

    Ok((nonce_bytes, ciphertext))
}

/// Decrypt a recovery envelope with the recovery key.
/// `blob` must be `nonce (12 bytes) || ciphertext (includes GCM tag)`.
pub fn decrypt_envelope(blob: &[u8], recovery_key: &[u8; 32]) -> Result<[u8; 32], String> {
    if blob.len() < 12 + 16 {
        return Err("Envelope too short (must contain nonce + ciphertext + tag)".to_string());
    }
    let (nonce_bytes, ciphertext) = blob.split_at(12);

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(recovery_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Envelope decryption failed — wrong recovery phrase?".to_string())?;

    let payload: EnvelopePayload = serde_json::from_slice(&plaintext)
        .map_err(|e| format!("Envelope JSON parse failed: {e}"))?;

    if payload.version != 1 {
        return Err(format!(
            "Unsupported recovery envelope version {}",
            payload.version
        ));
    }

    let vault_key = hex::decode(&payload.vault_key_hex)
        .map_err(|e| format!("Invalid vault key hex in envelope: {e}"))?;
    if vault_key.len() != 32 {
        return Err("Invalid vault key length in envelope".to_string());
    }
    let mut result = [0u8; 32];
    result.copy_from_slice(&vault_key);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation_deterministic() {
        let salt = [1u8; 16];
        let key1 = derive_key("test_password", &salt).unwrap();
        let key2 = derive_key("test_password", &salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_derivation_different_passwords() {
        let salt = [1u8; 16];
        let key1 = derive_key("password_one", &salt).unwrap();
        let key2 = derive_key("password_two", &salt).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_derivation_different_salts() {
        let key1 = derive_key("test_password", &[1u8; 16]).unwrap();
        let key2 = derive_key("test_password", &[2u8; 16]).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_salt_uniqueness() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_salt_length() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16);
    }

    #[test]
    fn test_recovery_phrase_is_12_words() {
        let phrase = generate_recovery_phrase();
        let words: Vec<&str> = phrase.split_ascii_whitespace().collect();
        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_recovery_phrase_different_each_time() {
        let p1 = generate_recovery_phrase();
        let p2 = generate_recovery_phrase();
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_valid_phrase_check() {
        let phrase = generate_recovery_phrase();
        assert!(is_valid_recovery_phrase(&phrase));
    }

    #[test]
    fn test_invalid_phrase_check() {
        assert!(!is_valid_recovery_phrase("hello world"));
        assert!(!is_valid_recovery_phrase(""));
    }

    #[test]
    fn test_envelope_roundtrip() {
        let vault_key = [42u8; 32];
        let recovery_key = [99u8; 32];
        let (nonce, ciphertext) = encrypt_envelope(&vault_key, &recovery_key).unwrap();
        let mut blob = nonce.to_vec();
        blob.extend_from_slice(&ciphertext);
        let decrypted = decrypt_envelope(&blob, &recovery_key).unwrap();
        assert_eq!(decrypted, vault_key);
    }

    #[test]
    fn test_envelope_wrong_key_fails() {
        let vault_key = [42u8; 32];
        let recovery_key = [99u8; 32];
        let wrong_key = [77u8; 32];
        let (nonce, ciphertext) = encrypt_envelope(&vault_key, &recovery_key).unwrap();
        let mut blob = nonce.to_vec();
        blob.extend_from_slice(&ciphertext);
        let result = decrypt_envelope(&blob, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_envelope_wrong_nonce_slice_position() {
        let vault_key = [42u8; 32];
        let recovery_key = [99u8; 32];
        let (nonce, ciphertext) = encrypt_envelope(&vault_key, &recovery_key).unwrap();
        // Corrupt the nonce
        let mut blob = nonce.to_vec();
        blob[0] ^= 0xff;
        blob.extend_from_slice(&ciphertext);
        let result = decrypt_envelope(&blob, &recovery_key);
        assert!(result.is_err());
    }
}
