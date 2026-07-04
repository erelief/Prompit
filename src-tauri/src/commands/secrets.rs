use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

/// In-memory store: provider-id → plaintext API key.
type SecretStore = HashMap<String, String>;

fn secrets_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("secrets.json"))
}

/// Load the secret store, transparently migrating the legacy on-disk format.
///
/// On-disk format history:
/// - **Legacy**: `HashMap<String, EncryptedPayload>` — each value encrypted
///   independently, keys (e.g. `provider_0`) stored in plaintext. This leaked
///   the count and naming scheme of configured providers.
/// - **Current**: a single `EncryptedPayload` wrapping the serialized
///   `HashMap<String, String>`. Matches the other 4 data files (history,
///   personas, dictionaries, sparkles) and hides the key names.
///
/// Migration is one-way and lazy: if the file parses as the legacy map, every
/// value is decrypted, the store is re-encrypted as a single payload, and the
/// file is rewritten. Subsequent loads hit the new path directly.
fn load_store(app: &AppHandle) -> Result<SecretStore, String> {
    let path = secrets_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;

    // Current format: single EncryptedPayload wrapping HashMap<String, String>.
    if let Ok(payload) = serde_json::from_str::<EncryptedPayload>(&content) {
        let bytes = crypto::decrypt("secrets", &payload)?;
        return serde_json::from_slice::<SecretStore>(&bytes)
            .map_err(|e| format!("deserialize secrets: {e}"));
    }

    // Legacy format: HashMap<String, EncryptedPayload>. Decrypt each value,
    // rebuild a plaintext-keyed store, then persist in the new single-payload
    // format so this branch never runs again for this file.
    let legacy: HashMap<String, EncryptedPayload> =
        serde_json::from_str(&content).map_err(|e| format!("parse secrets: {e}"))?;
    let mut store: SecretStore = HashMap::new();
    for (k, payload) in legacy {
        let bytes = crypto::decrypt("secrets", &payload)?;
        let plaintext = String::from_utf8(bytes).map_err(|e| format!("utf8: {e}"))?;
        store.insert(k, plaintext);
    }
    // Best-effort rewrite; a failure here is non-fatal (the store is already
    // correct in memory) — the next successful save persists the new format.
    let _ = save_store(app, &store);
    Ok(store)
}

fn save_store(app: &AppHandle, store: &SecretStore) -> Result<(), String> {
    let path = secrets_path(app)?;
    let json = serde_json::to_vec(store).map_err(|e| format!("serialize: {e}"))?;
    let payload = crypto::encrypt("secrets", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn save_secret(app: AppHandle, key_id: String, plaintext: String) -> Result<(), String> {
    let mut store = load_store(&app)?;
    store.insert(key_id, plaintext);
    save_store(&app, &store)
}

#[tauri::command]
pub fn read_secret(app: AppHandle, key_id: String) -> Result<String, String> {
    // Env var override for development isolation. The frontend stores provider
    // keys under the `provider_<i>` id (see secretKeyId in config.ts), so the
    // override must match that prefix — not the legacy `model_` one. Restrict
    // the index to ASCII digits so a malformed key_id can't construct an
    // arbitrary environment variable name.
    if let Some(index_str) = key_id.strip_prefix("provider_") {
        if index_str.chars().all(|c| c.is_ascii_digit()) {
            let env_name = format!("PROMPIT_API_KEY_{}", index_str);
            if let Ok(val) = std::env::var(&env_name) {
                if !val.is_empty() {
                    return Ok(val);
                }
            }
        }
    }

    let store = load_store(&app)?;
    Ok(store.get(&key_id).cloned().unwrap_or_default())
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
    fn test_secrets_store_roundtrip_via_single_payload() {
        crate::crypto::set_master_key([0x42u8; 32]);
        let mut store: SecretStore = HashMap::new();
        store.insert("provider_0".to_string(), "sk-test-123".to_string());
        store.insert("provider_1".to_string(), "sk-other".to_string());

        let json = serde_json::to_vec(&store).unwrap();
        let payload = crate::crypto::encrypt("secrets", &json).unwrap();
        let out = serde_json::to_string_pretty(&payload).unwrap();

        // Re-read as the new format.
        let parsed_payload: EncryptedPayload = serde_json::from_str(&out).unwrap();
        let bytes = crate::crypto::decrypt("secrets", &parsed_payload).unwrap();
        let back: SecretStore = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.get("provider_0"), Some(&"sk-test-123".to_string()));
        assert_eq!(back.get("provider_1"), Some(&"sk-other".to_string()));
    }

    #[test]
    fn test_legacy_format_is_migratable() {
        crate::crypto::set_master_key([0x42u8; 32]);
        // Build the legacy on-disk shape: HashMap<String, EncryptedPayload>.
        let mut legacy: HashMap<String, EncryptedPayload> = HashMap::new();
        legacy.insert(
            "provider_0".to_string(),
            crate::crypto::encrypt("secrets", b"sk-legacy").unwrap(),
        );
        let legacy_json = serde_json::to_string_pretty(&legacy).unwrap();

        // Simulate load_store's migration path: fails new-format parse, falls
        // back to legacy map, decrypts each value.
        let parsed_legacy: HashMap<String, EncryptedPayload> =
            serde_json::from_str(&legacy_json).unwrap();
        let mut migrated: SecretStore = HashMap::new();
        for (k, payload) in parsed_legacy {
            let bytes = crate::crypto::decrypt("secrets", &payload).unwrap();
            migrated.insert(k, String::from_utf8(bytes).unwrap());
        }
        assert_eq!(migrated.get("provider_0"), Some(&"sk-legacy".to_string()));

        // And the migrated store re-encrypts to the new single-payload shape.
        let json = serde_json::to_vec(&migrated).unwrap();
        let new_payload = crate::crypto::encrypt("secrets", &json).unwrap();
        let new_out = serde_json::to_string_pretty(&new_payload).unwrap();
        // The re-serialized file must parse as a single EncryptedPayload (not a map).
        let reparsed: EncryptedPayload = serde_json::from_str(&new_out).unwrap();
        let bytes = crate::crypto::decrypt("secrets", &reparsed).unwrap();
        let back: SecretStore = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.get("provider_0"), Some(&"sk-legacy".to_string()));
    }
}
