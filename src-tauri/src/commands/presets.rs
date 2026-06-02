use crate::config::ProviderPreset;

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
