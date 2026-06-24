use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkleEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
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
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    let bytes =
        crypto::decrypt("sparkles", &payload).or_else(|_: String| -> Result<Vec<u8>, String> {
            let plaintext = crypto::decrypt_legacy(&payload)?;
            let new_payload = crypto::encrypt("sparkles", &plaintext)?;
            let out = serde_json::to_string_pretty(&new_payload).map_err(|e| format!("{e}"))?;
            fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
            Ok(plaintext)
        })?;

    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

fn save_sparkles_encrypted(app: &AppHandle, sparkles: &[SparkleEntry]) -> Result<(), String> {
    let path = sparkles_path(app)?;
    let json = serde_json::to_vec(sparkles).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt("sparkles", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
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
        let entries = vec![SparkleEntry {
            name: "Formal".to_string(),
            prompt: "Translate formally".to_string(),
            description: "Rewrite input formally".to_string(),
            enabled: true,
        }];
        let json = serde_json::to_string(&entries).unwrap();
        let deserialized: Vec<SparkleEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Formal");
        assert_eq!(deserialized[0].description, "Rewrite input formally");
        assert!(deserialized[0].enabled);
    }

    #[test]
    fn test_legacy_sparkle_without_description_defaults_empty() {
        // Older files persisted before the `description` field existed.
        let json = r#"[{"name":"Polish","prompt":"Rewrite nicely","enabled":true}]"#;
        let deserialized: Vec<SparkleEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Polish");
        assert_eq!(deserialized[0].description, "");
        assert!(deserialized[0].enabled);
    }
}
