use crate::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PersonaConfig, ProviderConfig, ProviderModel};
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
            }],
            active_provider_index: 0,
            active_model_index: 0,
            target_lang: "Japanese".to_string(),
            personas: vec![PersonaConfig {
                name: "Formal".to_string(),
                prompt: "Translate in a formal tone".to_string(),
                enabled: true,
            }],
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
        assert_eq!(loaded.personas.len(), 1);
        assert_eq!(loaded.personas[0].name, "Formal");
        assert!(loaded.personas[0].enabled);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_empty_config_returns_default() {
        let json = r#"{"providers":[],"active_provider_index":0,"active_model_index":0,"target_lang":"English","personas":[]}"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.providers.is_empty());
        assert!(config.personas.is_empty());
        assert_eq!(config.target_lang, "English");
    }
}
