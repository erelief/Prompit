use std::fs;
use std::path::PathBuf;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Hard cap on a single file read into memory (open-dialog / drag-drop intake).
const MAX_READ_BYTES: u64 = 20 * 1024 * 1024;

/// Attachment metadata persisted on a HistoryEntry. Payload bytes live in a
/// file under `<data_dir>/history_attachments/`; `path` is relative to that
/// directory (`<entry_timestamp>/<index>_<name>`), so the encrypted
/// history.json stays small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryAttachment {
    pub name: String,
    pub mime: String,
    pub size: u64,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct AttachmentFileData {
    pub name: String,
    pub size: u64,
    pub data_base64: String,
}

#[derive(Debug, Deserialize)]
pub struct NewAttachment {
    pub name: String,
    pub mime: String,
    pub data_base64: String,
}

pub fn attachments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::get_data_dir(app)?.join("history_attachments"))
}

/// Read an arbitrary user-picked file (open dialog / native drag-drop) and
/// return its bytes as base64. Type classification (image vs text vs
/// unsupported) lives in the frontend — this is a dumb byte pipe so the type
/// rules have a single source of truth.
#[tauri::command]
pub fn read_attachment_file(path: String) -> Result<AttachmentFileData, String> {
    let p = PathBuf::from(&path);
    let meta = fs::metadata(&p).map_err(|e| format!("stat: {e}"))?;
    if meta.len() > MAX_READ_BYTES {
        return Err(format!("file too large ({} bytes)", meta.len()));
    }
    let bytes = fs::read(&p).map_err(|e| format!("read: {e}"))?;
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    Ok(AttachmentFileData {
        name,
        size: bytes.len() as u64,
        data_base64: B64.encode(bytes),
    })
}

/// Replace characters that are unsafe in file names across platforms.
fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Persist the composing attachments for one history entry: decode base64 and
/// write each file under `history_attachments/<entry_ts>/`, returning the
/// metadata to embed in the entry. The frontend calls this right before
/// `save_history` with the entry's timestamp.
#[tauri::command]
pub fn save_history_attachments(
    app: AppHandle,
    entry_ts: u64,
    files: Vec<NewAttachment>,
) -> Result<Vec<HistoryAttachment>, String> {
    let dir = attachments_dir(&app)?.join(entry_ts.to_string());
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;

    let mut out = Vec::with_capacity(files.len());
    for (i, f) in files.iter().enumerate() {
        let bytes = B64
            .decode(&f.data_base64)
            .map_err(|e| format!("base64: {e}"))?;
        let file_name = format!("{i}_{}", sanitize_file_name(&f.name));
        fs::write(dir.join(&file_name), &bytes).map_err(|e| format!("write: {e}"))?;
        out.push(HistoryAttachment {
            name: f.name.clone(),
            mime: f.mime.clone(),
            size: bytes.len() as u64,
            path: format!("{entry_ts}/{file_name}"),
        });
    }
    Ok(out)
}

/// Resolve a stored relative attachment path, guarding against traversal.
fn resolve_attachment_path(app: &AppHandle, rel: &str) -> Result<PathBuf, String> {
    if rel.contains("..") || rel.starts_with('/') || rel.starts_with('\\') {
        return Err("invalid attachment path".to_string());
    }
    Ok(attachments_dir(app)?.join(rel))
}

/// Copy a persisted attachment to a user-chosen destination (history panel's
/// download button). `path` must stay inside the attachments dir.
#[tauri::command]
pub fn export_history_attachment(
    app: AppHandle,
    path: String,
    dest: String,
) -> Result<(), String> {
    let src = resolve_attachment_path(&app, &path)?;
    fs::copy(&src, &dest).map_err(|e| format!("copy: {e}"))?;
    Ok(())
}

/// Read a persisted attachment back as base64 (hover thumbnails in the
/// history panel; restoring attachments into the composer).
#[tauri::command]
pub fn read_history_attachment(app: AppHandle, path: String) -> Result<AttachmentFileData, String> {
    let p = resolve_attachment_path(&app, &path)?;
    let bytes = fs::read(&p).map_err(|e| format!("read: {e}"))?;
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    Ok(AttachmentFileData {
        name,
        size: bytes.len() as u64,
        data_base64: B64.encode(bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_file_name_strips_unsafe_chars() {
        assert_eq!(sanitize_file_name("a/b\\c:d.txt"), "a_b_c_d.txt");
        assert_eq!(sanitize_file_name("normal.png"), "normal.png");
        assert_eq!(sanitize_file_name("中文 文件.md"), "中文 文件.md");
    }

    #[test]
    fn test_sanitize_file_name_fallbacks() {
        assert_eq!(sanitize_file_name(""), "file");
        assert_eq!(sanitize_file_name("..."), "file");
        assert_eq!(sanitize_file_name("  "), "file");
    }
}
