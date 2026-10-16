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

/// Shared shape of the built-in file and the GitHub copy. Unknown fields are
/// ignored — the data file still carries a top-level `version` solely for the
/// 0.8.0 release, whose refresh logic compared it; current builds never read
/// it (freshness is decided by content comparison instead).
#[derive(serde::Deserialize)]
struct PresetsFile {
    presets: Vec<ProviderPreset>,
}

/// On-disk snapshot of the last fetched GitHub copy, tagged with the app
/// version that wrote it. A cache from an older build is ignored: a fresh
/// release's built-in list is at least as new as anything the old build could
/// have fetched (releases are cut from main after preset edits land there).
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheFile {
    app_version: String,
    presets: Vec<ProviderPreset>,
}

fn parse_presets(raw: &str) -> Result<PresetsFile, String> {
    serde_json::from_str(raw).map_err(|e| format!("parse presets: {e}"))
}

/// Sanity-check a preset list before trusting it: every preset needs a name.
fn validate_presets(presets: &[ProviderPreset]) -> Result<(), String> {
    if presets.iter().any(|p| p.name.trim().is_empty()) {
        return Err("presets file contains a preset with an empty name".to_string());
    }
    Ok(())
}

/// Semantic equality of two preset lists (formatting / key-order insensitive).
fn same_presets(a: &[ProviderPreset], b: &[ProviderPreset]) -> bool {
    match (serde_json::to_value(a), serde_json::to_value(b)) {
        (Ok(va), Ok(vb)) => va == vb,
        _ => false,
    }
}

/// Drop sandbox-only presets unless the app runs in sandbox mode. Applied only
/// where the list leaves the backend — the cache and the refresh comparison
/// always work on the full list.
fn visible_presets(presets: &[ProviderPreset], sandbox: bool) -> Vec<ProviderPreset> {
    presets
        .iter()
        .filter(|p| sandbox || !p.sandbox_only)
        .cloned()
        .collect()
}

fn remote_cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::get_data_dir(app)?.join(REMOTE_CACHE_FILE))
}

/// Extract the preset list from a cache blob, but only when it was written by
/// this exact app version and passes validation. Anything else (older build,
/// foreign shape, corrupt JSON) is treated as "no cache" — the built-in layer
/// simply stays in effect.
fn current_cache_presets(raw: &str) -> Option<Vec<ProviderPreset>> {
    let cache: CacheFile = serde_json::from_str(raw).ok()?;
    if cache.app_version != env!("CARGO_PKG_VERSION") {
        return None;
    }
    validate_presets(&cache.presets).ok()?;
    Some(cache.presets)
}

fn read_remote_cache(app: &AppHandle) -> Option<Vec<ProviderPreset>> {
    let raw = std::fs::read_to_string(remote_cache_path(app).ok()?).ok()?;
    current_cache_presets(&raw)
}

/// The layer currently in effect: the cached GitHub copy when one exists for
/// this app version, otherwise the built-in list.
fn current_presets(app: &AppHandle) -> Result<Vec<ProviderPreset>, String> {
    if let Some(cached) = read_remote_cache(app) {
        return Ok(cached);
    }
    Ok(parse_presets(BUILTIN_RAW)?.presets)
}

#[tauri::command]
pub fn read_provider_presets(app: AppHandle) -> Result<Vec<ProviderPreset>, String> {
    Ok(visible_presets(
        &current_presets(&app)?,
        crate::sandbox_enabled(),
    ))
}

#[derive(serde::Serialize)]
pub struct PresetsRefreshResult {
    /// True when the remote list differed from what the app already had and
    /// the cache was updated.
    pub updated: bool,
    pub preset_count: usize,
}

/// Fetch the latest preset list from GitHub and cache it on disk when its
/// content differs from the list currently in effect. Runs through the same
/// proxy discovery (`read_proxy_url`) as `llm_http` and the updater, so it
/// works behind the same system proxy.
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
    validate_presets(&remote.presets)?;

    let current = current_presets(&app)?;
    let sandbox = crate::sandbox_enabled();
    if same_presets(&remote.presets, &current) {
        return Ok(PresetsRefreshResult {
            updated: false,
            preset_count: visible_presets(&current, sandbox).len(),
        });
    }

    // Atomic-ish replace: write a temp file, then rename over the old cache,
    // so a crash mid-write never leaves a truncated cache behind.
    let path = remote_cache_path(&app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create dir: {e}"))?;
    }
    let visible_count = visible_presets(&remote.presets, sandbox).len();
    let cache = CacheFile {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        presets: remote.presets,
    };
    let out = serde_json::to_vec(&cache).map_err(|e| format!("serialize cache: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, out).map_err(|e| format!("write cache: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("replace cache: {e}"))?;

    Ok(PresetsRefreshResult {
        updated: true,
        preset_count: visible_count,
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
    fn builtin_file_is_wellformed() {
        let builtin = parse_presets(BUILTIN_RAW).unwrap();
        assert!(!builtin.presets.is_empty());
        assert!(builtin.presets.iter().all(|p| !p.name.trim().is_empty()));
    }

    #[test]
    fn validate_presets_rejects_empty_name() {
        let file = parse_presets(r#"{"presets":[{"name":"  "}]}"#).unwrap();
        assert!(validate_presets(&file.presets).is_err());
    }

    #[test]
    fn cache_is_used_only_from_same_app_version() {
        let current = format!(
            r#"{{"app_version":"{}","presets":[{{"name":"X"}}]}}"#,
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(current_cache_presets(&current).unwrap().len(), 1);

        let stale = r#"{"app_version":"0.0.0","presets":[{"name":"X"}]}"#;
        assert!(current_cache_presets(stale).is_none());

        assert!(current_cache_presets("not json").is_none());
    }

    #[test]
    fn same_presets_ignores_formatting_and_key_order() {
        let a = parse_presets(r#"{"presets":[{"name":"X","base_url":"u"}]}"#).unwrap();
        let b =
            parse_presets("{ \"presets\": [ { \"base_url\": \"u\", \"name\": \"X\" } ] }").unwrap();
        assert!(same_presets(&a.presets, &b.presets));

        let c = parse_presets(r#"{"presets":[{"name":"X"},{"name":"Y"}]}"#).unwrap();
        assert!(!same_presets(&a.presets, &c.presets));
    }

    #[test]
    fn visible_presets_filters_sandbox_only_unless_sandbox() {
        let file = parse_presets(
            r#"{"presets":[{"name":"Normal"},{"name":"Test","sandbox_only":true}]}"#,
        )
        .unwrap();
        let normal = visible_presets(&file.presets, false);
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].name, "Normal");
        assert_eq!(visible_presets(&file.presets, true).len(), 2);
    }

    #[test]
    fn builtin_sandbox_only_entry_points_at_mock_host() {
        let builtin = parse_presets(BUILTIN_RAW).unwrap();
        let test = builtin
            .presets
            .iter()
            .find(|p| p.sandbox_only)
            .expect("builtin file carries one sandbox-only test preset");
        assert_eq!(test.base_url, "https://sandbox-mock.local/v1");
    }
}
