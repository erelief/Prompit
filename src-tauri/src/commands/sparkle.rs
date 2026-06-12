use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkleEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct EncryptedSparkles {
    ciphertext: String,
    nonce: String,
}

fn sparkle_key() -> [u8; 32] {
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

fn sparkles_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("sparkles.json"))
}

fn load_sparkles_encrypted(app: &AppHandle) -> Result<Vec<SparkleEntry>, String> {
    let path = sparkles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let enc: EncryptedSparkles = serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let key = sparkle_key();
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

fn save_sparkles_encrypted(app: &AppHandle, sparkles: &[SparkleEntry]) -> Result<(), String> {
    let path = sparkles_path(app)?;
    let json = serde_json::to_vec(sparkles).map_err(|e| format!("serialize: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use rand::RngCore;

    let key = sparkle_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, json.as_slice())
        .map_err(|e| format!("encrypt: {e}"))?;

    let enc = EncryptedSparkles {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    };
    let out = serde_json::to_string_pretty(&enc).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn read_sparkles(app: AppHandle) -> Result<Vec<SparkleEntry>, String> {
    load_sparkles_encrypted(&app)
}

#[tauri::command]
pub fn save_sparkles(app: AppHandle, sparkles: Vec<SparkleEntry>) -> Result<(), String> {
    save_sparkles_encrypted(&app, &sparkles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkle_entry_serialize_roundtrip() {
        let entries = vec![
            SparkleEntry {
                name: "Formal".to_string(),
                prompt: "Translate formally".to_string(),
                enabled: true,
            },
            ];
        let json = serde_json::to_string(&entries).unwrap();
        let deserialized: Vec<SparkleEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Formal");
        assert!(deserialized[0].enabled);
    }
}
