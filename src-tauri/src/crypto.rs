use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use zeroize::Zeroizing;

#[derive(Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub ciphertext: String,
    pub nonce: String,
}

/// In-memory Master Key. Set once at startup by the vault module (after
/// unwrapping it from `vault.key` via the local KEK — see `kek.rs`). When
/// present, all per-scope data keys are derived from it (portable, random).
/// When absent (unit tests that never call `set_master_key`), `derive_key`
/// panics rather than silently deriving wrong keys.
static MASTER_KEY: OnceLock<Zeroizing<[u8; 32]>> = OnceLock::new();

/// Install the Master Key into the process. Called exactly once from the vault
/// module during app setup. Subsequent calls are no-ops (the first wins), which
/// keeps tests that re-initialize from corrupting a running process.
pub fn set_master_key(key: [u8; 32]) {
    let _ = MASTER_KEY.set(Zeroizing::new(key));
}

/// Copy out the installed Master Key, or error if none is set (should not
/// happen at runtime — setup always unlocks the vault first). Used by the
/// vault's export path to wrap the key under a password.
pub fn get_master_key() -> Result<[u8; 32], String> {
    MASTER_KEY
        .get()
        .map(|mk| **mk)
        .ok_or_else(|| "master key not initialized".to_string())
}

fn machine_seed() -> String {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".into());
    let app_id = "com.translator.realtime";
    format!("{}:{}:{}", hostname, username, app_id)
}

/// Raw machine-seed key, NOT routed through the Master Key. The vault uses this
/// to wrap/unwrap the Master Key itself (otherwise we'd recurse: deriving the
/// master-key-protecting key from the master key).
pub fn derive_raw_machine_key() -> [u8; 32] {
    let seed = machine_seed();
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Per-scope data key derived from the installed Master Key (domain-separated
/// by `scope`). Panics if the Master Key is not installed — at runtime setup
/// always installs it before any data is read, so this never fires in prod.
fn derive_key(scope: &str) -> [u8; 32] {
    // Domain-separated derivation: distinct 32-byte key per scope, all rooted
    // at the random Master Key. SHA256 is a fine KDF here because the input is
    // already a full-entropy 256-bit key, not a password. The Master Key is
    // always installed by the time any data is read (vault::unlock_or_migrate
    // runs in setup, before commands are served), so there is no fallback.
    let mk = MASTER_KEY
        .get()
        .expect("derive_key called before Master Key was installed");
    let mut hasher = Sha256::new();
    hasher.update(mk.as_slice());
    hasher.update(b":");
    hasher.update(scope.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

pub fn encrypt(scope: &str, plaintext: &[u8]) -> Result<EncryptedPayload, String> {
    encrypt_with_key(&derive_key(scope), plaintext)
}

pub fn decrypt(scope: &str, payload: &EncryptedPayload) -> Result<Vec<u8>, String> {
    decrypt_with_key(&derive_key(scope), payload)
}

/// Encrypt with an explicit raw key (bypasses scope derivation). Used by the
/// vault to wrap the Master Key itself, where we must NOT route through
/// derive_key (that would recurse).
pub fn encrypt_with_key(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedPayload, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("encrypt: {e}"))?;
    Ok(EncryptedPayload {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    })
}

/// Decrypt with an explicit raw key. Mirror of encrypt_with_key.
pub fn decrypt_with_key(key: &[u8; 32], payload: &EncryptedPayload) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce_bytes = BASE64
        .decode(&payload.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
    if nonce_bytes.len() != 12 {
        return Err("invalid nonce length".into());
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = BASE64
        .decode(&payload.ciphertext)
        .map_err(|e| format!("decode ct: {e}"))?;
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decrypt: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Install a fixed Master Key for the test process. OnceLock is first-write-
    /// wins, so this runs once; subsequent calls are no-ops. All encrypt/decrypt
    /// tests below share this key, which is fine — they assert self-consistency,
    /// not key independence.
    fn ensure_test_master_key() {
        set_master_key([0x42u8; 32]);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        ensure_test_master_key();
        let plaintext = b"hello world";
        let payload = encrypt("test", plaintext).unwrap();
        let decrypted = decrypt("test", &payload).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_scope_fails() {
        ensure_test_master_key();
        let plaintext = b"secret data";
        let payload = encrypt("secrets", plaintext).unwrap();
        assert!(decrypt("history", &payload).is_err());
    }

    #[test]
    fn test_different_scopes_produce_different_keys() {
        ensure_test_master_key();
        // Round-trip under two scopes; wrong-scope decrypt fails (asserted
        // above), which implies the per-scope keys differ.
        let a = encrypt("history", b"x").unwrap();
        let b = encrypt("secrets", b"x").unwrap();
        assert!(decrypt("secrets", &a).is_err());
        assert!(decrypt("history", &b).is_err());
    }

    #[test]
    fn test_raw_machine_key_stable_and_32() {
        let a = derive_raw_machine_key();
        let b = derive_raw_machine_key();
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn test_encrypt_with_key_roundtrip() {
        let key = [7u8; 32];
        let payload = encrypt_with_key(&key, b"data").unwrap();
        let out = decrypt_with_key(&key, &payload).unwrap();
        assert_eq!(out, b"data");
    }

    #[test]
    fn test_encrypt_with_key_wrong_key_fails() {
        let payload = encrypt_with_key(&[1u8; 32], b"data").unwrap();
        assert!(decrypt_with_key(&[2u8; 32], &payload).is_err());
    }
}
