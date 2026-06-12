use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub ciphertext: String,
    pub nonce: String,
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

fn derive_key(scope: &str) -> [u8; 32] {
    let seed = format!("{}:{}", machine_seed(), scope);
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn derive_legacy_key() -> [u8; 32] {
    let seed = machine_seed();
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

pub fn encrypt(scope: &str, plaintext: &[u8]) -> Result<EncryptedPayload, String> {
    let key = derive_key(scope);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
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

pub fn decrypt(scope: &str, payload: &EncryptedPayload) -> Result<Vec<u8>, String> {
    let key = derive_key(scope);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce_bytes = BASE64
        .decode(&payload.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = BASE64
        .decode(&payload.ciphertext)
        .map_err(|e| format!("decode ct: {e}"))?;
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decrypt: {e}"))
}

pub fn decrypt_legacy(payload: &EncryptedPayload) -> Result<Vec<u8>, String> {
    let key = derive_legacy_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce_bytes = BASE64
        .decode(&payload.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
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

    #[test]
    fn test_scoped_key_differs_from_legacy() {
        let scoped = derive_key("history");
        let legacy = derive_legacy_key();
        assert_ne!(scoped, legacy, "scoped and legacy keys must differ");
    }

    #[test]
    fn test_different_scopes_produce_different_keys() {
        let a = derive_key("history");
        let b = derive_key("secrets");
        assert_ne!(a, b);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"hello world";
        let payload = encrypt("test", plaintext).unwrap();
        let decrypted = decrypt("test", &payload).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_scope_fails() {
        let plaintext = b"secret data";
        let payload = encrypt("secrets", plaintext).unwrap();
        assert!(decrypt("history", &payload).is_err());
    }

    #[test]
    fn test_legacy_decrypt_reads_old_format() {
        let key = derive_legacy_key();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce_bytes = [0u8; 12];
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, b"legacy data".as_slice()).unwrap();
        let payload = EncryptedPayload {
            ciphertext: BASE64.encode(&ciphertext),
            nonce: BASE64.encode(nonce_bytes),
        };
        let decrypted = decrypt_legacy(&payload).unwrap();
        assert_eq!(decrypted, b"legacy data");
    }

    #[test]
    fn test_derive_key_deterministic() {
        let a = derive_key("secrets");
        let b = derive_key("secrets");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }
}
