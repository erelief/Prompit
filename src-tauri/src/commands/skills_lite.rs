use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

/// Crypto scope for skills-lite data. Was `"sparkles"` historically; renamed
/// alongside the on-disk file. The scope feeds AES-GCM key derivation, so a
/// rename requires re-encrypting existing data (see `migrate_legacy_file`).
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

fn legacy_sparkles_path(dir: &PathBuf) -> PathBuf {
    dir.join("sparkles.json")
}

/// One-way migration of the legacy on-disk format. Historically this data
/// lived in `sparkles.json` encrypted under scope `"sparkles"`. Both the
/// filename and the crypto scope have been renamed to `skills_lite`, so the
/// file must be re-encrypted (the scope participates in AES-GCM key derivation)
/// and moved. This runs once at startup; afterwards the new file is the only
/// one touched. Best-effort: any failure is logged and swallowed, leaving the
/// legacy file in place (the app still reads correctly via the fallback in
/// `load_skills_lites_encrypted`).
pub fn migrate_legacy_file(app: &AppHandle) {
    let dir = match crate::get_data_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    let legacy = legacy_sparkles_path(&dir);
    let current = dir.join("skills_lite.json");
    if !legacy.exists() || current.exists() {
        // Nothing to migrate, or already migrated (current wins).
        if current.exists() && legacy.exists() {
            let _ = fs::remove_file(&legacy);
        }
        return;
    }

    let result = (|| -> Result<(), String> {
        let content = fs::read_to_string(&legacy).map_err(|e| format!("read legacy: {e}"))?;
        let payload: EncryptedPayload =
            serde_json::from_str(&content).map_err(|e| format!("parse legacy: {e}"))?;
        // Decrypt with the OLD scope, re-encrypt with the new one.
        let bytes = crypto::decrypt("sparkles", &payload)?;
        let new_payload = crypto::encrypt(SCOPE, &bytes)?;
        let out =
            serde_json::to_string_pretty(&new_payload).map_err(|e| format!("serialize: {e}"))?;
        fs::write(&current, out).map_err(|e| format!("write: {e}"))?;
        fs::remove_file(&legacy).map_err(|e| format!("remove legacy: {e}"))?;
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("skills_lite migration skipped: {e}");
    }
}

fn load_skills_lites_encrypted(app: &AppHandle) -> Result<Vec<SkillsLiteEntry>, String> {
    let path = skills_lites_path(app)?;
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
        let payload: EncryptedPayload =
            serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;
        let bytes = crypto::decrypt(SCOPE, &payload)?;
        return serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"));
    }

    // Fallback: legacy sparkles.json encrypted under the old scope. Reached
    // only if the startup migration didn't run yet (e.g. migration failed).
    let dir = crate::get_data_dir(app)?;
    let legacy = legacy_sparkles_path(&dir);
    if !legacy.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&legacy).map_err(|e| format!("read legacy: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse legacy: {e}"))?;
    let bytes = crypto::decrypt("sparkles", &payload)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize legacy: {e}"))
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

    #[test]
    fn test_legacy_scope_decrypts_new_scope_does_not() {
        crate::crypto::set_master_key([0x42u8; 32]);
        let payload = crate::crypto::encrypt("sparkles", b"skills data").unwrap();
        // Legacy scope must decrypt.
        let bytes = crate::crypto::decrypt("sparkles", &payload).unwrap();
        assert_eq!(bytes, b"skills data");
        // New scope must NOT decrypt the legacy payload — proves the scopes
        // derive different keys, justifying the re-encrypt migration.
        assert!(crate::crypto::decrypt(SCOPE, &payload).is_err());
    }
}
