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
    pub response: HashMap<String, String>,
    #[serde(default)]
    pub system_key: String,
    #[serde(default)]
    pub force_fields: Vec<String>,
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
            response: HashMap::new(),
            system_key: String::new(),
            force_fields: Vec::new(),
        }
    }

    /// Anthropic Messages API format (the only non-OpenAI built-in).
    pub fn anthropic_default() -> Self {
        let mut extra_headers = HashMap::new();
        extra_headers.insert(
            "anthropic-version".to_string(),
            "2023-06-01".to_string(),
        );
        let mut response = HashMap::new();
        response.insert("content".to_string(), "content.0.text".to_string());
        Self {
            auth_header: "x-api-key".to_string(),
            auth_prefix: String::new(),
            extra_headers,
            chat_endpoint: "/messages".to_string(),
            models_endpoint: "/models".to_string(),
            response,
            system_key: "system".to_string(),
            force_fields: vec!["max_tokens".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresetVariantEndpoint {
    pub key: String,
    #[serde(default)]
    pub label: String,
    pub provider_name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresetVariantRegion {
    pub key: String,
    #[serde(default)]
    pub label: String,
    pub endpoints: Vec<PresetVariantEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresetVariants {
    #[serde(default)]
    pub default_region: String,
    #[serde(default)]
    pub default_endpoint: String,
    #[serde(default)]
    pub regions: Vec<PresetVariantRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreset {
    pub name: String,
    #[serde(default)]
    pub provider_name: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default, deserialize_with = "deserialize_model_series",)]
    pub model_series: Vec<String>,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default = "default_api_format", deserialize_with = "deserialize_api_format")]
    pub api_format: ApiFormat,
    #[serde(default)]
    pub is_local: bool,
    #[serde(default)]
    pub variants: Option<PresetVariants>,
}

/// Accept `model_series` as either a single string (`"Step"`) or an array of
/// strings (`["SenseNova","DeepSeek"]`), normalizing to `Vec<String>`.
fn deserialize_model_series<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Series {
        Single(String),
        Multi(Vec<String>),
    }
    Ok(match Option::<Series>::deserialize(deserializer)? {
        Some(Series::Single(s)) => vec![s],
        Some(Series::Multi(v)) => v,
        None => Vec::new(),
    })
}

fn default_api_format() -> ApiFormat {
    ApiFormat::openai_default()
}

/// Accept `api_format` as either a built-in tag string (`"openai"` /
/// `"anthropic"`, defaulting to OpenAI when absent) or a full inline object
/// for custom formats. This keeps provider-presets.json terse — OpenAI /
/// Anthropic presets only need a tag instead of repeating the whole object.
fn deserialize_api_format<'de, D>(deserializer: D) -> Result<ApiFormat, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Tag(String),
        Inline(ApiFormat),
    }
    Ok(match Option::<Raw>::deserialize(deserializer)? {
        Some(Raw::Tag(tag)) => match tag.as_str() {
            "anthropic" => ApiFormat::anthropic_default(),
            // "openai" and any unknown tag fall back to OpenAI defaults.
            _ => ApiFormat::openai_default(),
        },
        Some(Raw::Inline(fmt)) => fmt,
        None => ApiFormat::openai_default(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_capabilities: Option<ModelInputCapabilities>,
}

/// Multimodal INPUT capabilities of a model. Parent dimension for all input
/// modalities. Adding a new modality = one field here + one detection case on
/// the front-end. Today only `image` is implemented; `audio`/`video` are
/// reserved for future additions as peer `Option<bool>` fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelInputCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<bool>,
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
pub struct WebSearchProviderConfig {
    pub preset: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub active_mode: String,
    #[serde(default, alias = "translation_active_provider_index")]
    pub translate_active_provider_index: usize,
    #[serde(default, alias = "translation_active_model_index")]
    pub translate_active_model_index: usize,
    #[serde(default, alias = "sparkle_active_provider_index")]
    pub skills_lite_active_provider_index: usize,
    #[serde(default, alias = "sparkle_active_model_index")]
    pub skills_lite_active_model_index: usize,
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
    #[serde(default = "default_history_limit")]
    pub history_limit: u32,
    #[serde(default = "default_shortcut")]
    pub shortcut: String,
    #[serde(default = "default_mode_shortcut")]
    pub mode_shortcut: String,
    #[serde(default)]
    pub launch_on_startup: bool,
    #[serde(default, skip_serializing)]
    pub show_capability_icons: bool,
    #[serde(default)]
    pub web_search_providers: Vec<WebSearchProviderConfig>,
    #[serde(default = "default_web_search_active_index")]
    pub web_search_active_index: i64,
    #[serde(default, alias = "web_search_enabled_in_sparkle")]
    pub web_search_enabled_in_skills_lite: bool,
}

fn default_target_lang() -> String {
    "English".to_string()
}
fn default_theme() -> String {
    "system".to_string()
}
fn default_app_lang() -> String {
    // "auto" = follow the OS UI locale on each launch until the user explicitly
    // picks a language. Resolved in the frontend via `navigator.language`
    // (see `resolveAppLang`).
    "auto".to_string()
}
fn default_floating_opacity() -> u8 {
    90
}
fn default_true() -> bool {
    true
}
fn default_history_limit() -> u32 {
    50
}
fn default_shortcut() -> String {
    "Alt+Y".to_string()
}
fn default_mode_shortcut() -> String {
    "Alt+M".to_string()
}
/// `-1` means "no provider selected" (no built-in fallback anymore).
fn default_web_search_active_index() -> i64 {
    -1
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            providers: vec![],
            active_mode: "translate".to_string(),
            translate_active_provider_index: 0,
            translate_active_model_index: 0,
            skills_lite_active_provider_index: 0,
            skills_lite_active_model_index: 0,
            target_lang: "English".to_string(),
            user_dict_enabled: false,
            custom_languages: vec![],
            language_order: vec![],
            app_lang: "auto".to_string(),
            theme: "system".to_string(),
            floating_opacity: 90,
            show_startup_reminder: true,
            history_limit: 50,
            shortcut: "Alt+Y".to_string(),
            mode_shortcut: "Alt+M".to_string(),
            launch_on_startup: false,
            show_capability_icons: false,
            web_search_providers: vec![],
            web_search_active_index: -1,
            web_search_enabled_in_skills_lite: false,
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
        assert_eq!(config.active_mode, "translate");
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_config_serialize_roundtrip() {
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
                preset: Some("OpenAI".to_string()),
                api_format: None,
            }],
            active_mode: "translate".to_string(),
            translate_active_provider_index: 0,
            translate_active_model_index: 0,
            skills_lite_active_provider_index: 0,
            skills_lite_active_model_index: 0,
            target_lang: "Japanese".to_string(),
            user_dict_enabled: false,
            custom_languages: vec!["Klingon".to_string()],
            language_order: vec![
                "English".to_string(),
                "Japanese".to_string(),
                "Klingon".to_string(),
            ],
            app_lang: "zh-CN".to_string(),
            theme: "dark".to_string(),
            floating_opacity: 90,
            show_startup_reminder: true,
            history_limit: 50,
            shortcut: "Alt+Y".to_string(),
            mode_shortcut: "Alt+M".to_string(),
            launch_on_startup: false,
            show_capability_icons: false,
            web_search_providers: vec![],
            web_search_active_index: -1,
            web_search_enabled_in_skills_lite: false,
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
            "active_mode": "translate",
            "translation_active_provider_index": 0,
            "translation_active_model_index": 0,
            "target_lang": "English",
            "personas": []
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.target_lang, "English");
        assert_eq!(config.theme, "system");
        assert_eq!(config.app_lang, "auto");
    }

    #[test]
    fn test_provider_model_input_capabilities_roundtrip() {
        let json = r#"{"id":"gpt-4o","input_capabilities":{"image":true}}"#;
        let m: ProviderModel = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, "gpt-4o");
        assert_eq!(m.input_capabilities.as_ref().unwrap().image, Some(true));
    }

    #[test]
    fn test_provider_model_without_capabilities_loads() {
        // Old config.json files have no input_capabilities field — must load fine.
        let json = r#"{"id":"deepseek-chat"}"#;
        let m: ProviderModel = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, "deepseek-chat");
        assert!(m.input_capabilities.is_none());
    }

    #[test]
    fn test_provider_model_skips_empty_capabilities_on_serialize() {
        // Unknown-capability models must not pollute config.json with empty {}.
        let m = ProviderModel {
            id: "x".to_string(),
            input_capabilities: None,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("input_capabilities"));
    }

    #[test]
    fn test_config_defaults_for_custom_languages() {
        let json = r#"{
            "providers": [],
            "active_mode": "translate",
            "translation_active_provider_index": 0,
            "translation_active_model_index": 0,
            "target_lang": "English"
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.custom_languages.is_empty());
        assert!(config.language_order.is_empty());
        assert_eq!(config.theme, "system");
        assert_eq!(config.app_lang, "auto");
        assert_eq!(config.history_limit, 50);
    }

    #[test]
    fn test_config_theme_values() {
        for theme in &["light", "dark", "system"] {
            let json = format!(
                r#"{{"providers":[],"active_mode":"translate","translation_active_provider_index":0,"translation_active_model_index":0,"target_lang":"English","theme":"{}"}}"#,
                theme
            );
            let config: AppConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(config.theme, *theme);
        }
    }

    #[test]
    fn test_per_mode_active_indices_survive_roundtrip() {
        // Both translate_active_* and skills_lite_active_* must persist through a
        // save_config → read_config cycle. Field names must match the
        // `active_mode` ids ("translate"/"skills_lite") used for dynamic access.
        let json = r#"{
            "providers": [],
            "active_mode": "skills_lite",
            "translate_active_provider_index": 3,
            "translate_active_model_index": 1,
            "skills_lite_active_provider_index": 2,
            "skills_lite_active_model_index": 4,
            "target_lang": "English"
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        let written = serde_json::to_string_pretty(&config).unwrap();
        let reloaded: AppConfig = serde_json::from_str(&written).unwrap();

        assert_eq!(reloaded.translate_active_provider_index, 3);
        assert_eq!(reloaded.translate_active_model_index, 1);
        assert_eq!(reloaded.skills_lite_active_provider_index, 2);
        assert_eq!(reloaded.skills_lite_active_model_index, 4);
        // Written JSON must use the new names.
        assert!(written.contains("translate_active_provider_index"));
        assert!(written.contains("translate_active_model_index"));
        assert!(written.contains("skills_lite_active_provider_index"));
        assert!(written.contains("skills_lite_active_model_index"));
    }

    #[test]
    fn test_legacy_sparkle_active_alias_migrates() {
        // Old config files used `sparkle_active_*` / `web_search_enabled_in_sparkle`
        // (note: "sparkle"). The new fields are `skills_lite_active_*` /
        // `web_search_enabled_in_skills_lite`; serde aliases keep reads working.
        let json = r#"{
            "providers": [],
            "active_mode": "translate",
            "sparkle_active_provider_index": 2,
            "sparkle_active_model_index": 4,
            "web_search_enabled_in_sparkle": true,
            "target_lang": "English"
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.skills_lite_active_provider_index, 2);
        assert_eq!(config.skills_lite_active_model_index, 4);
        assert!(config.web_search_enabled_in_skills_lite);
        // Once re-serialized, the new name is written (migration is one-way).
        let written = serde_json::to_string(&config).unwrap();
        assert!(written.contains("skills_lite_active_provider_index"));
        assert!(!written.contains("sparkle_active_"));
        assert!(written.contains("web_search_enabled_in_skills_lite"));
        assert!(!written.contains("web_search_enabled_in_sparkle"));
    }

    #[test]
    fn test_legacy_translation_active_alias_migrates() {
        // Old config files used `translation_active_*` (note: "translation").
        // The new field is `translate_active_*`; serde alias keeps reads working.
        let json = r#"{
            "providers": [],
            "active_mode": "translate",
            "translation_active_provider_index": 3,
            "translation_active_model_index": 2,
            "target_lang": "English"
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.translate_active_provider_index, 3);
        assert_eq!(config.translate_active_model_index, 2);
        // Once re-serialized, the new name is written (migration is one-way).
        let written = serde_json::to_string(&config).unwrap();
        assert!(written.contains("translate_active_provider_index"));
        assert!(!written.contains("translation_active_"));
    }

    #[test]
    fn test_show_capability_icons_not_persisted() {
        // show_capability_icons is a build-time switch; it must NOT appear in
        // the serialized JSON so that changing the code default always wins.
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("show_capability_icons"));
        // But deserializing a config that happens to contain it should still work.
        let json_with = r#"{"providers":[],"active_mode":"translate","target_lang":"English","show_capability_icons":true}"#;
        let c2: AppConfig = serde_json::from_str(json_with).unwrap();
        assert!(c2.show_capability_icons);
    }
}
