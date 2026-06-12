use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

type SecretStore = HashMap<String, EncryptedPayload>;

fn secrets_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
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
    let payload = crypto::encrypt("secrets", plaintext.as_bytes())?;
    let mut store = load_store(&app)?;
    store.insert(key_id, payload);
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

    let bytes = crypto::decrypt("secrets", entry)
        .or_else(|_| {
            // Legacy migration: try old key (no scope)
            let plaintext = crypto::decrypt_legacy(entry)?;
            // Re-encrypt with scoped key
            let new_payload = crypto::encrypt("secrets", &plaintext)?;
            let mut store = load_store(&app)?;
            store.insert(key_id, new_payload);
            save_store(&app, &store)?;
            Ok::<Vec<u8>, String>(plaintext)
        })?;

    String::from_utf8(bytes).map_err(|e| format!("utf8: {e}"))
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
    fn test_secrets_store_type_compiles() {
        let mut store: SecretStore = HashMap::new();
        let payload = crate::crypto::encrypt("secrets", b"test-key").unwrap();
        store.insert("provider_0".to_string(), payload);
        assert!(store.contains_key("provider_0"));
    }
}
