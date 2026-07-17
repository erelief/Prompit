//! WebDAV as a storage location for the normal backup/restore flow.
//!
//! This module is NOT a sync engine: the backup file is the vault's ordinary
//! password-protected export bundle (see `vault.rs`), the WebDAV server is
//! just where it can be uploaded to / restored from instead of a local file.
//! The user enters the backup password per operation, exactly like the
//! file-based path, so no secret besides the WebDAV account password is ever
//! persisted (that one lives in the OS credential store, mirroring `kek.rs`).
//!
//! All HTTP stays in Rust via `reqwest` (same pattern as `http_proxy.rs`).
//! Only MKCOL / PUT / GET / PROPFIND are used. PROPFIND is parsed for the
//! backup file listing (Depth:1) via `quick-xml`, extracting `<href>` local
//! names only — tolerant of any namespace prefix the server picks.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::AppHandle;

use crate::commands::config_cmd;
use crate::vault;

const KEYRING_SERVICE: &str = "com.prompit.app";
const KEYRING_USER_SERVER: &str = "webdav-server-password";

/// Fallback backup file name when the configured one is empty. The upload
/// name is user-configured (in the WebDAV settings page) because there is no
/// OS save dialog on this path.
const DEFAULT_FILE_NAME: &str = "prompit-backup.json";

/// Per-request network timeout. Bundles are small (a few MB at most), so this
/// is generous; it mainly bounds unreachable-server waits.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Maximum redirects reqwest will follow (mirrors http_proxy's cap).
const MAX_REDIRECTS: usize = 3;

// ── Connection descriptor ───────────────────────────────────────────────────

/// Connection parameters for one WebDAV operation. Mirrors the `webdav`
/// section of `config.json`, plus the account password which travels per-call
/// (or is pulled from the keyring when omitted) instead of being persisted.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebdavConnection {
    pub url: String,
    #[serde(default)]
    pub username: String,
    /// Explicit account password; when `None`, the keyring-stored one is used.
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
}

fn default_remote_dir() -> String {
    "prompit".to_string()
}

/// Build a connection from the saved config; the account password is resolved
/// from the keyring at request time. Errors when WebDAV is not configured —
/// this is what gates the backup/restore-to-WebDAV features on the frontend.
fn saved_connection(app: &AppHandle) -> Result<WebdavConnection, String> {
    let cfg = config_cmd::read_config(app.clone())?;
    let wd = cfg.webdav;
    if wd.url.trim().is_empty() {
        return Err("WebDAV server is not configured".into());
    }
    Ok(WebdavConnection {
        url: wd.url,
        username: wd.username,
        password: None,
        remote_dir: wd.remote_dir,
    })
}

/// The configured upload file name (falling back to the default), validated.
fn saved_file_name(app: &AppHandle) -> Result<String, String> {
    let cfg = config_cmd::read_config(app.clone())?;
    let name = cfg.webdav.file_name.trim();
    let name = if name.is_empty() { DEFAULT_FILE_NAME } else { name };
    validate_file_name(name)?;
    Ok(name.to_string())
}

/// A backup file name must be a plain file name — it is appended to the
/// remote directory URL, so reject separators and traversal outright.
fn validate_file_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(format!("invalid backup file name: {name:?}"));
    }
    Ok(())
}

/// Resolve the account password: explicit value wins, keyring is the fallback.
fn resolve_password(conn: &WebdavConnection) -> Result<String, String> {
    match conn.password.as_deref().filter(|p| !p.is_empty()) {
        Some(pw) => Ok(pw.to_string()),
        None => keyring_get(KEYRING_USER_SERVER)
            .ok_or_else(|| "WebDAV password not saved; enter it and save the server config".into()),
    }
}

/// Validate `raw` as an http/https URL (mirrors http_proxy's scheme allow-list).
fn validated_url(raw: &str) -> Result<url::Url, String> {
    let parsed = url::Url::parse(raw).map_err(|e| format!("invalid WebDAV URL: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "unsupported scheme '{}' (only http/https allowed)",
            parsed.scheme()
        ));
    }
    Ok(parsed)
}

/// URL of the remote backup directory (`<base>/<remote_dir>`).
fn dir_url(conn: &WebdavConnection) -> Result<String, String> {
    let base = validated_url(conn.url.trim())?;
    let base = base.as_str().trim_end_matches('/');
    let dir = conn.remote_dir.trim_matches('/');
    Ok(if dir.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{dir}")
    })
}

/// URL of a file inside the remote backup directory.
fn file_url(conn: &WebdavConnection, name: &str) -> Result<String, String> {
    validate_file_name(name)?;
    Ok(format!("{}/{}", dir_url(conn)?, name))
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()
        .map_err(|e| format!("build client: {e}"))
}

/// Attach Basic auth when credentials are present; anonymous access is allowed
/// for servers that don't require it.
fn authed(
    req: reqwest::RequestBuilder,
    conn: &WebdavConnection,
    password: &str,
) -> reqwest::RequestBuilder {
    if conn.username.is_empty() && password.is_empty() {
        req
    } else {
        req.basic_auth(conn.username.clone(), Some(password.to_string()))
    }
}

// ── Keyring helpers ─────────────────────────────────────────────────────────

fn keyring_get(user: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, user).ok()?;
    entry.get_password().ok()
}

fn keyring_set(user: &str, value: &str) -> Result<(), String> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, user).map_err(|e| format!("keyring entry: {e}"))?;
    entry
        .set_password(value)
        .map_err(|e| format!("keyring store: {e}"))
}

fn keyring_delete(user: &str) -> Result<(), String> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, user).map_err(|e| format!("keyring entry: {e}"))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keyring delete: {e}")),
    }
}

// ── WebDAV protocol primitives ──────────────────────────────────────────────

/// Create the remote backup directory (and any missing parents), segment by
/// segment. Best-effort in the WebDAV sense: 201 = created, 405/2xx = already
/// there; anything else is an error.
async fn ensure_remote_dir(
    client: &reqwest::Client,
    conn: &WebdavConnection,
    password: &str,
) -> Result<(), String> {
    let dir = conn.remote_dir.trim_matches('/');
    if dir.is_empty() {
        return Ok(());
    }
    let base = validated_url(conn.url.trim())?;
    let mut current = base.as_str().trim_end_matches('/').to_string();
    let mkcol = reqwest::Method::from_bytes(b"MKCOL").expect("MKCOL is a valid method");
    for segment in dir.split('/') {
        current = format!("{current}/{segment}");
        let resp = authed(client.request(mkcol.clone(), &current), conn, password)
            .send()
            .await
            .map_err(|e| format!("create remote directory failed: {e}"))?;
        let status = resp.status();
        if status.is_success() || status.as_u16() == 405 {
            continue;
        }
        return Err(format!(
            "create remote directory failed: HTTP {status} (check permissions on {current})"
        ));
    }
    Ok(())
}

/// PUT the bundle bytes to `<dir>/<name>`, creating the directory first.
/// Returns the remote file URL.
async fn upload_bytes(conn: &WebdavConnection, name: &str, bytes: Vec<u8>) -> Result<String, String> {
    let password = resolve_password(conn)?;
    let client = http_client()?;
    ensure_remote_dir(&client, conn, &password).await?;
    let url = file_url(conn, name)?;
    let resp = authed(
        client
            .put(&url)
            .header("Content-Type", "application/json")
            .body(bytes),
        conn,
        &password,
    )
    .send()
    .await
    .map_err(|e| format!("upload failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(status_error("upload", status));
    }
    Ok(url)
}

/// GET a file's bytes from the remote backup directory.
async fn download_bytes(conn: &WebdavConnection, name: &str) -> Result<Vec<u8>, String> {
    let password = resolve_password(conn)?;
    let client = http_client()?;
    let url = file_url(conn, name)?;
    let resp = authed(client.get(&url), conn, &password)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 404 {
        return Err("backup file not found on the server".into());
    }
    if !status.is_success() {
        return Err(status_error("download", status));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("read response: {e}"))
}

/// Render an HTTP failure status as a readable error, calling out auth and
/// permission problems explicitly since those are the common misconfigurations.
fn status_error(op: &str, status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 => format!("{op} failed: authentication rejected (HTTP 401)"),
        403 => format!("{op} failed: permission denied (HTTP 403)"),
        507 => format!("{op} failed: insufficient storage on server (HTTP 507)"),
        _ => format!("{op} failed: HTTP {status}"),
    }
}

/// Extract backup file names from a PROPFIND Depth:1 multistatus body: last
/// path segment of every `<href>` that looks like a `.json` file (collections
/// end in `/` and are skipped). `<href>`s are matched by local name, so any
/// namespace prefix (D:, d:, lp1:, none) works. Percent-escapes are left
/// as-is — uploaded names are configured in-app and expected to be plain.
fn backup_names_from_multistatus(xml: &[u8]) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut names = Vec::new();
    let mut in_href = false;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                in_href = e.name().local_name().as_ref() == b"href";
            }
            Ok(Event::Text(t)) if in_href => {
                if let Ok(s) = t.unescape() {
                    let href = s.trim();
                    let last = href.rsplit('/').next().unwrap_or("");
                    if !last.is_empty() && last.ends_with(".json") {
                        names.push(last.to_string());
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().local_name().as_ref() == b"href" {
                    in_href = false;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    names.sort();
    names.dedup();
    names
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnResult {
    /// False when the server+auth check passed but the backup directory does
    /// not exist yet (it will be created on the first upload).
    pub dir_exists: bool,
}

/// Probe the server with PROPFIND Depth:0 on the backup directory. A 404 is
/// still a success (auth worked; the directory gets created on upload).
#[tauri::command]
pub async fn webdav_test_connection(conn: WebdavConnection) -> Result<TestConnResult, String> {
    if conn.url.trim().is_empty() {
        return Err("WebDAV server URL is empty".into());
    }
    let password = resolve_password(&conn)?;
    let client = http_client()?;
    let url = dir_url(&conn)?;
    let propfind = reqwest::Method::from_bytes(b"PROPFIND").expect("PROPFIND is a valid method");
    let resp = authed(
        client.request(propfind, &url).header("Depth", "0"),
        &conn,
        &password,
    )
    .send()
    .await
    .map_err(|e| format!("connection failed: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        Ok(TestConnResult { dir_exists: true })
    } else if status.as_u16() == 404 {
        Ok(TestConnResult { dir_exists: false })
    } else {
        Err(status_error("connection test", status))
    }
}

/// Persist the account password to the OS credential store. An empty string
/// deletes the entry.
#[tauri::command]
pub fn webdav_save_password(password: String) -> Result<(), String> {
    if password.is_empty() {
        keyring_delete(KEYRING_USER_SERVER)
    } else {
        keyring_set(KEYRING_USER_SERVER, &password)
    }
}

/// Whether the account password is present in the OS credential store (the UI
/// shows a "saved" hint without ever reading it back).
#[tauri::command]
pub fn webdav_has_password() -> bool {
    keyring_get(KEYRING_USER_SERVER).is_some()
}

/// List the backup files in the configured remote directory (PROPFIND
/// Depth:1). A missing directory yields an empty list, not an error.
#[tauri::command]
pub async fn webdav_list_files(app: AppHandle) -> Result<Vec<String>, String> {
    let conn = saved_connection(&app)?;
    let password = resolve_password(&conn)?;
    let client = http_client()?;
    let url = dir_url(&conn)?;
    let propfind = reqwest::Method::from_bytes(b"PROPFIND").expect("PROPFIND is a valid method");
    let resp = authed(
        client.request(propfind, &url).header("Depth", "1"),
        &conn,
        &password,
    )
    .send()
    .await
    .map_err(|e| format!("list files failed: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 404 {
        return Ok(vec![]);
    }
    if !status.is_success() {
        return Err(status_error("list files", status));
    }
    let body = resp
        .bytes()
        .await
        .map_err(|e| format!("read response: {e}"))?;
    Ok(backup_names_from_multistatus(&body))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    pub bytes: u64,
    pub remote: String,
}

/// Build an encrypted bundle of the selected categories and upload it to the
/// configured server + file name.
#[tauri::command]
pub async fn webdav_export(
    app: AppHandle,
    password: String,
    categories: Option<Vec<String>>,
) -> Result<UploadResult, String> {
    if password.len() < 6 {
        return Err("password must be at least 6 characters".into());
    }
    let conn = saved_connection(&app)?;
    let name = saved_file_name(&app)?;
    let app2 = app.clone();
    let cats = categories.unwrap_or_default();
    // Argon2 at export strength is CPU-heavy; keep it off the async runtime.
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        vault::build_bundle(&app2, &password, &cats)
    })
    .await
    .map_err(|e| format!("backup task: {e}"))??;
    let len = bytes.len() as u64;
    let remote = upload_bytes(&conn, &name, bytes).await?;
    Ok(UploadResult { bytes: len, remote })
}

/// Download a remote backup file and return its per-category preview (nothing
/// is written locally). Wrong backup password surfaces the vault's standard
/// error, same as the file-based inspect.
#[tauri::command]
pub async fn webdav_inspect_file(
    app: AppHandle,
    name: String,
    password: String,
) -> Result<vault::BundlePreview, String> {
    validate_file_name(&name)?;
    let conn = saved_connection(&app)?;
    let bytes = download_bytes(&conn, &name).await?;
    tauri::async_runtime::spawn_blocking(move || vault::inspect_bundle_bytes(&bytes, &password))
        .await
        .map_err(|e| format!("inspect task: {e}"))?
}

/// Download a remote backup file and restore the selected categories onto
/// this machine (re-wraps the Master Key under this machine's KEK, exactly
/// like the file-based restore).
#[tauri::command]
pub async fn webdav_restore_file(
    app: AppHandle,
    name: String,
    password: String,
    categories: Option<Vec<String>>,
) -> Result<(), String> {
    validate_file_name(&name)?;
    let conn = saved_connection(&app)?;
    let bytes = download_bytes(&conn, &name).await?;
    let cats = categories.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        vault::import_bundle_bytes(&app, &password, &bytes, &cats)
    })
    .await
    .map_err(|e| format!("restore task: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn(url: &str, dir: &str) -> WebdavConnection {
        WebdavConnection {
            url: url.to_string(),
            username: "u".to_string(),
            password: Some("p".to_string()),
            remote_dir: dir.to_string(),
        }
    }

    #[test]
    fn dir_url_joins_and_normalizes() {
        assert_eq!(
            dir_url(&conn("https://dav.example.com/files/u/", "/prompit/")).unwrap(),
            "https://dav.example.com/files/u/prompit"
        );
        assert_eq!(
            dir_url(&conn("https://dav.example.com", "a/b")).unwrap(),
            "https://dav.example.com/a/b"
        );
        // Empty remote dir falls back to the base URL itself.
        assert_eq!(
            dir_url(&conn("https://dav.example.com/root", "")).unwrap(),
            "https://dav.example.com/root"
        );
    }

    #[test]
    fn file_url_appends_name() {
        assert_eq!(
            file_url(&conn("https://dav.example.com", "prompit"), "a.json").unwrap(),
            "https://dav.example.com/prompit/a.json"
        );
    }

    #[test]
    fn urls_reject_bad_input() {
        assert!(dir_url(&conn("not a url", "x")).is_err());
        assert!(dir_url(&conn("ftp://dav.example.com", "x")).is_err());
        assert!(dir_url(&conn("", "x")).is_err());
    }

    #[test]
    fn file_names_are_validated() {
        assert!(validate_file_name("prompit-backup.json").is_ok());
        assert!(validate_file_name("").is_err());
        assert!(validate_file_name("a/b.json").is_err());
        assert!(validate_file_name("a\\b.json").is_err());
        assert!(validate_file_name("..\\cfg").is_err());
        assert!(file_url(&conn("https://dav.example.com", "d"), "../x").is_err());
    }

    #[test]
    fn status_errors_call_out_auth() {
        assert!(status_error("upload", reqwest::StatusCode::UNAUTHORIZED).contains("401"));
        assert!(status_error("upload", reqwest::StatusCode::FORBIDDEN).contains("403"));
        assert!(status_error("upload", reqwest::StatusCode::INTERNAL_SERVER_ERROR).contains("500"));
    }

    #[test]
    fn multistatus_parsing_extracts_json_file_names() {
        // Nextcloud-flavoured response: D: prefixes, absolute hrefs, the
        // collection itself, a subdirectory, and a non-json file.
        let xml = br#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:">
  <d:response><d:href>/dav/files/u/prompit/</d:href></d:response>
  <d:response><d:href>/dav/files/u/prompit/prompit-backup.json</d:href></d:response>
  <d:response><d:href>/dav/files/u/prompit/older.json</d:href></d:response>
  <d:response><d:href>/dav/files/u/prompit/notes.txt</d:href></d:response>
  <d:response><d:href>/dav/files/u/prompit/subdir/</d:href></d:response>
</d:multistatus>"#;
        assert_eq!(
            backup_names_from_multistatus(xml),
            vec!["older.json".to_string(), "prompit-backup.json".to_string()]
        );
    }

    #[test]
    fn multistatus_parsing_tolerates_any_prefix_and_duplicates() {
        let xml = br#"<multistatus xmlns="DAV:">
  <response><href>/x/a.json</href></response>
  <lp1:response xmlns:lp1="DAV:"><lp1:href>http://h/x/a.json</lp1:href></lp1:response>
</multistatus>"#;
        assert_eq!(backup_names_from_multistatus(xml), vec!["a.json".to_string()]);
        assert!(backup_names_from_multistatus(b"not xml at all").is_empty());
    }
}
