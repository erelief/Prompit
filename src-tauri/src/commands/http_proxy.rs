//! HTTP proxy for LLM API requests.
//!
//! Routes provider HTTP calls through the Rust backend (via `reqwest`) instead
//! of the frontend `fetch`, so the WebView2 CORS restrictions that block
//! providers without CORS headers (e.g. Volcano Engine Ark) are bypassed
//! entirely. The frontend keeps its existing URL/header/body/parse logic —
//! only the transport layer changes.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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

/// Issue an HTTP request on behalf of the frontend and return the raw response.
///
/// Network/transport failures (DNS, connection refused, TLS, timeout) surface
/// as `Err(String)` so the frontend can render a friendly message. HTTP error
/// responses (4xx/5xx) are *not* errors here — they come back as a normal
/// `LlmHttpResponse` with `ok: false`, letting the frontend reuse its existing
/// status-based error handling.
#[tauri::command]
pub async fn llm_http(req: LlmHttpRequest) -> Result<LlmHttpResponse, String> {
    let timeout = Duration::from_millis(req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

    let client = reqwest::Client::builder()
        .timeout(timeout)
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

    let response = request.send().await.map_err(|e| format!("request failed: {e}"))?;

    let status = response.status().as_u16();
    let ok = response.status().is_success();
    let body = response.text().await.map_err(|e| format!("read body: {e}"))?;

    Ok(LlmHttpResponse { status, body, ok })
}
