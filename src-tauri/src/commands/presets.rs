use crate::config::{ModelInputCapabilities, ProviderPreset};

#[derive(serde::Deserialize)]
struct PresetsFile {
    presets: Vec<ProviderPreset>,
}

#[tauri::command]
pub fn read_provider_presets() -> Result<Vec<ProviderPreset>, String> {
    let raw = include_str!("../../../provider-presets.json");
    let file: PresetsFile = serde_json::from_str(raw).map_err(|e| format!("parse presets: {e}"))?;
    Ok(file.presets)
}

#[derive(serde::Deserialize)]
struct ModelCapabilityEntry {
    id: String,
    input_capabilities: ModelInputCapabilities,
}

#[derive(serde::Deserialize)]
struct ModelCapabilitiesFile {
    models: Vec<ModelCapabilityEntry>,
}

#[derive(serde::Serialize)]
pub struct ModelCapabilityItem {
    pub id: String,
    pub input_capabilities: ModelInputCapabilities,
}

#[tauri::command]
pub fn read_model_capabilities() -> Result<Vec<ModelCapabilityItem>, String> {
    let raw = include_str!("../../../model-capabilities.json");
    let file: ModelCapabilitiesFile =
        serde_json::from_str(raw).map_err(|e| format!("parse model capabilities: {e}"))?;
    Ok(file
        .models
        .into_iter()
        .map(|e| ModelCapabilityItem {
            id: e.id,
            input_capabilities: e.input_capabilities,
        })
        .collect())
}
