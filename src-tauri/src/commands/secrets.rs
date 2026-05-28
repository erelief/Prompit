use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
struct SecretEntry {
    ciphertext: String,
    nonce: String,
}

type SecretStore = HashMap<String, SecretEntry>;

fn get_machine_seed() -> String {
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".into());
    let app_id = "com.translator.realtime";
    format!("{}:{}:{}", hostname, username, app_id)
}

fn derive_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let seed = get_machine_seed();
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn secrets_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("secrets.json"))
}

fn load_store(app: &AppHandle) -> Result<SecretStore, String> {
    let path = secrets_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))
}

fn save_store(app: &AppHandle, store: &SecretStore) -> Result<(), String> {
    let path = secrets_path(app)?;
    let json = serde_json::to_string_pretty(store).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn save_secret(app: AppHandle, key_id: String, plaintext: String) -> Result<(), String> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use rand::RngCore;

    let key = derive_key();
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encrypt: {e}"))?;

    let entry = SecretEntry {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    };

    let mut store = load_store(&app)?;
    store.insert(key_id, entry);
    save_store(&app, &store)
}

#[tauri::command]
pub fn read_secret(app: AppHandle, key_id: String) -> Result<String, String> {
    // Env var override for development isolation
    if let Some(index_str) = key_id.strip_prefix("model_") {
        let env_name = format!("PROMPIT_API_KEY_{}", index_str);
        if let Ok(val) = std::env::var(&env_name) {
            if !val.is_empty() {
                return Ok(val);
            }
        }
    }

    let store = load_store(&app)?;
    let entry = match store.get(&key_id) {
        Some(e) => e,
        None => return Ok(String::new()),
    };

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let key = derive_key();
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;

    let nonce_bytes = BASE64
        .decode(&entry.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = BASE64
        .decode(&entry.ciphertext)
        .map_err(|e| format!("decode ct: {e}"))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decrypt: {e}"))?;

    String::from_utf8(plaintext).map_err(|e| format!("utf8: {e}"))
}

#[tauri::command]
pub fn delete_secret(app: AppHandle, key_id: String) -> Result<(), String> {
    let mut store = load_store(&app)?;
    store.remove(&key_id);
    save_store(&app, &store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_deterministic() {
        let k1 = derive_key();
        let k2 = derive_key();
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        use aes_gcm::aead::Aead;
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

        let key = derive_key();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = Nonce::from_slice(&[0u8; 12]);

        let plaintext = "sk-test-not-real-key-for-unit-tests-only";
        let ct = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
        let pt = cipher.decrypt(nonce, ct.as_ref()).unwrap();
        assert_eq!(String::from_utf8(pt).unwrap(), plaintext);
    }
}
