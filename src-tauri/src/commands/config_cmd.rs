use crate::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("config.json"))
}

#[tauri::command]
pub fn read_config(app: AppHandle) -> Result<AppConfig, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    let json =
        serde_json::to_string_pretty(&config).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn set_onboarding_complete(app: AppHandle) {
    if let Some(state) = app.try_state::<crate::state::OnboardingState>() {
        state.set_complete(true);
    }
}

#[tauri::command]
pub fn get_config_dir(app: AppHandle) -> Result<String, String> {
    let dir = crate::get_data_dir(&app)?;
    Ok(dir.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_shortcut_label(app: AppHandle) -> String {
    read_config(app)
        .map(|c| c.shortcut)
        .unwrap_or_else(|_| "Alt+Y".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderConfig, ProviderModel};
    use std::io::Write;

    #[test]
    fn test_config_roundtrip_via_json() {
        let config = AppConfig {
            providers: vec![ProviderConfig {
                name: "OpenAI".to_string(),
                api_key: "".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                models: vec![ProviderModel { id: "gpt-4o-mini".to_string() }],
                temperature: Some(0.3),
                max_tokens: Some(1024),
                preset: None,
                api_format: None,
            }],
            active_mode: "translate".to_string(),
            translation_active_provider_index: 0,
            translation_active_model_index: 0,
            target_lang: "Japanese".to_string(),
            user_dict_enabled: false,
            custom_languages: vec![],
            language_order: vec![],
            app_lang: "en".to_string(),
            theme: "system".to_string(),
            floating_opacity: 90,
            show_startup_reminder: true,
            history_limit: 50,
            shortcut: "Alt+Y".to_string(),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let dir = std::env::temp_dir().join("translator_test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let loaded: AppConfig = serde_json::from_str(&content).unwrap();

        assert_eq!(loaded.providers.len(), 1);
        assert_eq!(loaded.providers[0].name, "OpenAI");
        assert_eq!(loaded.target_lang, "Japanese");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_empty_config_returns_default() {
        let json = r#"{"providers":[],"active_mode":"translate","translation_active_provider_index":0,"translation_active_model_index":0,"target_lang":"English"}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.providers.is_empty());
        assert_eq!(config.target_lang, "English");
    }
}
