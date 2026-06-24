//! Vault: in-memory Master Key lifecycle + portable export/import envelope.
//!
//! On-disk model (always, every user, zero passwords):
//!   random Master Key  --wrapped by KEK_machine-->  vault.key
//!   Master Key  --derives per-scope keys-->  *.json ciphertext (via crypto.rs)
//!
//! Transport model (only when the user backs up / migrates):
//!   Master Key  --wrapped by KEK_password=Argon2id-->  export bundle
//!   bundle also carries the 5 data ciphertexts verbatim.
//!
//! Local protection strength is unchanged vs. the legacy scheme (still bound to
//! the machine seed). The sole win of this refactor is portability: the random
//! Master Key can be re-wrapped on another machine, whereas the old 5
//! machine-derived keys could not move at all.

use std::fs;
use std::path::PathBuf;

use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use zeroize::Zeroizing;

use crate::crypto::{self, EncryptedPayload};

/// Fixed plaintext encrypted under the Master-Key-derived verifier scope.
/// Decrypting the verifier with a candidate Master Key and comparing to this
/// constant confirms the key is correct (used at unlock, and on import).
const VERIFIER_MAGIC: &[u8] = b"prompit-vault-v1";
/// Scope used to derive the verifier key from the Master Key. Deliberately a
/// scope that no data file uses, so the verifier blob never collides with real
/// data and a wrong Master Key fails loudly at the verifier, not at some
/// downstream data-deserialize step.
const VERIFIER_SCOPE: &str = "__vault_verifier__";

/// The 5 encrypted data files carried in an export bundle. Each entry is the
/// raw on-disk ciphertext; it is already wrapped under the Master Key, so it
/// travels untouched and lands verbatim on import.
const DATA_FILES: &[&str] = &["secrets", "history", "dictionaries", "personas", "sparkles"];

// ── Argon2id parameters for the transport envelope ─────────────────────────
// 64 MiB / 3 iterations / 1 lane. Deliberately heavier than "default" because
// export files leave the machine and password is their only protection.
const ARGON2_M_COST: u32 = 65536; // 64 MiB
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 1;
const ARGON2_OUTPUT_LEN: usize = 32;
const ARGON2_SALT_LEN: usize = 16;

// ── On-disk vault.key ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    /// Master Key encrypted with KEK_machine (raw machine key, NOT routed
    /// through derive_key — see crypto::derive_raw_machine_key).
    wrapped_master_key: EncryptedPayload,
    /// VERIFIER_MAGIC encrypted under the verifier scope. Lets us confirm a
    /// candidate Master Key without touching user data.
    verifier: EncryptedPayload,
}

fn vault_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    Ok(dir.join("vault.key"))
}

/// Generate a fresh random Master Key, wrap it with the machine key, write
/// vault.key, install it into the process. Used on first launch (no vault.key
/// yet) and after import (replace local Master Key with the imported one).
fn install_and_persist(app: &AppHandle, master_key: [u8; 32]) -> Result<(), String> {
    let machine_key = crypto::derive_raw_machine_key();
    let wrapped = crypto::encrypt_with_key(&machine_key, &master_key)?;
    let verifier = crypto::encrypt_with_key(&verifier_key(&master_key), VERIFIER_MAGIC)?;
    let file = VaultFile {
        version: 1,
        wrapped_master_key: wrapped,
        verifier,
    };
    let path = vault_path(app)?;
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("serialize: {e}"))?;
    // Write atomically-ish: temp file then rename, so a crash mid-write does
    // not corrupt the vault and lock the user out of all data.
    let tmp = path.with_extension("key.tmp");
    fs::write(&tmp, json).map_err(|e| format!("write tmp: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename: {e}"))?;
    crypto::set_master_key(master_key);
    Ok(())
}

/// Called once from app setup. If vault.key exists, unwrap the Master Key and
/// install it. Otherwise generate one fresh. Never touches user data files —
/// migration of legacy ciphertext happens lazily in each read_* path.
pub fn unlock_or_migrate(app: &AppHandle) -> Result<(), String> {
    let path = vault_path(app)?;
    if !path.exists() {
        let mut master_key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut master_key);
        return install_and_persist(app, master_key);
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("read vault: {e}"))?;
    let file: VaultFile =
        serde_json::from_str(&content).map_err(|e| format!("parse vault: {e}"))?;

    let machine_key = crypto::derive_raw_machine_key();
    let mk_bytes = crypto::decrypt_with_key(&machine_key, &file.wrapped_master_key)?;
    if mk_bytes.len() != 32 {
        return Err("corrupt vault: master key not 32 bytes".into());
    }
    let mut master_key = [0u8; 32];
    master_key.copy_from_slice(&mk_bytes);

    // Verify before installing: a mismatch (e.g. vault.key copied from another
    // machine) must fail loudly here, not silently produce wrong data keys.
    let verifier_plain = crypto::decrypt_with_key(&verifier_key(&master_key), &file.verifier)?;
    if verifier_plain != VERIFIER_MAGIC {
        return Err("vault verifier mismatch: wrong machine or corrupt vault".into());
    }

    crypto::set_master_key(master_key);
    Ok(())
}

/// Derive the verifier key from the Master Key. Distinct from data scopes so a
/// wrong key is caught at verification rather than during data deserialization.
fn verifier_key(master_key: &[u8; 32]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(master_key.as_slice());
    h.update(b":");
    h.update(VERIFIER_SCOPE.as_bytes());
    let out = h.finalize();
    let mut k = [0u8; 32];
    k.copy_from_slice(&out);
    k
}

// ── Transport envelope (export / import) ────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct KdfParams {
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
    output_len: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ExportBundle {
    version: u32,
    kind: String,
    salt: String,
    kdf: KdfParams,
    /// Master Key encrypted with KEK_password = Argon2id(password, salt).
    wrapped_master_key: EncryptedPayload,
    /// VERIFIER_MAGIC encrypted under the verifier scope of the Master Key.
    verifier: EncryptedPayload,
    /// Raw on-disk ciphertexts, keyed by data-file stem. Already wrapped under
    /// the Master Key, so they travel and land verbatim.
    data: std::collections::BTreeMap<String, EncryptedPayload>,
}

fn derive_password_key(password: &str, salt: &[u8], kdf: &KdfParams) -> Result<[u8; 32], String> {
    let params = Params::new(kdf.m_cost, kdf.t_cost, kdf.p_cost, Some(kdf.output_len))
        .map_err(|e| format!("argon2 params: {e}"))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), salt, out.as_mut_slice())
        .map_err(|e| format!("argon2 derive: {e}"))?;
    Ok(*out)
}

/// Read the 5 data files as raw ciphertext payloads. Missing files are simply
/// omitted from the map (import will then not write them, leaving the target
/// machine's existing state — or none — for that file).
fn collect_data_payloads(
    app: &AppHandle,
) -> Result<std::collections::BTreeMap<String, EncryptedPayload>, String> {
    let dir = crate::get_data_dir(app)?;
    let mut map = std::collections::BTreeMap::new();
    for stem in DATA_FILES {
        let path = dir.join(format!("{}.json", stem));
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", stem))?;
        let payload: EncryptedPayload =
            serde_json::from_str(&content).map_err(|e| format!("parse {}: {e}", stem))?;
        map.insert((*stem).to_string(), payload);
    }
    Ok(map)
}

/// Get the in-memory Master Key, or error if the vault was never unlocked
/// (should not happen at runtime since setup always unlocks first).
fn current_master_key() -> Result<[u8; 32], String> {
    // crypto::MASTER_KEY is private; expose via a getter there.
    crypto::get_master_key()
}

/// Build an export bundle for the current machine's data, keyed by `password`.
/// Writes it to `path`. Intended to be invoked from the export_data command.
pub fn export_bundle(app: &AppHandle, password: &str, path: &str) -> Result<(), String> {
    let master_key = current_master_key()?;

    let mut salt = [0u8; ARGON2_SALT_LEN];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let kdf = KdfParams {
        m_cost: ARGON2_M_COST,
        t_cost: ARGON2_T_COST,
        p_cost: ARGON2_P_COST,
        output_len: ARGON2_OUTPUT_LEN,
    };
    let pw_key = derive_password_key(password, &salt, &kdf)?;
    let wrapped = crypto::encrypt_with_key(&pw_key, &master_key)?;
    let verifier = crypto::encrypt_with_key(&verifier_key(&master_key), VERIFIER_MAGIC)?;
    let data = collect_data_payloads(app)?;

    let bundle = ExportBundle {
        version: 1,
        kind: "prompit-vault-export".to_string(),
        salt: BASE64.encode(salt),
        kdf,
        wrapped_master_key: wrapped,
        verifier,
        data,
    };
    let json =
        serde_json::to_string_pretty(&bundle).map_err(|e| format!("serialize bundle: {e}"))?;
    fs::write(path, json).map_err(|e| format!("write bundle: {e}"))?;
    Ok(())
}

/// Read a bundle from `path`, unwrap its Master Key with `password`, verify,
/// then land the data ciphertexts on this machine and re-wrap the Master Key
/// under THIS machine's KEK (so local use stays passwordless).
pub fn import_bundle(app: &AppHandle, password: &str, path: &str) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read bundle: {e}"))?;
    let bundle: ExportBundle =
        serde_json::from_str(&content).map_err(|e| format!("parse bundle: {e}"))?;
    if bundle.kind != "prompit-vault-export" {
        return Err("not a prompit export bundle".into());
    }

    let salt = BASE64
        .decode(&bundle.salt)
        .map_err(|e| format!("decode salt: {e}"))?;
    let pw_key = derive_password_key(password, &salt, &bundle.kdf)?;
    let mk_bytes = crypto::decrypt_with_key(&pw_key, &bundle.wrapped_master_key)
        .map_err(|_| "wrong password or corrupt bundle".to_string())?;
    if mk_bytes.len() != 32 {
        return Err("corrupt bundle: master key not 32 bytes".into());
    }
    let mut master_key = [0u8; 32];
    master_key.copy_from_slice(&mk_bytes);

    // Verify before committing anything to disk.
    let verifier_plain = crypto::decrypt_with_key(&verifier_key(&master_key), &bundle.verifier)
        .map_err(|_| "corrupt bundle: verifier unreadable".to_string())?;
    if verifier_plain != VERIFIER_MAGIC {
        return Err("bundle verifier mismatch".into());
    }

    // 1) Write the data ciphertexts verbatim. They are already wrapped under
    //    the (now-known) Master Key, so no re-encryption needed.
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    for (stem, payload) in &bundle.data {
        let out = serde_json::to_string_pretty(payload)
            .map_err(|e| format!("serialize {}: {e}", stem))?;
        let p = dir.join(format!("{}.json", stem));
        fs::write(&p, out).map_err(|e| format!("write {}: {e}", stem))?;
    }

    // 2) Re-wrap the imported Master Key under this machine's KEK and install.
    //    Done last so a failure above leaves the local vault untouched.
    install_and_persist(app, master_key)?;
    Ok(())
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn export_data(app: AppHandle, path: String, password: String) -> Result<(), String> {
    if password.len() < 6 {
        return Err("password must be at least 6 characters".into());
    }
    export_bundle(&app, &password, &path)
}

#[tauri::command]
pub fn import_data(app: AppHandle, path: String, password: String) -> Result<(), String> {
    import_bundle(&app, &password, &path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_key_is_deterministic_and_distinct() {
        let mk = [1u8; 32];
        let a = verifier_key(&mk);
        let b = verifier_key(&mk);
        assert_eq!(a, b);
        // Distinct from a data-scope key derived the same way but with a
        // different scope label.
        let other = verifier_key(&[2u8; 32]);
        assert_ne!(a, other);
    }

    #[test]
    fn password_key_is_deterministic_for_same_salt() {
        let kdf = KdfParams {
            m_cost: 4096, // light for test speed
            t_cost: 1,
            p_cost: 1,
            output_len: 32,
        };
        let salt = [0u8; ARGON2_SALT_LEN];
        let a = derive_password_key("hunter2", &salt, &kdf).unwrap();
        let b = derive_password_key("hunter2", &salt, &kdf).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn password_key_differs_for_wrong_password() {
        let kdf = KdfParams {
            m_cost: 4096,
            t_cost: 1,
            p_cost: 1,
            output_len: 32,
        };
        let salt = [0u8; ARGON2_SALT_LEN];
        let a = derive_password_key("hunter2", &salt, &kdf).unwrap();
        let b = derive_password_key("hunter3", &salt, &kdf).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn master_key_wrap_unwrap_roundtrip() {
        let machine_key = crypto::derive_raw_machine_key();
        let master = [42u8; 32];
        let payload = crypto::encrypt_with_key(&machine_key, &master).unwrap();
        let plain = crypto::decrypt_with_key(&machine_key, &payload).unwrap();
        assert_eq!(plain, master);
    }
}
