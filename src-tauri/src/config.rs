use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiFormat {
    #[serde(default = "default_auth_header")]
    pub auth_header: String,
    #[serde(default = "default_auth_prefix")]
    pub auth_prefix: String,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    #[serde(default = "default_chat_endpoint")]
    pub chat_endpoint: String,
    #[serde(default)]
    pub models_endpoint: String,
    #[serde(default)]
    pub request: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response: HashMap<String, String>,
}

fn default_auth_header() -> String {
    "Authorization".to_string()
}
fn default_auth_prefix() -> String {
    "Bearer ".to_string()
}
fn default_chat_endpoint() -> String {
    "/chat/completions".to_string()
}

impl ApiFormat {
    pub fn openai_default() -> Self {
        Self {
            auth_header: "Authorization".to_string(),
            auth_prefix: "Bearer ".to_string(),
            extra_headers: HashMap::new(),
            chat_endpoint: "/chat/completions".to_string(),
            models_endpoint: "/models".to_string(),
            request: HashMap::new(),
            response: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub name: String,
    pub provider_name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub api_format: ApiFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(default)]
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<ProviderModel>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub api_format: Option<ApiFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub active_provider_index: usize,
    #[serde(default)]
    pub active_model_index: usize,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
    #[serde(default)]
    pub user_dict_enabled: bool,
    #[serde(default)]
    pub custom_languages: Vec<String>,
    #[serde(default)]
    pub language_order: Vec<String>,
    #[serde(default = "default_app_lang")]
    pub app_lang: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_floating_opacity")]
    pub floating_opacity: u8,
    #[serde(default = "default_true")]
    pub show_startup_reminder: bool,
}

fn default_target_lang() -> String {
    "English".to_string()
}
fn default_theme() -> String {
    "system".to_string()
}
fn default_app_lang() -> String {
    "en".to_string()
}
fn default_floating_opacity() -> u8 {
    90
}
fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            providers: vec![],
            active_provider_index: 0,
            active_model_index: 0,
            target_lang: "English".to_string(),
            user_dict_enabled: false,
            custom_languages: vec![],
            language_order: vec![],
            app_lang: "en".to_string(),
            theme: "system".to_string(),
            floating_opacity: 90,
            show_startup_reminder: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.target_lang, "English");
        assert_eq!(config.active_provider_index, 0);
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let config = AppConfig {
            providers: vec![ProviderConfig {
                name: "OpenAI".to_string(),
                api_key: "".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                models: vec![ProviderModel { id: "gpt-4o-mini".to_string() }],
                temperature: Some(0.3),
                max_tokens: Some(1024),
                preset: Some("OpenAI".to_string()),
                api_format: None,
            }],
            active_provider_index: 0,
            active_model_index: 0,
            target_lang: "Japanese".to_string(),
            user_dict_enabled: false,
            custom_languages: vec!["Klingon".to_string()],
            language_order: vec!["English".to_string(), "Japanese".to_string(), "Klingon".to_string()],
            app_lang: "zh-CN".to_string(),
            theme: "dark".to_string(),
            floating_opacity: 90,
            show_startup_reminder: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.providers[0].name, "OpenAI");
        assert_eq!(deserialized.target_lang, "Japanese");
        assert_eq!(deserialized.custom_languages, vec!["Klingon"]);
        assert_eq!(deserialized.language_order.len(), 3);
        assert_eq!(deserialized.theme, "dark");
        assert_eq!(deserialized.app_lang, "zh-CN");
    }

    #[test]
    fn test_config_deserialize_missing_optional_fields() {
        let json = r#"{
            "providers": [],
            "active_provider_index": 0,
            "active_model_index": 0,
            "target_lang": "English",
            "personas": []
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.target_lang, "English");
        assert_eq!(config.theme, "system");
        assert_eq!(config.app_lang, "en");
    }

    #[test]
    fn test_config_defaults_for_custom_languages() {
        let json = r#"{
            "providers": [],
            "active_provider_index": 0,
            "active_model_index": 0,
            "target_lang": "English"
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.custom_languages.is_empty());
        assert!(config.language_order.is_empty());
        assert_eq!(config.theme, "system");
        assert_eq!(config.app_lang, "en");
    }

    #[test]
    fn test_config_theme_values() {
        for theme in &["light", "dark", "system"] {
            let json = format!(
                r#"{{"providers":[],"active_provider_index":0,"active_model_index":0,"target_lang":"English","theme":"{}"}}"#,
                theme
            );
            let config: AppConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(config.theme, *theme);
        }
    }
}
