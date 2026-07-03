//! HTTP proxy for LLM / web-search API requests.
//!
//! Routes provider HTTP calls through the Rust backend (via `reqwest`) instead
//! of the frontend `fetch`, so the WebView2 CORS restrictions that block
//! providers without CORS headers (e.g. Volcano Engine Ark) are bypassed
//! entirely. The frontend keeps its existing URL/header/body/parse logic —
//! only the transport layer changes.
//!
//! ## Security
//!
//! - **Scheme allow-list** (VULN-4): only `http``/`https` URLs are accepted.
//!   This blocks `file://`, `gopher://`, `data:`, etc. All IP ranges are
//!   permitted (including loopback / private) because local-model providers
//!   like LM Studio (`http://localhost:1234/v1`) and Ollama require them.
//! - **Redirect cap**: reqwest follows at most 3 redirects (down from the
//!   default 10) to limit redirect-based bypass of the scheme check.
//! - **Abort channel** (VULN-5): each request is tracked by a unique id; the
//!   frontend can cancel an in-flight request via `llm_http_abort`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Notify;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmHttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    /// Request timeout in milliseconds. Defaults to 60000 (LLM responses can
    /// be slow, especially for long translations).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Client-generated unique id, used to cancel this request via
    /// `llm_http_abort`. If omitted, the request cannot be aborted.
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LlmHttpResponse {
    /// HTTP status code (0 when the request never reached the server).
    pub status: u16,
    /// Raw response body text.
    pub body: String,
    /// True when status is in the 200–299 success range.
    pub ok: bool,
}

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Maximum redirects reqwest will follow. Capped below reqwest's default of 10
/// to reduce redirect-based SSRF / scheme-check-bypass surface.
const MAX_REDIRECTS: usize = 3;

/// Tracks in-flight requests so the frontend can abort them. The id is issued
/// server-side per request; the frontend receives it via the response and can
/// pass it to `llm_http_abort`.
#[derive(Default)]
pub struct InflightRegistry {
    /// request id → abort signal. The `llm_http` task watches `Notify` via
    /// `select!` and exits early when notified.
    aborts: std::sync::Mutex<HashMap<String, Arc<Notify>>>,
}

/// Validate the request URL: only http/https schemes are accepted. All hosts
/// (including localhost / private IPs) are allowed — local-model providers
/// need loopback, and `base_url` is user-controlled (not attacker-controlled
/// in the threat model of a desktop app).
fn validate_url(raw: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(raw).map_err(|e| format!("invalid url: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "unsupported scheme '{}' (only http/https allowed)",
            parsed.scheme()
        ));
    }
    Ok(parsed)
}

/// Issue an HTTP request on behalf of the frontend and return the raw response.
///
/// Network/transport failures (DNS, connection refused, TLS, timeout) surface
/// as `Err(String)` so the frontend can render a friendly message. HTTP error
/// responses (4xx/5xx) are *not* errors here — they come back as a normal
/// `LlmHttpResponse` with `ok: false`, letting the frontend reuse its existing
/// status-based error handling.
#[tauri::command]
pub async fn llm_http(
    req: LlmHttpRequest,
    registry: State<'_, InflightRegistry>,
) -> Result<LlmHttpResponse, String> {
    // VULN-4: reject non-http(s) schemes before anything touches the network.
    let _validated = validate_url(&req.url)?;

    let timeout = Duration::from_millis(req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()
        .map_err(|e| format!("build client: {e}"))?;

    let method = reqwest::Method::from_bytes(req.method.as_bytes())
        .map_err(|e| format!("invalid method '{}': {e}", req.method))?;

    let mut request = client.request(method, &req.url);
    for (k, v) in &req.headers {
        request = request.header(k, v);
    }
    if let Some(body) = req.body.as_deref().filter(|b| !b.is_empty()) {
        request = request.body(body.to_string());
    }

    // VULN-5: register an abort signal so the frontend can cancel this request.
    // The id is client-supplied so the caller already knows it and can pass it
    // to `llm_http_abort` without waiting for a response.
    let request_id = req.request_id.clone();
    let abort = Arc::new(Notify::new());
    if let Some(ref id) = request_id {
        let mut guard = registry.aborts.lock().unwrap();
        guard.insert(id.clone(), abort.clone());
    }

    // Run the request under select! with the abort signal so a cancel from
    // the frontend short-circuits the await.
    let send_future = request.send();
    let result = tokio::select! {
        biased;
        _ = abort.notified() => {
            // Clean up our entry; the task is ending.
            if let Some(ref id) = request_id {
                if let Some(mut guard) = registry.aborts.lock().ok() {
                    guard.remove(id);
                }
            }
            return Err("request aborted".to_string());
        }
        response = send_future => response,
    };

    // Remove the abort entry now that the request completed.
    if let Some(ref id) = request_id {
        if let Some(mut guard) = registry.aborts.lock().ok() {
            guard.remove(id);
        }
    }

    let response = result.map_err(|e| format!("request failed: {e}"))?;

    let status = response.status().as_u16();
    let ok = response.status().is_success();
    let body = response.text().await.map_err(|e| format!("read body: {e}"))?;

    Ok(LlmHttpResponse { status, body, ok })
}

/// Cancel an in-flight request by id. Returns `Ok(())` whether or not the id
/// existed — aborting an already-completed request is a no-op, not an error.
#[tauri::command]
pub fn llm_http_abort(request_id: String, registry: State<'_, InflightRegistry>) -> Result<(), String> {
    if let Some(notify) = registry.aborts.lock().unwrap().remove(&request_id) {
        notify.notify_one();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_url_accepts_http_and_https() {
        assert!(validate_url("https://api.openai.com/v1/chat").is_ok());
        assert!(validate_url("http://localhost:1234/v1/chat").is_ok());
        assert!(validate_url("http://192.168.1.10:8080").is_ok());
    }

    #[test]
    fn validate_url_rejects_non_http_schemes() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("gopher://localhost/x").is_err());
        assert!(validate_url("data:text/html,<script>").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn validate_url_rejects_malformed() {
        assert!(validate_url("not a url at all").is_err());
        assert!(validate_url("http://").is_err());
    }
}
