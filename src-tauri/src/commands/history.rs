use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub input: String,
    pub output: String,
    pub timestamp: u64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default)]
    pub sparkle: Option<String>,
    #[serde(default)]
    pub searched: bool,
    #[serde(default)]
    pub sources: Option<Vec<serde_json::Value>>,
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("history.json"))
}

#[tauri::command]
pub fn read_history(app: AppHandle) -> Result<Vec<HistoryEntry>, String> {
    let path = history_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    let bytes = crypto::decrypt("history", &payload)?;

    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

#[tauri::command]
pub fn save_history(
    app: AppHandle,
    entries: Vec<HistoryEntry>,
    limit: Option<usize>,
) -> Result<(), String> {
    let limit = limit.unwrap_or(50);
    let trimmed: Vec<&HistoryEntry> = entries.iter().take(limit).collect();
    let trimmed: Vec<HistoryEntry> = trimmed.into_iter().cloned().collect();
    let json = serde_json::to_vec(&trimmed).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt("history", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    let path = history_path(&app)?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    let path = history_path(&app)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("delete: {e}"))?;
    }
    Ok(())
}
