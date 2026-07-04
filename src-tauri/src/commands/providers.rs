use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::config::ProviderConfig;
use crate::crypto::{self, EncryptedPayload};

/// Crypto scope for the AI-provider bundle (provider list + their active
/// indices). The provider `api_key` fields are encrypted at rest here, so they
/// travel with the bundle and never land in plaintext `config.json`.
const SCOPE: &str = "providers";

/// The encrypted, on-disk AI-provider bundle. Self-contained: the provider
/// array plus every per-mode active index that references it, so importing this
/// file alone restores the full "which provider/model is selected" state. The
/// API keys live inline on each `ProviderConfig.api_key` and are encrypted
/// together with the rest of this struct (no separate secrets store).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersBundle {
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default, alias = "translation_active_provider_index")]
    pub translate_active_provider_index: usize,
    #[serde(default, alias = "translation_active_model_index")]
    pub translate_active_model_index: usize,
    #[serde(default, alias = "sparkle_active_provider_index")]
    pub skills_lite_active_provider_index: usize,
    #[serde(default, alias = "sparkle_active_model_index")]
    pub skills_lite_active_model_index: usize,
}

fn providers_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("providers.json"))
}

fn load_encrypted(app: &AppHandle) -> Result<ProvidersBundle, String> {
    let path = providers_path(app)?;
    if !path.exists() {
        return Ok(ProvidersBundle::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let payload: EncryptedPayload =
        serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;
    let bytes = crypto::decrypt(SCOPE, &payload)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize: {e}"))
}

fn save_encrypted(app: &AppHandle, bundle: &ProvidersBundle) -> Result<(), String> {
    let path = providers_path(app)?;
    let json = serde_json::to_vec(bundle).map_err(|e| format!("serialize: {e}"))?;
    let payload = crypto::encrypt(SCOPE, &json)?;
    let out = serde_json::to_string_pretty(&payload).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

/// One-way migration of provider data out of legacy plaintext `config.json`.
///
/// Historically `providers[]` and the four per-mode active indices lived in
/// plaintext `config.json` (with `api_key` blanked there and the real key held
/// positionally in `secrets.json` as `provider_<i>`). They have been promoted
/// into this encrypted file so the whole provider configuration — structure
/// AND keys AND selection state — travels together and stays protected at rest.
///
/// This runs once at startup. If `providers.json` already exists it wins and
/// nothing is migrated; otherwise we lift the fields out of `config.json` and
/// persist them here. Best-effort: any failure is logged and swallowed — the
/// frontend also has a read-side fallback.
pub fn migrate_legacy_from_config(app: &AppHandle) {
    let path = match providers_path(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("providers migration skipped: {e}");
            return;
        }
    };
    if path.exists() {
        return; // already migrated
    }

    let result = (|| -> Result<(), String> {
        let cfg = crate::commands::config_cmd::read_config(app.clone())?;
        let bundle = ProvidersBundle {
            providers: cfg.providers.clone(),
            translate_active_provider_index: cfg.translate_active_provider_index,
            translate_active_model_index: cfg.translate_active_model_index,
            skills_lite_active_provider_index: cfg.skills_lite_active_provider_index,
            skills_lite_active_model_index: cfg.skills_lite_active_model_index,
        };
        save_encrypted(app, &bundle)?;

        // Best-effort: blank the migrated fields out of config.json so no
        // plaintext provider data lingers. The frontend re-reads config.json
        // fresh on next launch; rewriting it with empty arrays here keeps the
        // two stores from disagreeing.
        let mut stripped = cfg.clone();
        stripped.providers = vec![];
        let _ = crate::commands::config_cmd::save_config(app.clone(), stripped);
        Ok(())
    })();

    if let Err(e) = result {
        eprintln!("providers migration skipped: {e}");
    }
}

#[tauri::command]
pub fn read_providers(app: AppHandle) -> Result<ProvidersBundle, String> {
    load_encrypted(&app)
}

/// Read providers, applying the `PROMPIT_API_KEY_<i>` environment-variable
/// override per provider index (dev isolation workflow, ported from the old
/// positional-secrets path). If the env var is set and non-empty, it takes
/// precedence over the on-disk api_key for that provider.
#[tauri::command]
pub fn read_providers_resolved(app: AppHandle) -> Result<ProvidersBundle, String> {
    let mut bundle = load_encrypted(&app)?;
    for (i, p) in bundle.providers.iter_mut().enumerate() {
        let env_name = format!("PROMPIT_API_KEY_{}", i);
        if let Ok(val) = std::env::var(&env_name) {
            if !val.is_empty() {
                p.api_key = val;
            }
        }
    }
    Ok(bundle)
}

#[tauri::command]
pub fn save_providers(app: AppHandle, bundle: ProvidersBundle) -> Result<(), String> {
    save_encrypted(&app, &bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    #[test]
    fn test_providers_bundle_roundtrip() {
        crate::crypto::set_master_key([0x42u8; 32]);
        let bundle = ProvidersBundle {
            providers: vec![ProviderConfig {
                name: "OpenAI".to_string(),
                api_key: "sk-test".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                models: vec![],
                temperature: Some(0.3),
                max_tokens: None,
                preset: Some("OpenAI".to_string()),
                api_format: None,
            }],
            translate_active_provider_index: 0,
            translate_active_model_index: 2,
            skills_lite_active_provider_index: 1,
            skills_lite_active_model_index: 0,
        };
        let json = serde_json::to_vec(&bundle).unwrap();
        let payload = crypto::encrypt(SCOPE, &json).unwrap();
        let out = serde_json::to_string_pretty(&payload).unwrap();

        let parsed: EncryptedPayload = serde_json::from_str(&out).unwrap();
        let bytes = crypto::decrypt(SCOPE, &parsed).unwrap();
        let back: ProvidersBundle = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.providers.len(), 1);
        assert_eq!(back.providers[0].api_key, "sk-test");
        assert_eq!(back.translate_active_model_index, 2);
    }

    #[test]
    fn test_legacy_alias_translation_migrates() {
        let legacy = r#"{
            "providers": [],
            "translation_active_provider_index": 3,
            "translation_active_model_index": 1
        }"#;
        let bundle: ProvidersBundle = serde_json::from_str(legacy).unwrap();
        assert_eq!(bundle.translate_active_provider_index, 3);
        assert_eq!(bundle.translate_active_model_index, 1);
    }

    #[test]
    fn test_legacy_alias_sparkle_migrates() {
        let legacy = r#"{
            "providers": [],
            "sparkle_active_provider_index": 2,
            "sparkle_active_model_index": 4
        }"#;
        let bundle: ProvidersBundle = serde_json::from_str(legacy).unwrap();
        assert_eq!(bundle.skills_lite_active_provider_index, 2);
        assert_eq!(bundle.skills_lite_active_model_index, 4);
    }

    #[test]
    fn test_appconfig_still_carries_legacy_fields_for_migration() {
        // Sanity: AppConfig must still parse the legacy plaintext config.json so
        // migrate_legacy_from_config can lift providers out of it.
        let json = r#"{
            "providers": [{"name":"X","base_url":"u","models":[]}],
            "translate_active_provider_index": 1,
            "active_mode": "translate"
        }"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.translate_active_provider_index, 1);
    }
}
