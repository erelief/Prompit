use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct EncryptedPersonas {
    ciphertext: String,
    nonce: String,
}

fn derive_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".into());
    let app_id = "com.translator.realtime";
    let seed = format!("{}:{}:{}", hostname, username, app_id);
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn personas_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("personas.json"))
}

fn load_personas_encrypted(app: &AppHandle) -> Result<Vec<PersonaEntry>, String> {
    let path = personas_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let enc: EncryptedPersonas = serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce_bytes = BASE64
        .decode(&enc.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = BASE64
        .decode(&enc.ciphertext)
        .map_err(|e| format!("decode ct: {e}"))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decrypt: {e}"))?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("deserialize: {e}"))
}

fn save_personas_encrypted(app: &AppHandle, personas: &[PersonaEntry]) -> Result<(), String> {
    let path = personas_path(app)?;
    let json = serde_json::to_vec(personas).map_err(|e| format!("serialize: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use rand::RngCore;

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, json.as_slice())
        .map_err(|e| format!("encrypt: {e}"))?;

    let enc = EncryptedPersonas {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    };
    let out = serde_json::to_string_pretty(&enc).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn read_personas(app: AppHandle) -> Result<Vec<PersonaEntry>, String> {
    load_personas_encrypted(&app)
}

#[tauri::command]
pub fn save_personas(app: AppHandle, personas: Vec<PersonaEntry>) -> Result<(), String> {
    save_personas_encrypted(&app, &personas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_entry_serialize_roundtrip() {
        let entries = vec![
            PersonaEntry {
                name: "Formal".to_string(),
                prompt: "Translate formally".to_string(),
                enabled: true,
            },
            PersonaEntry {
                name: "Casual".to_string(),
                prompt: "Translate casually".to_string(),
                enabled: false,
            },
        ];
        let json = serde_json::to_string(&entries).unwrap();
        let deserialized: Vec<PersonaEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].name, "Formal");
        assert!(deserialized[0].enabled);
        assert!(!deserialized[1].enabled);
    }
}
