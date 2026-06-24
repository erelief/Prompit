use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaEntry {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub enabled: bool,
}

fn personas_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("personas.json"))
}

fn load_personas_encrypted(app: &AppHandle) -> Result<Vec<PersonaEntry>, String> {
    let path = personas_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    let bytes = crypto::decrypt("personas", &payload)?;

    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

fn save_personas_encrypted(app: &AppHandle, personas: &[PersonaEntry]) -> Result<(), String> {
    let path = personas_path(app)?;
    let json = serde_json::to_vec(personas).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt("personas", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
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
