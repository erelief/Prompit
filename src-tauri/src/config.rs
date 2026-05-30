use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub models: Vec<ProviderModel>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
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
}

fn default_target_lang() -> String {
    "English".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            providers: vec![],
            active_provider_index: 0,
            active_model_index: 0,
            target_lang: "English".to_string(),
            user_dict_enabled: false,
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
            }],
            active_provider_index: 0,
            active_model_index: 0,
            target_lang: "Japanese".to_string(),
            user_dict_enabled: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.providers[0].name, "OpenAI");
        assert_eq!(deserialized.target_lang, "Japanese");
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
    }
}
