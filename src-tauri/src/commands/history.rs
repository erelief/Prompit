use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::commands::attachments::{attachments_dir, HistoryAttachment};
use crate::crypto::{self, EncryptedPayload};

/// Token usage reported by the provider for a single request. Absent when the
/// provider does not return a `usage` object (or for pre-feature entries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub prompt: Option<u64>,
    #[serde(default)]
    pub completion: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub input: String,
    pub output: String,
    pub timestamp: u64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub persona: Option<String>,
    #[serde(default, alias = "sparkle")]
    pub skills_lite: Option<String>,
    #[serde(default)]
    pub searched: bool,
    #[serde(default)]
    pub sources: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub edited: bool,
    /// Attachment metadata; payload files live under history_attachments/.
    #[serde(default)]
    pub attachments: Option<Vec<HistoryAttachment>>,
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
    let json = serde_json::to_vec(&trimmed).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt("history", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    let path = history_path(&app)?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    sweep_orphan_attachments(&app, &trimmed);
    Ok(())
}

/// Delete attachment dirs whose owning entry is no longer in the kept history
/// (pruned by limit, removed individually, or cleared). Best-effort: a failed
/// sweep must not fail the save.
fn sweep_orphan_attachments(app: &AppHandle, kept: &[&HistoryEntry]) {
    let dir = match attachments_dir(app) {
        Ok(d) => d,
        Err(_) => return,
    };
    if !dir.exists() {
        return;
    }
    let kept_ts: HashSet<String> = kept.iter().map(|e| e.timestamp.to_string()).collect();
    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !kept_ts.contains(&name) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    let path = history_path(&app)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("delete: {e}"))?;
    }
    let attachments = attachments_dir(&app)?;
    if attachments.exists() {
        fs::remove_dir_all(&attachments).map_err(|e| format!("delete attachments: {e}"))?;
    }
    Ok(())
}
