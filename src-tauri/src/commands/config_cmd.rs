use crate::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Fallback shortcut used when the saved config can't be read.
const DEFAULT_SHORTCUT: &str = "Alt+Y";

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
    let json = serde_json::to_string_pretty(&config).map_err(|e| format!("serialize: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn set_onboarding_complete(app: AppHandle) {
    if let Some(state) = app.try_state::<crate::state::OnboardingState>() {
        state.set_complete(true);
    }
}

/// Whether the startup reminder has already been shown in this process session.
/// Frontend checks this before deciding to route to /startup-reminder, so a
/// wake-triggered reload (which re-runs main.ts) doesn't re-show the reminder.
#[tauri::command]
pub fn has_shown_startup_reminder(app: AppHandle) -> bool {
    app.try_state::<crate::state::StartupReminderState>()
        .map(|s| s.has_shown())
        .unwrap_or(false)
}

/// Marks the startup reminder as shown for this process session. Called by the
/// frontend after navigating to /startup-reminder.
#[tauri::command]
pub fn mark_startup_reminder_shown(app: AppHandle) {
    if let Some(state) = app.try_state::<crate::state::StartupReminderState>() {
        state.mark_shown();
    }
}

/// Whether the system has resumed from sleep at least once in this process
/// session. The frontend polls this on mount so a wake-triggered remount
/// (which races ahead of the `system-resumed` event listener being wired up)
/// can still detect the wake and force a geometry recompute, fixing the layout
/// corruption (wrong window size / misplaced controls) seen after lid open.
#[cfg(target_os = "windows")]
#[tauri::command]
pub fn woke_since_process_start() -> bool {
    crate::power_watcher::woke_since_process_start()
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
        .unwrap_or_else(|_| DEFAULT_SHORTCUT.to_string())
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
                models: vec![ProviderModel {
                    id: "gpt-4o-mini".to_string(),
                    input_capabilities: None,
                }],
                temperature: Some(0.3),
                max_tokens: Some(1024),
                preset: None,
                api_format: None,
            }],
            active_mode: "translate".to_string(),
            translate_active_provider_index: 0,
            translate_active_model_index: 0,
            skills_lite_active_provider_index: 0,
            skills_lite_active_model_index: 0,
            target_lang: "Japanese".to_string(),
            user_dict_enabled: false,
            custom_languages: vec![],
            language_order: vec![],
            app_lang: "en".to_string(),
            theme: "system".to_string(),
            floating_opacity: 90,
            show_startup_reminder: true,
            history_limit: 100,
            shortcut: "Alt+Y".to_string(),
            mode_shortcut: "Alt+M".to_string(),
            forward_shortcut: "Alt+F".to_string(),
            edit_shortcut: "Alt+E".to_string(),
            skills_prev_shortcut: "Alt+Up".to_string(),
            skills_next_shortcut: "Alt+Down".to_string(),
            launch_on_startup: false,
            show_capability_icons: true,
            web_search_providers: vec![],
            web_search_active_index: -1,
            web_search_enabled_in_skills_lite: false,
            webdav: crate::config::WebdavSettings::default(),
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
