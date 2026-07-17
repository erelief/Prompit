//! Vault: in-memory Master Key lifecycle + portable export/import envelope.
//!
//! On-disk model (always, every user, zero passwords):
//!   random Master Key  --wrapped by KEK_machine-->  vault.key
//!   Master Key  --derives per-scope keys-->  *.json ciphertext (via crypto.rs)
//!
//! Local protection (KEK_machine) has two sources, recorded in `vault.key`'s
//! `version` field (see `kek.rs`):
//!   version 2 (preferred): random KEK held in the OS credential store
//!     (Windows Credential Manager / macOS Keychain / Linux Secret Service).
//!     OS-bound: a stolen `vault.key` is useless without the owning session.
//!   version 1 (fallback):  SHA256(hostname:username:app_id). Used when the OS
//!     store is unreachable (e.g. Linux without a Secret Service daemon). A v1
//!     file is silently upgraded to v2 the first time the OS store is usable.
//!
//! Transport model (only when the user backs up / migrates):
//!   Master Key  --wrapped by KEK_password=Argon2id-->  export bundle
//!   bundle also carries the 7 data ciphertexts verbatim, plus an encrypted
//!   snapshot of the (otherwise plaintext) `config.json` software settings.
//!
//! The local KEK layer (this file + kek.rs) is fully decoupled from the
//! password transport layer: export/import never touches the machine KEK, so
//! changing the KEK source does not affect cross-device backups. The random
//! Master Key remains portable — it is re-wrapped under each machine's own KEK
//! on import.

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
use crate::kek;

/// Fixed plaintext encrypted under the Master-Key-derived verifier scope.
/// Decrypting the verifier with a candidate Master Key and comparing to this
/// constant confirms the key is correct (used at unlock, and on import).
const VERIFIER_MAGIC: &[u8] = b"prompit-vault-v1";
/// Scope used to derive the verifier key from the Master Key. Deliberately a
/// scope that no data file uses, so the verifier blob never collides with real
/// data and a wrong Master Key fails loudly at the verifier, not at some
/// downstream data-deserialize step.
const VERIFIER_SCOPE: &str = "__vault_verifier__";

/// The encrypted data files carried in an export bundle. Each entry is the raw
/// on-disk ciphertext; it is already wrapped under the Master Key, so it
/// travels untouched and lands verbatim on import. `providers` and `websearch`
/// carry the AI-provider / web-search-engine configurations (with their api
/// keys and active indices) so a backup restores the full service
/// configuration, not just the user content.
const DATA_FILES: &[&str] = &[
    "history",
    "dictionaries",
    "personas",
    "skills_lite",
    "providers",
    "websearch",
];

/// Pseudo-stem used in the export bundle for the software-settings payload.
/// Unlike the entries in `DATA_FILES`, the on-disk source (`config.json`) is
/// plaintext; export encrypts it under the Master Key's `"settings"` scope and
/// import writes it back as plaintext. Kept distinct from `DATA_FILES` so the
/// raw-ciphertext fast path (which lands data files verbatim) is not confused
/// by the encrypt/decrypt step this category requires.
const SETTINGS_STEM: &str = "settings";

/// Pseudo-stem for the WebDAV server settings. These live inside the
/// plaintext `config.json` (`webdav` key), but are their own backup category
/// (default unselected): at export the key is split out of the settings
/// snapshot into its own entry, and at import each stem merges back into
/// `config.json` independently. The account password is never part of this —
/// it stays in the OS credential store.
const WEBDAV_STEM: &str = "webdav";

/// Every stem that may legally appear as a key in `ExportBundle.data`. Used as
/// the import/inspect allowlist (replaces the path-traversal guard that used to
/// consult `DATA_FILES` alone).
const KNOWN_STEMS: &[&str] = &[
    "history",
    "dictionaries",
    "personas",
    "skills_lite",
    "providers",
    "websearch",
    "settings",
    "webdav",
];

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
    /// Which KEK source wraps `wrapped_master_key`. 1 = legacy
    /// `SHA256(host:user:app)` (see `crypto::derive_raw_machine_key`); 2 =
    /// random KEK held in the OS credential store (see `kek.rs`).
    version: u32,
    /// Master Key encrypted with the version-appropriate local KEK (raw KEK,
    /// NOT routed through derive_key — wrapping the Master Key itself must not
    /// recurse through derive_key).
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

/// Coerce a byte slice into a 32-byte array, tagging the error with `what` so
/// the caller's context (vault vs bundle) shows up in the message.
fn to_array32(bytes: &[u8], what: &str) -> Result<[u8; 32], String> {
    if bytes.len() != 32 {
        return Err(format!("{what}: master key not 32 bytes"));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(bytes);
    Ok(arr)
}

/// Wrap `master_key` under the best-available local KEK, write `vault.key`,
/// and install the Master Key into the process. Used on first launch (no
/// vault.key yet), after import (replace local Master Key with the imported
/// one), and when silently upgrading a v1 file to v2.
///
/// The KEK source is chosen by `kek::kek_for_write`: OS credential store when
/// reachable, else the machine-seed hash. The chosen source's version is
/// recorded in the file so a later unlock picks the matching KEK.
fn install_and_persist(app: &AppHandle, master_key: [u8; 32]) -> Result<(), String> {
    let (kek, source) = kek::kek_for_write();
    let wrapped = crypto::encrypt_with_key(&kek, &master_key)?;
    let verifier = crypto::encrypt_with_key(&scope_key(&master_key, VERIFIER_SCOPE), VERIFIER_MAGIC)?;
    let file = VaultFile {
        version: source.version(),
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
///
/// `vault.key` `version` selects the KEK source used to unwrap: v1 → machine
/// seed, v2 → OS credential store. A v1 file is silently re-wrapped as v2 the
/// first time the OS store becomes available, so existing users are upgraded
/// automatically with no data re-encryption (the Master Key is unchanged; only
/// its local wrap layer changes).
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

    let source = match file.version {
        1 => kek::KekSource::EnvHash,
        2 => kek::KekSource::OsKeystore,
        v => return Err(format!("unsupported vault.key version: {v}")),
    };
    let kek = kek::kek_for_read(source)?;
    let mk_bytes = crypto::decrypt_with_key(&kek, &file.wrapped_master_key)?;
    let master_key = to_array32(&mk_bytes, "corrupt vault")?;

    // Verify before installing: a mismatch (e.g. vault.key copied from another
    // machine) must fail loudly here, not silently produce wrong data keys.
    let verifier_plain = crypto::decrypt_with_key(&scope_key(&master_key, VERIFIER_SCOPE), &file.verifier)?;
    if verifier_plain != VERIFIER_MAGIC {
        return Err("vault verifier mismatch: wrong machine or corrupt vault".into());
    }

    crypto::set_master_key(master_key);

    // Silent upgrade: a legacy v1 wrap (machine seed) is re-wrapped as v2
    // (OS credential store) the first time the OS store is usable. `kek_for_write`
    // picks the OS store when available, so this is a no-op on machines where it
    // isn't (the file is just re-written as v1). The Master Key itself never
    // changes, so all data ciphertexts stay valid.
    if source == kek::KekSource::EnvHash {
        // Best-effort: a failure here leaves the file at v1, which still unlocks
        // fine next launch. The Master Key is already installed in memory.
        let _ = install_and_persist(app, master_key);
    }

    Ok(())
}

/// Master-Key-derived per-scope key: `SHA256(master_key || ":" || scope)`.
/// This mirrors `crypto::derive_key`, which is private to that module and
/// requires the Master Key to be installed in-process. Several paths here
/// (verifier, settings, inspect) need a scope key WITHOUT the Master Key being
/// installed — e.g. inspecting a bundle on a fresh machine before import — so
/// the derivation is duplicated here taking the key as an explicit argument.
/// `VERIFIER_SCOPE` is deliberately a scope no data file uses, so a wrong
/// Master Key fails loudly at the verifier, not at some downstream data step.
fn scope_key(master_key: &[u8; 32], scope: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(master_key.as_slice());
    h.update(b":");
    h.update(scope.as_bytes());
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
    /// Ciphertexts keyed by data-file stem. The 7 user-data files share a
    /// single on-disk shape (a single `EncryptedPayload` wrapping the
    /// serialized structure) and travel verbatim. The `"settings"` entry is the
    /// otherwise-plaintext `config.json` (minus its `webdav` key) re-encrypted
    /// under the Master Key's `"settings"` scope at export time; the `"webdav"`
    /// entry carries that split-out key under its own scope. Both are decrypted
    /// back to plaintext and merged into `config.json` on import. All values
    /// are wrapped under the Master Key.
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

/// Read the 7 data files as raw ciphertext payloads, plus the plaintext
/// `config.json` re-encrypted under the `"settings"` scope. Missing files are
/// simply omitted from the map (import will then not write them, leaving the
/// target machine's existing state — or none — for that file). The 7 data files
/// share the single-`EncryptedPayload` on-disk shape; `config.json` is read as
/// plaintext and encrypted here so the bundle never carries plaintext settings.
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
    // config.json is the only plaintext source — encrypt it under the settings
    // scope so the bundle stays ciphertext end-to-end. Absent on a fresh
    // install. The `webdav` key is split out into its own category so users
    // can leave server settings out of a backup while keeping the rest.
    let cfg_path = dir.join("config.json");
    if cfg_path.exists() {
        let content = fs::read(&cfg_path).map_err(|e| format!("read config: {e}"))?;
        let (settings_part, webdav_part) = split_config_webdav(&content);
        let payload = crypto::encrypt(SETTINGS_STEM, &settings_part)?;
        map.insert(SETTINGS_STEM.to_string(), payload);
        if let Some(webdav) = webdav_part {
            let payload = crypto::encrypt(WEBDAV_STEM, &webdav)?;
            map.insert(WEBDAV_STEM.to_string(), payload);
        }
    }
    Ok(map)
}

/// Split a plaintext `config.json` into (settings-without-webdav, webdav).
/// An unparseable config is returned whole with no split (matches the old
/// encrypt-verbatim behavior). Pure helper, unit-tested below.
fn split_config_webdav(content: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    let mut value: serde_json::Value = match serde_json::from_slice(content) {
        Ok(v) => v,
        Err(_) => return (content.to_vec(), None),
    };
    let webdav = value.as_object_mut().and_then(|o| o.remove(WEBDAV_STEM));
    match webdav {
        Some(w) => match (serde_json::to_vec(&value), serde_json::to_vec(&w)) {
            (Ok(settings), Ok(webdav)) => (settings, Some(webdav)),
            _ => (content.to_vec(), None),
        },
        None => (content.to_vec(), None),
    }
}

/// Merge an imported settings payload into the local `config.json` while
/// preserving the LOCAL `webdav` key: the settings category no longer carries
/// it (it is its own stem), so a settings-only restore must not clobber the
/// WebDAV server config on this machine. Pure helper, unit-tested below.
fn merge_settings_preserving_webdav(imported: &[u8], existing: Option<&[u8]>) -> Vec<u8> {
    let mut imported_v: serde_json::Value = match serde_json::from_slice(imported) {
        Ok(v) => v,
        Err(_) => return imported.to_vec(),
    };
    let existing_webdav = existing
        .and_then(|e| serde_json::from_slice::<serde_json::Value>(e).ok())
        .and_then(|v| v.get(WEBDAV_STEM).cloned());
    if let (Some(obj), Some(w)) = (imported_v.as_object_mut(), existing_webdav) {
        obj.insert(WEBDAV_STEM.to_string(), w);
    }
    serde_json::to_vec_pretty(&imported_v).unwrap_or_else(|_| imported.to_vec())
}

/// Merge an imported `webdav` category payload into the local `config.json`
/// (creating a minimal config from `{}` when none exists). Pure helper.
fn merge_webdav_into_config(webdav: &[u8], existing: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let webdav_v: serde_json::Value =
        serde_json::from_slice(webdav).map_err(|e| format!("parse webdav payload: {e}"))?;
    let mut base: serde_json::Value = existing
        .and_then(|e| serde_json::from_slice(e).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    match base.as_object_mut() {
        Some(obj) => {
            obj.insert(WEBDAV_STEM.to_string(), webdav_v);
            serde_json::to_vec_pretty(&base).map_err(|e| format!("serialize config: {e}"))
        }
        None => Err("local config.json is not a JSON object".into()),
    }
}

/// Build an export bundle for the current machine's data, keyed by `password`,
/// returning the serialized bundle bytes. `categories` filters which entries
/// land in the bundle; pass an empty slice (or all of `KNOWN_STEMS`) to include
/// everything. This is the in-memory form of `export_bundle`, used when the
/// bundle is sent somewhere other than a local file (e.g. WebDAV upload).
pub fn build_bundle(
    app: &AppHandle,
    password: &str,
    categories: &[String],
) -> Result<Vec<u8>, String> {
    let master_key = crypto::get_master_key()?;

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
    let verifier = crypto::encrypt_with_key(&scope_key(&master_key, VERIFIER_SCOPE), VERIFIER_MAGIC)?;
    let mut data = collect_data_payloads(app)?;

    // Filter to the requested categories. Anything not in the allowlist or not
    // present on this machine is dropped. An empty request means "all known",
    // matching the pre-selective behavior.
    let want: std::collections::HashSet<&str> = if categories.is_empty() {
        KNOWN_STEMS.iter().copied().collect()
    } else {
        categories.iter().map(|s| s.as_str()).collect()
    };
    data.retain(|stem, _| want.contains(stem.as_str()) && KNOWN_STEMS.contains(&stem.as_str()));

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
    Ok(json.into_bytes())
}

/// Build an export bundle for the current machine's data, keyed by `password`.
/// Writes it to `path`. `categories` filters which entries land in the bundle;
/// pass an empty slice (or all of `KNOWN_STEMS`) to include everything.
/// Intended to be invoked from the export_data command.
pub fn export_bundle(
    app: &AppHandle,
    password: &str,
    path: &str,
    categories: &[String],
) -> Result<(), String> {
    let bytes = build_bundle(app, password, categories)?;
    fs::write(path, bytes).map_err(|e| format!("write bundle: {e}"))?;
    Ok(())
}

/// Read a bundle from `path`, unwrap its Master Key with `password`, verify,
/// then land the data ciphertexts on this machine and re-wrap the Master Key
/// under THIS machine's KEK (so local use stays passwordless). `categories`
/// selects which bundle entries are written; an empty slice means all known
/// stems present in the bundle.
pub fn import_bundle(
    app: &AppHandle,
    password: &str,
    path: &str,
    categories: &[String],
) -> Result<(), String> {
    let (bundle, master_key) = open_bundle(path, password)?;
    land_bundle(app, bundle, master_key, categories)
}

/// In-memory form of `import_bundle`: unwrap a bundle held in `bytes` (e.g.
/// downloaded from WebDAV) and land its data on this machine. Identical
/// semantics to `import_bundle`, including the final Master Key re-wrap.
pub fn import_bundle_bytes(
    app: &AppHandle,
    password: &str,
    bytes: &[u8],
    categories: &[String],
) -> Result<(), String> {
    let (bundle, master_key) = open_bundle_bytes(bytes, password)?;
    land_bundle(app, bundle, master_key, categories)
}

/// Land an already-opened bundle's data ciphertexts on this machine and
/// re-wrap the Master Key under THIS machine's KEK (so local use stays
/// passwordless). `categories` selects which bundle entries are written; an
/// empty slice means all known stems present in the bundle. Shared by the
/// file-based and bytes-based import paths.
fn land_bundle(
    app: &AppHandle,
    bundle: ExportBundle,
    master_key: [u8; 32],
    categories: &[String],
) -> Result<(), String> {
    let want: std::collections::HashSet<&str> = if categories.is_empty() {
        KNOWN_STEMS.iter().copied().collect()
    } else {
        categories.iter().map(|s| s.as_str()).collect()
    };

    // 1) Write each requested, allowlisted entry. The 7 data files are raw
    //    ciphertexts and land verbatim (already wrapped under the Master Key).
    //    The "settings"/"webdav" entries are Master-Key-encrypted blobs of the
    //    otherwise-plaintext config.json — decrypt them back to plaintext and
    //    merge: settings keeps the LOCAL webdav key, the webdav stem only
    //    replaces that one key.
    let dir = crate::get_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| format!("create dir: {e}"))?;
    let cfg_path = dir.join("config.json");
    for (stem, payload) in &bundle.data {
        if !KNOWN_STEMS.contains(&stem.as_str()) || !want.contains(stem.as_str()) {
            continue;
        }
        if stem == SETTINGS_STEM || stem == WEBDAV_STEM {
            let plain = crypto::decrypt_with_key(&scope_key(&master_key, stem), payload)
                .map_err(|e| format!("decrypt {}: {e}", stem))?;
            let existing = fs::read(&cfg_path).ok();
            let merged = if stem == SETTINGS_STEM {
                merge_settings_preserving_webdav(&plain, existing.as_deref())
            } else {
                merge_webdav_into_config(&plain, existing.as_deref())?
            };
            fs::write(&cfg_path, merged).map_err(|e| format!("write config: {e}"))?;
        } else {
            let out = serde_json::to_string_pretty(payload)
                .map_err(|e| format!("serialize {}: {e}", stem))?;
            let p = dir.join(format!("{}.json", stem));
            fs::write(&p, out).map_err(|e| format!("write {}: {e}", stem))?;
        }
    }

    // 2) Re-wrap the imported Master Key under this machine's KEK and install.
    //    Done last so a failure above leaves the local vault untouched.
    install_and_persist(app, master_key)?;
    Ok(())
}

/// Read-only inspection of a bundle: returns a per-category summary (which
/// stems are present and a rough count for the list-shaped ones) without
/// writing anything to disk. Used by the import UI to let the user pick what to
/// restore. Wrong password yields the same error string as `import_bundle`.
#[derive(Serialize)]
pub struct CategoryPreview {
    pub id: String,
    /// `Some(n)` for the list-shaped categories (providers/websearch count),
    /// `None` for opaque/mapping categories where a count is not meaningful.
    pub count: Option<usize>,
}

#[derive(Serialize)]
pub struct BundlePreview {
    pub version: u32,
    pub categories: Vec<CategoryPreview>,
}

pub fn inspect_bundle_file(
    path: &str,
    password: &str,
) -> Result<BundlePreview, String> {
    let (bundle, master_key) = open_bundle(path, password)?;
    summarize_bundle(&bundle, &master_key)
}

/// In-memory form of `inspect_bundle_file`: inspect a bundle held in `bytes`
/// (e.g. downloaded from WebDAV) without writing anything to disk. Wrong
/// password yields the same error string as `import_bundle_bytes`.
pub fn inspect_bundle_bytes(
    bytes: &[u8],
    password: &str,
) -> Result<BundlePreview, String> {
    let (bundle, master_key) = open_bundle_bytes(bytes, password)?;
    summarize_bundle(&bundle, &master_key)
}

/// Build the per-category preview for an already-opened bundle. Shared by the
/// file-based and bytes-based inspect paths.
fn summarize_bundle(
    bundle: &ExportBundle,
    master_key: &[u8; 32],
) -> Result<BundlePreview, String> {
    let mut cats = Vec::new();
    for stem in KNOWN_STEMS {
        if let Some(payload) = bundle.data.get(*stem) {
            let count = count_for(stem, payload, master_key)?;
            cats.push(CategoryPreview {
                id: (*stem).to_string(),
                count,
            });
        }
    }
    Ok(BundlePreview {
        version: bundle.version,
        categories: cats,
    })
}

/// Open + password-verify a bundle, returning the parsed bundle and the
/// recovered Master Key. Shared by import and inspect so both reject bad
/// passwords identically.
fn open_bundle(path: &str, password: &str) -> Result<(ExportBundle, [u8; 32]), String> {
    let bytes = fs::read(path).map_err(|e| format!("read bundle: {e}"))?;
    open_bundle_bytes(&bytes, password)
}

/// In-memory form of `open_bundle`: parse + password-verify a bundle held in
/// `bytes`. Shared by the file-based and WebDAV download paths so both reject
/// bad passwords identically.
fn open_bundle_bytes(bytes: &[u8], password: &str) -> Result<(ExportBundle, [u8; 32]), String> {
    let bundle: ExportBundle =
        serde_json::from_slice(bytes).map_err(|e| format!("parse bundle: {e}"))?;
    if bundle.kind != "prompit-vault-export" {
        return Err("not a prompit export bundle".into());
    }
    let salt = BASE64
        .decode(&bundle.salt)
        .map_err(|e| format!("decode salt: {e}"))?;
    let pw_key = derive_password_key(password, &salt, &bundle.kdf)?;
    let mk_bytes = crypto::decrypt_with_key(&pw_key, &bundle.wrapped_master_key)
        .map_err(|_| "wrong password or corrupt bundle".to_string())?;
    let master_key = to_array32(&mk_bytes, "corrupt bundle")?;
    let verifier_plain = crypto::decrypt_with_key(&scope_key(&master_key, VERIFIER_SCOPE), &bundle.verifier)
        .map_err(|_| "corrupt bundle: verifier unreadable".to_string())?;
    if verifier_plain != VERIFIER_MAGIC {
        return Err("bundle verifier mismatch".into());
    }
    Ok((bundle, master_key))
}

/// Decrypt the named stem's payload and report a count for the categories
/// whose shape is a list. `providers`/`websearch` carry a `Vec`-shaped bundle;
/// the other categories are mappings or arbitrary JSON and report `None`.
fn count_for(
    stem: &str,
    payload: &EncryptedPayload,
    master_key: &[u8; 32],
) -> Result<Option<usize>, String> {
    match stem {
        "providers" => {
            let plain = crypto::decrypt_with_key(&scope_key(master_key, "providers"), payload)?;
            let v: serde_json::Value =
                serde_json::from_slice(&plain).map_err(|e| format!("parse providers: {e}"))?;
            Ok(v.get("providers").and_then(|p| p.as_array()).map(|a| a.len()))
        }
        "websearch" => {
            let plain = crypto::decrypt_with_key(&scope_key(master_key, "websearch"), payload)?;
            let v: serde_json::Value =
                serde_json::from_slice(&plain).map_err(|e| format!("parse websearch: {e}"))?;
            Ok(v
                .get("web_search_providers")
                .and_then(|p| p.as_array())
                .map(|a| a.len()))
        }
        _ => Ok(None),
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn export_data(
    app: AppHandle,
    path: String,
    password: String,
    categories: Option<Vec<String>>,
) -> Result<(), String> {
    if password.len() < 6 {
        return Err("password must be at least 6 characters".into());
    }
    export_bundle(&app, &password, &path, &categories.unwrap_or_default())
}

#[tauri::command]
pub fn import_data(
    app: AppHandle,
    path: String,
    password: String,
    categories: Option<Vec<String>>,
) -> Result<(), String> {
    import_bundle(&app, &password, &path, &categories.unwrap_or_default())
}

#[tauri::command]
pub fn inspect_bundle(path: String, password: String) -> Result<BundlePreview, String> {
    inspect_bundle_file(&path, &password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_key_is_deterministic_and_distinct() {
        let mk = [1u8; 32];
        // Same (master_key, scope) → same key.
        assert_eq!(scope_key(&mk, VERIFIER_SCOPE), scope_key(&mk, VERIFIER_SCOPE));
        // Different master key → different key.
        assert_ne!(scope_key(&mk, VERIFIER_SCOPE), scope_key(&[2u8; 32], VERIFIER_SCOPE));
        // Different scope → different key. This is what keeps the verifier
        // (a scope no data file uses) from colliding with a data-scope key.
        assert_ne!(scope_key(&mk, VERIFIER_SCOPE), scope_key(&mk, SETTINGS_STEM));
        assert_ne!(scope_key(&mk, "providers"), scope_key(&mk, "websearch"));
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

    /// The local KEK layer must be decoupled from the password transport layer.
    /// This test proves a v1-wrap VaultFile round-trips through the same KEK
    /// source that wrote it (env-hash), independent of the export password path.
    #[test]
    fn vaultfile_v1_roundtrips_through_env_hash_kek() {
        let master = [0x11u8; 32];
        let (kek, source) = kek::kek_for_write();
        // Force the env-hash source for a deterministic test: the machine-seed
        // KEK is always available, so re-derive it directly.
        let env_kek = crypto::derive_raw_machine_key();
        let wrapped = crypto::encrypt_with_key(&env_kek, &master).unwrap();
        let verifier = crypto::encrypt_with_key(&scope_key(&master, VERIFIER_SCOPE), VERIFIER_MAGIC).unwrap();
        let file = VaultFile {
            version: kek::KekSource::EnvHash.version(),
            wrapped_master_key: wrapped,
            verifier,
        };

        // Serialize/deserialize (the on-disk step).
        let json = serde_json::to_string(&file).unwrap();
        let back: VaultFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.version, 1);

        // Read-back KEK selection mirrors unlock_or_migrate's logic.
        let read_source = match back.version {
            1 => kek::KekSource::EnvHash,
            2 => kek::KekSource::OsKeystore,
            _ => unreachable!(),
        };
        let read_kek = kek::kek_for_read(read_source).unwrap();
        // When the chosen write source was env-hash, the read KEK matches.
        if source == kek::KekSource::EnvHash {
            assert_eq!(read_kek, env_kek);
        }
        let recovered = crypto::decrypt_with_key(&read_kek, &back.wrapped_master_key).unwrap();
        assert_eq!(recovered, master);
        // Verifier must still validate under the recovered Master Key.
        let v = crypto::decrypt_with_key(&scope_key(&master, VERIFIER_SCOPE), &back.verifier).unwrap();
        assert_eq!(v, VERIFIER_MAGIC);
        // kek is used in the write branch above; reference it to avoid an
        // unused-variable lint on platforms where the compiler can't see it.
        let _ = kek;
    }

    /// Regression guard: the export/import envelope (password path) must be
    /// completely independent of the local KEK source. Build a bundle with a
    /// fixed Master Key, then unwrap it — the local KEK module is never invoked
    /// on the unwrap side.
    #[test]
    fn export_bundle_unwrap_is_independent_of_local_kek() {
        let master = [0x77u8; 32];
        let kdf = KdfParams {
            m_cost: 4096,
            t_cost: 1,
            p_cost: 1,
            output_len: ARGON2_OUTPUT_LEN,
        };
        let salt = [0u8; ARGON2_SALT_LEN];
        let pw_key = derive_password_key("correct horse", &salt, &kdf).unwrap();

        // Wrap as export does.
        let wrapped = crypto::encrypt_with_key(&pw_key, &master).unwrap();
        let verifier = crypto::encrypt_with_key(&scope_key(&master, VERIFIER_SCOPE), VERIFIER_MAGIC).unwrap();

        // Unwrap as import does — note: NO call to kek:: anything.
        let mk_bytes = crypto::decrypt_with_key(&pw_key, &wrapped).unwrap();
        assert_eq!(mk_bytes, master);
        let v = crypto::decrypt_with_key(&scope_key(&master, VERIFIER_SCOPE), &verifier).unwrap();
        assert_eq!(v, VERIFIER_MAGIC);

        // A wrong password must fail, proving the wrap is genuinely keyed by
        // the password (not by any local machine state).
        let wrong = derive_password_key("wrong horse", &salt, &kdf).unwrap();
        assert!(crypto::decrypt_with_key(&wrong, &wrapped).is_err());
    }

    /// The in-memory bundle path (WebDAV download) must parse, verify, and
    /// inspect exactly like the file path, and reject bad input identically.
    #[test]
    fn bundle_bytes_open_and_inspect_roundtrip() {
        let master = [0x42u8; 32];
        let kdf = KdfParams {
            m_cost: 4096, // light for test speed
            t_cost: 1,
            p_cost: 1,
            output_len: ARGON2_OUTPUT_LEN,
        };
        let salt = [7u8; ARGON2_SALT_LEN];
        let pw_key = derive_password_key("correct horse", &salt, &kdf).unwrap();
        let wrapped = crypto::encrypt_with_key(&pw_key, &master).unwrap();
        let verifier =
            crypto::encrypt_with_key(&scope_key(&master, VERIFIER_SCOPE), VERIFIER_MAGIC).unwrap();

        // One data entry: a 2-item providers list under the master-key scope.
        let providers_json = serde_json::json!({ "providers": [{ "a": 1 }, { "b": 2 }] });
        let providers_payload = crypto::encrypt_with_key(
            &scope_key(&master, "providers"),
            serde_json::to_string(&providers_json).unwrap().as_bytes(),
        )
        .unwrap();
        let mut data = std::collections::BTreeMap::new();
        data.insert("providers".to_string(), providers_payload);

        let bundle = ExportBundle {
            version: 1,
            kind: "prompit-vault-export".to_string(),
            salt: BASE64.encode(salt),
            kdf,
            wrapped_master_key: wrapped,
            verifier,
            data,
        };
        let bytes = serde_json::to_vec(&bundle).unwrap();

        // Right password: opens, recovers the Master Key, inspects.
        let (opened, mk) = open_bundle_bytes(&bytes, "correct horse").unwrap();
        assert_eq!(mk, master);
        assert!(opened.data.contains_key("providers"));
        let preview = inspect_bundle_bytes(&bytes, "correct horse").unwrap();
        assert_eq!(preview.version, 1);
        assert_eq!(preview.categories.len(), 1);
        assert_eq!(preview.categories[0].id, "providers");
        assert_eq!(preview.categories[0].count, Some(2));

        // Wrong password fails with the same message as the file path.
        let err = open_bundle_bytes(&bytes, "wrong horse")
            .err()
            .expect("wrong password must fail");
        assert!(err.contains("wrong password"));
        assert!(inspect_bundle_bytes(&bytes, "wrong horse").is_err());

        // Garbage bytes are a clean parse error, not a panic.
        assert!(open_bundle_bytes(b"not json", "correct horse").is_err());
    }

    #[test]
    fn split_config_webdav_separates_the_key() {
        let cfg = br#"{"theme":"dark","webdav":{"url":"https://dav","username":"u"}}"#;
        let (settings, webdav) = split_config_webdav(cfg);
        let s: serde_json::Value = serde_json::from_slice(&settings).unwrap();
        assert_eq!(s.get("theme").unwrap(), "dark");
        assert!(s.get("webdav").is_none());
        let w: serde_json::Value = serde_json::from_slice(&webdav.unwrap()).unwrap();
        assert_eq!(w.get("url").unwrap(), "https://dav");
    }

    #[test]
    fn split_config_webdav_passes_through_when_absent_or_unparseable() {
        let cfg = br#"{"theme":"dark"}"#;
        let (settings, webdav) = split_config_webdav(cfg);
        assert_eq!(settings, cfg);
        assert!(webdav.is_none());
        let (raw, none2) = split_config_webdav(b"not json");
        assert_eq!(raw, b"not json");
        assert!(none2.is_none());
    }

    #[test]
    fn merge_settings_preserves_local_webdav() {
        let imported = br#"{"theme":"light"}"#;
        let existing = br#"{"theme":"dark","webdav":{"url":"https://local"}}"#;
        let merged = merge_settings_preserving_webdav(imported, Some(existing));
        let v: serde_json::Value = serde_json::from_slice(&merged).unwrap();
        assert_eq!(v.get("theme").unwrap(), "light");
        assert_eq!(v["webdav"]["url"], "https://local");
        // No local config yet → imported lands as-is.
        let merged = merge_settings_preserving_webdav(imported, None);
        let v: serde_json::Value = serde_json::from_slice(&merged).unwrap();
        assert!(v.get("webdav").is_none());
    }

    #[test]
    fn merge_webdav_replaces_only_that_key() {
        let webdav = br#"{"url":"https://imported"}"#;
        let existing = br#"{"theme":"dark","webdav":{"url":"https://local"}}"#;
        let merged = merge_webdav_into_config(webdav, Some(existing)).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&merged).unwrap();
        assert_eq!(v.get("theme").unwrap(), "dark");
        assert_eq!(v["webdav"]["url"], "https://imported");
        // No local config → a minimal one is created around the webdav key.
        let merged = merge_webdav_into_config(webdav, None).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&merged).unwrap();
        assert_eq!(v["webdav"]["url"], "https://imported");
        // Unparseable payload is an error, not a silent clobber.
        assert!(merge_webdav_into_config(b"not json", None).is_err());
    }
}
