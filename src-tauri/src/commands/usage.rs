use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

/// One recorded LLM request. Token fields are optional: some providers do not
/// return a `usage` object, in which case only the request itself is counted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub ts: u64,
    pub mode: String,
    pub provider: String,
    /// Stable provider identity ("name|base_url"): two provider configs with
    /// the same display name must still aggregate separately. Optional for
    /// records written before this field existed.
    #[serde(default)]
    pub provider_key: Option<String>,
    pub model: String,
    #[serde(default)]
    pub prompt: Option<u64>,
    #[serde(default)]
    pub completion: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
}

fn usage_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("usage.json"))
}

#[tauri::command]
pub fn read_usage(app: AppHandle) -> Result<Vec<UsageRecord>, String> {
    let path = usage_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    let bytes = crypto::decrypt("usage", &payload)?;

    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

#[tauri::command]
pub fn save_usage(app: AppHandle, records: Vec<UsageRecord>) -> Result<(), String> {
    let json = serde_json::to_vec(&records).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt("usage", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    let path = usage_path(&app)?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_record_roundtrip_with_and_without_tokens() {
        let records = vec![
            UsageRecord {
                ts: 1_700_000_000_000,
                mode: "translate".to_string(),
                provider: "DeepSeek".to_string(),
                provider_key: Some("DeepSeek|https://api.deepseek.com".to_string()),
                model: "deepseek-v4-flash".to_string(),
                prompt: Some(120),
                completion: Some(30),
                total: Some(150),
            },
            // Provider without usage reporting: only the request is counted.
            UsageRecord {
                ts: 1_700_000_100_000,
                mode: "skills_lite".to_string(),
                provider: "Local".to_string(),
                provider_key: None,
                model: "qwen2.5".to_string(),
                prompt: None,
                completion: None,
                total: None,
            },
        ];
        let json = serde_json::to_string(&records).unwrap();
        let back: Vec<UsageRecord> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].total, Some(150));
        assert_eq!(back[1].total, None);
        assert_eq!(back[1].mode, "skills_lite");
    }
}
