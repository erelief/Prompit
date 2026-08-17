use std::path::PathBuf;

use tauri::AppHandle;

use crate::config::{ModelInputCapabilities, ProviderPreset};

/// Built-in presets, compiled into the binary: the always-available fallback
/// layer (offline, first run, or when the remote layer fails).
const BUILTIN_RAW: &str = include_str!("../../data/provider-presets.json");

/// Hot-update layer source. Pinned to the project's own GitHub repo over
/// HTTPS — preset `base_url`s decide where users' API keys get sent, so the
/// URL is a compile-time constant and never user input. `PROMPIT_PRESETS_URL`
/// exists only for dev/testing (e.g. a local mock server).
const REMOTE_PRESETS_URL: &str =
    "https://raw.githubusercontent.com/erelief/Prompit/main/src-tauri/data/provider-presets.json";
const REMOTE_URL_ENV: &str = "PROMPIT_PRESETS_URL";

/// Hard cap on the remote file size (the built-in file is ~30 KB); anything
/// larger is rejected before parsing.
const MAX_REMOTE_BYTES: usize = 1024 * 1024;

const REMOTE_CACHE_FILE: &str = "provider-presets.remote.json";

#[derive(serde::Deserialize)]
struct PresetsFile {
    /// Monotonic version; bump it on every remote-side edit. Missing/0 marks
    /// an unversioned (legacy or hand-made) file, which the refresh path
    /// refuses to trust.
    #[serde(default)]
    version: u64,
    presets: Vec<ProviderPreset>,
}

fn parse_presets(raw: &str) -> Result<PresetsFile, String> {
    serde_json::from_str(raw).map_err(|e| format!("parse presets: {e}"))
}

/// Sanity-check a remote file before trusting it: it must carry a version and
/// every preset a non-empty name.
fn validate_remote(file: &PresetsFile) -> Result<(), String> {
    if file.version == 0 {
        return Err("remote presets file has no version".to_string());
    }
    if file.presets.iter().any(|p| p.name.trim().is_empty()) {
        return Err("remote presets file contains a preset with an empty name".to_string());
    }
    Ok(())
}

/// The layer currently in effect: the remote cache when (and only when) its
/// version is strictly newer than the built-in file. Version comparison is
/// the whole merge rule — a reverted or broken remote can never downgrade the
/// list a user already has.
fn pick_effective(builtin: PresetsFile, cached: Option<PresetsFile>) -> PresetsFile {
    match cached {
        Some(c) if c.version > builtin.version => c,
        _ => builtin,
    }
}

fn remote_cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::get_data_dir(app)?.join(REMOTE_CACHE_FILE))
}

/// Load the cached remote file. A missing or corrupt cache is not an error —
/// the built-in layer simply stays in effect.
fn read_remote_cache(app: &AppHandle) -> Option<PresetsFile> {
    let path = remote_cache_path(app).ok()?;
    let raw = std::fs::read_to_string(path).ok()?;
    let file = parse_presets(&raw).ok()?;
    validate_remote(&file).ok()?;
    Some(file)
}

fn current_presets(app: &AppHandle) -> Result<PresetsFile, String> {
    Ok(pick_effective(parse_presets(BUILTIN_RAW)?, read_remote_cache(app)))
}

#[tauri::command]
pub fn read_provider_presets(app: AppHandle) -> Result<Vec<ProviderPreset>, String> {
    Ok(current_presets(&app)?.presets)
}

#[derive(serde::Serialize)]
pub struct PresetsRefreshResult {
    /// True when the remote file was newer than what the app already had and
    /// the cache was updated.
    pub updated: bool,
    pub preset_count: usize,
}

/// Fetch the latest preset list from GitHub and cache it on disk when it is
/// newer than the layer currently in effect. Runs through the same proxy
/// discovery (`read_proxy_url`) as `llm_http` and the updater, so it works
/// behind the same system proxy.
#[tauri::command]
pub async fn refresh_provider_presets(app: AppHandle) -> Result<PresetsRefreshResult, String> {
    let url = std::env::var(REMOTE_URL_ENV).unwrap_or_else(|_| REMOTE_PRESETS_URL.to_string());

    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(3))
        .user_agent(concat!("Prompit/", env!("CARGO_PKG_VERSION")));
    if let Some(proxy_url) = crate::read_proxy_url() {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    let client = builder
        .build()
        .map_err(|e| format!("build client: {e}"))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("remote returned HTTP {}", response.status()));
    }
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("read body: {e}"))?;
    if body.len() > MAX_REMOTE_BYTES {
        return Err(format!(
            "remote presets file too large ({} bytes)",
            body.len()
        ));
    }
    let raw =
        String::from_utf8(body.to_vec()).map_err(|e| format!("remote presets not utf-8: {e}"))?;

    let remote = parse_presets(&raw)?;
    validate_remote(&remote)?;

    let current = current_presets(&app)?;
    if remote.version <= current.version {
        return Ok(PresetsRefreshResult {
            updated: false,
            preset_count: current.presets.len(),
        });
    }

    // Atomic-ish replace: write a temp file, then rename over the old cache,
    // so a crash mid-write never leaves a truncated cache behind.
    let path = remote_cache_path(&app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create dir: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &raw).map_err(|e| format!("write cache: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("replace cache: {e}"))?;

    Ok(PresetsRefreshResult {
        updated: true,
        preset_count: remote.presets.len(),
    })
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
    let raw = include_str!("../../data/model-capabilities.json");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_file_is_versioned_and_wellformed() {
        let builtin = parse_presets(BUILTIN_RAW).unwrap();
        assert!(builtin.version >= 1, "built-in presets must carry a version");
        assert!(!builtin.presets.is_empty());
        assert!(builtin.presets.iter().all(|p| !p.name.trim().is_empty()));
    }

    #[test]
    fn validate_remote_rejects_unversioned_file() {
        let file = parse_presets(r#"{"presets":[{"name":"X"}]}"#).unwrap();
        assert!(validate_remote(&file).is_err());
    }

    #[test]
    fn validate_remote_rejects_empty_preset_name() {
        let file = parse_presets(r#"{"version":2,"presets":[{"name":"  "}]}"#).unwrap();
        assert!(validate_remote(&file).is_err());
    }

    #[test]
    fn pick_effective_prefers_newer_cache() {
        let builtin = parse_presets(r#"{"version":1,"presets":[]}"#).unwrap();
        let newer = parse_presets(r#"{"version":2,"presets":[{"name":"X"}]}"#).unwrap();
        let picked = pick_effective(builtin, Some(newer));
        assert_eq!(picked.version, 2);
        assert_eq!(picked.presets.len(), 1);
    }

    #[test]
    fn pick_effective_keeps_builtin_on_tie_or_older_cache() {
        let newer_builtin = parse_presets(r#"{"version":3,"presets":[{"name":"B"}]}"#).unwrap();
        let tied = parse_presets(r#"{"version":3,"presets":[]}"#).unwrap();
        assert_eq!(pick_effective(newer_builtin, Some(tied)).presets[0].name, "B");

        let builtin = parse_presets(r#"{"version":5,"presets":[]}"#).unwrap();
        let older = parse_presets(r#"{"version":4,"presets":[]}"#).unwrap();
        assert_eq!(pick_effective(builtin, Some(older)).version, 5);
    }
}
