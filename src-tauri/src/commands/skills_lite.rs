use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

/// Crypto scope for skills-lite data. Feeds AES-GCM key derivation.
const SCOPE: &str = "skills_lite";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsLiteEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled: bool,
}

fn skills_lites_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("skills_lite.json"))
}

fn load_skills_lites_encrypted(app: &AppHandle) -> Result<Vec<SkillsLiteEntry>, String> {
    let path = skills_lites_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;
    let bytes = crypto::decrypt(SCOPE, &payload)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

fn save_skills_lites_encrypted(app: &AppHandle, skills_lites: &[SkillsLiteEntry]) -> Result<(), String> {
    let path = skills_lites_path(app)?;
    let json = serde_json::to_vec(skills_lites).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt(SCOPE, &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn read_skills_lites(app: AppHandle) -> Result<Vec<SkillsLiteEntry>, String> {
    load_skills_lites_encrypted(&app)
}

#[tauri::command]
pub fn save_skills_lites(app: AppHandle, skills_lites: Vec<SkillsLiteEntry>) -> Result<(), String> {
    save_skills_lites_encrypted(&app, &skills_lites)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_lite_entry_serialize_roundtrip() {
        let entries = vec![SkillsLiteEntry {
            name: "Formal".to_string(),
            prompt: "Translate formally".to_string(),
            description: "Rewrite input formally".to_string(),
            enabled: true,
        }];
        let json = serde_json::to_string(&entries).unwrap();
        let deserialized: Vec<SkillsLiteEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Formal");
        assert_eq!(deserialized[0].description, "Rewrite input formally");
        assert!(deserialized[0].enabled);
    }

    #[test]
    fn test_legacy_skills_lite_without_description_defaults_empty() {
        // Older files persisted before the `description` field existed.
        let json = r#"[{"name":"Polish","prompt":"Rewrite nicely","enabled":true}]"#;
        let deserialized: Vec<SkillsLiteEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].name, "Polish");
        assert_eq!(deserialized[0].description, "");
        assert!(deserialized[0].enabled);
    }
}
