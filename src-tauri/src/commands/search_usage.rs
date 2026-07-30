use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::crypto::{self, EncryptedPayload};

/// One recorded web search request. Searches are counted by request only —
/// there is no token semantic. `provider_key` ("preset|custom_name") keeps two
/// same-named search providers aggregating separately; optional for records
/// written before this field existed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchUsageRecord {
    pub ts: u64,
    pub provider: String,
    #[serde(default)]
    pub provider_key: Option<String>,
}

fn search_usage_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("search_usage.json"))
}

#[tauri::command]
pub fn read_search_usage(app: AppHandle) -> Result<Vec<SearchUsageRecord>, String> {
    let path = search_usage_path(&app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    let bytes = crypto::decrypt("search_usage", &payload)?;

    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

#[tauri::command]
pub fn save_search_usage(app: AppHandle, records: Vec<SearchUsageRecord>) -> Result<(), String> {
    let json = serde_json::to_vec(&records).map_err(|e| format!("serialize: {e}"))?;

    let payload = crypto::encrypt("search_usage", &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    let path = search_usage_path(&app)?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_usage_record_roundtrip() {
        let records = vec![
            SearchUsageRecord {
                ts: 1_700_000_000_000,
                provider: "Tavily".to_string(),
                provider_key: Some("tavily|Tavily".to_string()),
            },
            SearchUsageRecord {
                ts: 1_700_000_100_000,
                provider: "Brave".to_string(),
                provider_key: None,
            },
        ];
        let json = serde_json::to_string(&records).unwrap();
        let back: Vec<SearchUsageRecord> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].provider, "Tavily");
        assert_eq!(
            back[0].provider_key.as_deref(),
            Some("tavily|Tavily")
        );
        assert!(back[1].provider_key.is_none());
    }
}
