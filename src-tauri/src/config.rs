use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub display_name: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub models: Vec<ModelConfig>,
    #[serde(default)]
    pub selected_model_index: usize,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
    #[serde(default)]
    pub persona: String,
}

fn default_target_lang() -> String {
    "English".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            models: vec![],
            selected_model_index: 0,
            target_lang: "English".to_string(),
            persona: String::new(),
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
        assert_eq!(config.selected_model_index, 0);
        assert!(config.models.is_empty());
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let config = AppConfig {
            models: vec![ModelConfig {
                api_key: "sk-test".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                display_name: "GPT-4o Mini".to_string(),
                temperature: Some(0.3),
                max_tokens: Some(1024),
            }],
            selected_model_index: 0,
            target_lang: "Japanese".to_string(),
            persona: "formal".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.models[0].api_key, "sk-test");
        assert_eq!(deserialized.target_lang, "Japanese");
    }

    #[test]
    fn test_config_deserialize_missing_optional_fields() {
        let json = r#"{
            "models": [],
            "selected_model_index": 0,
            "target_lang": "English",
            "persona": ""
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.target_lang, "English");
    }
}
