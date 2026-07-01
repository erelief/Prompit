//! Local KEK (key-encrypting-key) for the on-disk Master Key wrap.
//!
//! The vault wraps its random Master Key under a *local* KEK before writing
//! `vault.key`. This module owns the KEK *source*. There are two:
//!
//! 1. **OS credential store** (preferred) — a random 32-byte KEK stored via the
//!    platform's secure store: Windows Credential Manager (DPAPI-protected),
//!    macOS Keychain, or the Linux Secret Service (GNOME Keyring / KWallet).
//!    This is hardware/OS-bound: a stolen `vault.key` is useless without the
//!    OS session that owns the credential entry.
//! 2. **Machine-seed hash** (fallback) — `SHA256(hostname:username:app_id)`.
//!    Used only when the OS store is unreachable (e.g. a Linux box with no
//!    Secret Service daemon). This is the legacy scheme; it is obfuscation,
//!    not true OS binding, but it is strictly better than no local wrap and it
//!    keeps the app fully functional on every platform.
//!
//! Which source was used is recorded in `vault.key`'s `version` field, so an
//! unlock reads back the right KEK and an upgrade from v1→v2 can happen
//! silently the first time the OS store becomes available.
//!
//! IMPORTANT: this module touches ONLY the local wrap layer. The
//! password-protected export/import envelope (Argon2id) is entirely separate
//! and never calls into here — see `vault.rs`.

use rand::RngCore;

use crate::crypto;

/// keyring entry identifiers. Service + user together name the credential.
const KEYRING_SERVICE: &str = "com.prompit.app";
const KEYRING_USER: &str = "vault-kek";

/// Which backing store produced a KEK. Mirrors `vault.key`'s `version`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KekSource {
    /// `vault.key` version 1: legacy `SHA256(host:user:app)` wrap.
    EnvHash,
    /// `vault.key` version 2: random KEK held in the OS credential store.
    OsKeystore,
}

impl KekSource {
    /// The `vault.key` version number this source corresponds to.
    pub fn version(self) -> u32 {
        match self {
            KekSource::EnvHash => 1,
            KekSource::OsKeystore => 2,
        }
    }
}

/// Resolve the local KEK for *writing* `vault.key`.
///
/// Prefers the OS credential store: if an entry already exists it is reused
/// (so the same KEK survives across launches); otherwise a fresh random KEK is
/// generated and stored. If the OS store is unreachable, falls back to the
/// machine-seed hash so the app stays functional on every platform.
pub fn kek_for_write() -> ([u8; 32], KekSource) {
    match os_keystore_get_or_create() {
        Ok(kek) => (kek, KekSource::OsKeystore),
        // Any failure (no daemon, entry missing AND create refused, platform
        // error) → fall back to the legacy machine-seed KEK. Never errors:
        // the vault must always be able to persist a wrap.
        Err(_) => (crypto::derive_raw_machine_key(), KekSource::EnvHash),
    }
}

/// Resolve the local KEK for *reading* a `vault.key` of the given source.
///
/// For the OS store this may fail (entry deleted, store unreachable) — in which
/// case the caller cannot unlock locally and should surface a message directing
/// the user to restore via a password-protected export bundle.
pub fn kek_for_read(source: KekSource) -> Result<[u8; 32], String> {
    match source {
        KekSource::EnvHash => Ok(crypto::derive_raw_machine_key()),
        KekSource::OsKeystore => os_keystore_get_existing().ok_or_else(|| {
            "vault KEK missing from OS credential store; restore via a password-protected \
                backup (Settings → Import Data)"
                .to_string()
        }),
    }
}

/// Fetch the existing KEK from the OS store, or `None` if absent/unreachable.
fn os_keystore_get_existing() -> Option<[u8; 32]> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    match entry.get_secret() {
        Ok(bytes) => bytes_to_kek(&bytes),
        Err(_) => None,
    }
}

/// Fetch the KEK from the OS store, creating + storing a random one if no entry
/// exists yet. Errors only when the store is unreachable (no entry AND unable
/// to create one), which signals the caller to use the machine-seed fallback.
fn os_keystore_get_or_create() -> Result<[u8; 32], String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("keyring entry: {e}"))?;

    // Try the existing entry first. A present entry is authoritative even on a
    // platform whose store is flaky — we never want to overwrite it.
    match entry.get_secret() {
        Ok(bytes) => {
            return bytes_to_kek(&bytes)
                .ok_or_else(|| "KEK in credential store is malformed".to_string());
        }
        Err(keyring::Error::NoEntry) => {
            // First launch on this machine: mint and store a fresh random KEK.
            let mut kek = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut kek);
            entry
                .set_secret(&kek)
                .map_err(|e| format!("keyring store: {e}"))?;
            Ok(kek)
        }
        Err(e) => Err(format!("keyring read: {e}")),
    }
}

/// Coerce an arbitrary byte slice into a 32-byte KEK. Returns `None` if the
/// stored value isn't exactly 32 bytes (corruption / manual edit).
fn bytes_to_kek(bytes: &[u8]) -> Option<[u8; 32]> {
    if bytes.len() == 32 {
        let mut kek = [0u8; 32];
        kek.copy_from_slice(bytes);
        Some(kek)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kek_source_version_mapping() {
        assert_eq!(KekSource::EnvHash.version(), 1);
        assert_eq!(KekSource::OsKeystore.version(), 2);
    }

    #[test]
    fn bytes_to_kek_rejects_wrong_length() {
        assert!(bytes_to_kek(&[]).is_none());
        assert!(bytes_to_kek(&[0u8; 31]).is_none());
        assert!(bytes_to_kek(&[0u8; 33]).is_none());
        let kek = bytes_to_kek(&[0xABu8; 32]).unwrap();
        assert_eq!(kek, [0xABu8; 32]);
    }

    #[test]
    fn env_hash_kek_is_stable() {
        // The fallback must be deterministic within a process (same machine
        // identity) so a v1 vault.key round-trips across launches.
        let a = crypto::derive_raw_machine_key();
        let b = crypto::derive_raw_machine_key();
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }
}
