use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DictEntry {
    pub source: String,
    pub target: String,
}

#[derive(Serialize, Deserialize)]
struct EncryptedDict {
    ciphertext: String,
    nonce: String,
}

type DictStore = HashMap<String, Vec<DictEntry>>;

fn derive_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".into());
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".into());
    let app_id = "com.translator.realtime";
    let seed = format!("{}:{}:{}", hostname, username, app_id);
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn dict_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("dictionaries.json"))
}

fn load_dict_store(app: &AppHandle) -> Result<DictStore, String> {
    let path = dict_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
    let enc: EncryptedDict = serde_json::from_str(&content).map_err(|e| format!("parse: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce_bytes = BASE64
        .decode(&enc.nonce)
        .map_err(|e| format!("decode nonce: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = BASE64
        .decode(&enc.ciphertext)
        .map_err(|e| format!("decode ct: {e}"))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decrypt: {e}"))?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("deserialize: {e}"))
}

fn save_dict_store(app: &AppHandle, store: &DictStore) -> Result<(), String> {
    let path = dict_path(app)?;
    let json = serde_json::to_vec(store).map_err(|e| format!("serialize: {e}"))?;

    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use rand::RngCore;

    let key = derive_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, json.as_slice())
        .map_err(|e| format!("encrypt: {e}"))?;

    let enc = EncryptedDict {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    };
    let out = serde_json::to_string_pretty(&enc).map_err(|e| format!("serialize enc: {e}"))?;
    fs::write(&path, out).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn read_dictionary(app: AppHandle, target_lang: String) -> Result<Vec<DictEntry>, String> {
    let store = load_dict_store(&app)?;
    Ok(store.get(&target_lang).cloned().unwrap_or_default())
}

#[tauri::command]
pub fn save_dictionary(
    app: AppHandle,
    target_lang: String,
    entries: Vec<DictEntry>,
) -> Result<(), String> {
    let mut store = load_dict_store(&app)?;
    store.insert(target_lang, entries);
    save_dict_store(&app, &store)
}

#[tauri::command]
pub fn import_dictionary_csv(
    app: AppHandle,
    target_lang: String,
    file_path: String,
) -> Result<usize, String> {
    let raw = fs::read(&file_path).map_err(|e| format!("read file: {e}"))?;
    let data = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &raw[3..]
    } else {
        &raw
    };
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data);
    let mut new_entries: Vec<DictEntry> = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("csv parse: {e}"))?;
        if record.len() < 2 {
            continue;
        }
        let source = record[0].trim().to_string();
        let target = record[1].trim().to_string();
        if source.is_empty() || target.is_empty() {
            continue;
        }
        new_entries.push(DictEntry { source, target });
    }

    let mut store = load_dict_store(&app)?;
    let existing = store.entry(target_lang).or_default();
    let existing_sources: std::collections::HashSet<String> =
        existing.iter().map(|e| e.source.clone()).collect();
    for entry in new_entries {
        if !existing_sources.contains(&entry.source) {
            existing.push(entry);
        }
    }
    let count = existing.len();
    save_dict_store(&app, &store)?;
    Ok(count)
}

#[tauri::command]
pub fn export_dictionary_csv(app: AppHandle, file_path: String) -> Result<(), String> {
    let store = load_dict_store(&app)?;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(&file_path)
        .map_err(|e| format!("create csv: {e}"))?;
    wtr.write_record(["lang", "source", "target"])
        .map_err(|e| format!("write header: {e}"))?;
    let mut langs: Vec<&String> = store.keys().collect();
    langs.sort();
    for lang in langs {
        for entry in &store[lang] {
            wtr.serialize((lang.as_str(), entry.source.as_str(), entry.target.as_str()))
                .map_err(|e| format!("write csv: {e}"))?;
        }
    }
    wtr.flush().map_err(|e| format!("flush csv: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_entry_serialize_roundtrip() {
        let entry = DictEntry {
            source: "hello".into(),
            target: "你好".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: DictEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }

    #[test]
    fn test_csv_parse_basic() {
        let data = "source,target\nhello,你好\nworld,世界\n";
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data.as_bytes());
        let mut entries = Vec::new();
        for result in rdr.records() {
            let record = result.unwrap();
            entries.push(DictEntry {
                source: record[0].to_string(),
                target: record[1].to_string(),
            });
        }
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, "hello");
        assert_eq!(entries[0].target, "你好");
        assert_eq!(entries[1].source, "world");
        assert_eq!(entries[1].target, "世界");
    }

    #[test]
    fn test_csv_parse_with_bom() {
        let csv_content = "source,target\nhello,你好\n";
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(csv_content.as_bytes());
        let stripped = &data[3..];
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(stripped);
        let mut entries = Vec::new();
        for result in rdr.records() {
            let record = result.unwrap();
            entries.push(DictEntry {
                source: record[0].to_string(),
                target: record[1].to_string(),
            });
        }
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "hello");
    }

    #[test]
    fn test_csv_generate() {
        let entries = vec![
            DictEntry {
                source: "hello".into(),
                target: "你好".into(),
            },
            DictEntry {
                source: "world".into(),
                target: "世界".into(),
            },
        ];
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);
        wtr.write_record(["source", "target"]).unwrap();
        for entry in &entries {
            wtr.serialize((entry.source.as_str(), entry.target.as_str()))
                .unwrap();
        }
        wtr.flush().unwrap();
        let output = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        assert!(output.contains("source,target"));
        assert!(output.contains("hello,你好"));
        assert!(output.contains("world,世界"));
    }
}
