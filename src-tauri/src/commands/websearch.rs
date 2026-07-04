use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::config::WebEngineConfig;
use crate::crypto::{self, EncryptedPayload};

/// Crypto scope for the web-search-engine bundle. The engine `api_key` fields
/// are encrypted at rest here, so they travel with the bundle and never land
/// in plaintext `config.json`.
const SCOPE: &str = "websearch";

/// The encrypted, on-disk web-search-engine bundle. Self-contained: the engine
/// array plus the single active index that references it. The API keys live
/// inline on each `WebEngineConfig.api_key` and are encrypted together with the
/// rest of this struct (no separate secrets store).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchBundle {
    #[serde(default)]
    pub web_engines: Vec<WebEngineConfig>,
    #[serde(default = "default_web_search_active_index")]
    pub web_search_active_index: i64,
}

/// Manual `Default` so the `-1` sentinel (anonymous fallback) is used when the
/// bundle is constructed in memory — not just when deserialized. The derived
/// `Default` would give `web_search_active_index: 0`, pointing at a (likely
/// nonexistent) engine. This matters for fresh installs where `load_encrypted`
/// returns `WebSearchBundle::default()` because no file exists yet.
impl Default for WebSearchBundle {
    fn default() -> Self {
        Self {
            web_engines: Vec::new(),
            web_search_active_index: default_web_search_active_index(),
        }
    }
}

fn default_web_search_active_index() -> i64 {
    -1
}

fn websearch_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("websearch.json"))
}

fn load_encrypted(app: &AppHandle) -> Result<WebSearchBundle, String> {
    let path = websearch_path(app)?;
    if !path.exists() {
        return Ok(WebSearchBundle::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;
    let bytes = crypto::decrypt(SCOPE, &payload)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

fn save_encrypted(app: &AppHandle, bundle: &WebSearchBundle) -> Result<(), String> {
    let path = websearch_path(app)?;
    let json = serde_json::to_vec(bundle).map_err(|e| format!("serialize: {e}"))?;
    let payload = crypto::encrypt(SCOPE, &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// One-way migration of web-search data out of legacy plaintext `config.json`.
/// Mirrors `providers::migrate_legacy_from_config`. Runs once at startup after
/// the vault is unlocked. Best-effort: failures are logged and swallowed.
pub fn migrate_legacy_from_config(app: &AppHandle) {
    let path = match websearch_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("websearch migration skipped: {e}");
            return;
        }
    };
    if path.exists() {
        return; // already migrated
    }

    let result = (|| -> Result<(), String> {
        let cfg = crate::commands::config_cmd::read_config(app.clone())?;
        let bundle = WebSearchBundle {
            web_engines: cfg.web_engines.clone(),
            web_search_active_index: cfg.web_search_active_index,
        };
        save_encrypted(app, &bundle)?;

        // Best-effort: blank the migrated fields out of config.json so no
        // plaintext engine data lingers.
        let mut stripped = cfg.clone();
        stripped.web_engines = vec![];
        // web_search_active_index is left in config.json — it is a cheap scalar
        // that the frontend still reads as a fallback, and harmless there. Only
        // the engine structures (with api_key presence) are sensitive.
        let _ = crate::commands::config_cmd::save_config(app.clone(), stripped);
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("websearch migration skipped: {e}");
    }
}

#[tauri::command]
pub fn read_websearch(app: AppHandle) -> Result<WebSearchBundle, String> {
    load_encrypted(&app)
}

#[tauri::command]
pub fn save_websearch(app: AppHandle, bundle: WebSearchBundle) -> Result<(), String> {
    save_encrypted(&app, &bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn test_websearch_bundle_roundtrip() {
        crate::crypto::set_master_key([0x42u8; 32]);
        let bundle = WebSearchBundle {
            web_engines: vec![WebEngineConfig {
                preset: "brave".to_string(),
                api_key: "bsk-test".to_string(),
                enabled: true,
                custom_name: None,
            }],
            web_search_active_index: 0,
        };
        let json = serde_json::to_vec(&bundle).unwrap();
        let payload = crypto::encrypt(SCOPE, &json).unwrap();
        let out = serde_json::to_string_pretty(&payload).unwrap();

        let parsed: EncryptedPayload = serde_json::from_str(&out).unwrap();
        let bytes = crypto::decrypt(SCOPE, &parsed).unwrap();
        let back: WebSearchBundle = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.web_engines.len(), 1);
        assert_eq!(back.web_engines[0].api_key, "bsk-test");
        assert_eq!(back.web_search_active_index, 0);
    }

    #[test]
    fn test_default_index_is_minus_one() {
        crate::crypto::set_master_key([0x42u8; 32]);
        let bundle = WebSearchBundle::default();
        assert_eq!(bundle.web_search_active_index, -1);
        assert!(bundle.web_engines.is_empty());
    }

    #[test]
    fn test_appconfig_still_carries_legacy_fields_for_migration() {
        // Sanity: AppConfig must still parse the legacy plaintext config.json so
        // migrate_legacy_from_config can lift web_engines out of it.
        let json = r#"{
            "providers": [],
            "web_engines": [{"preset":"brave","enabled":true,"api_key":""}],
            "web_search_active_index": 0,
            "active_mode": "translate"
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.web_engines.len(), 1);
        assert_eq!(cfg.web_search_active_index, 0);
    }
}
