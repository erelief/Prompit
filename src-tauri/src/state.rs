use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Override for app_config_dir in sandbox mode.
/// The temp directory is deleted on drop (normal exit or panic).
pub struct DataDir(pub Option<PathBuf>);

impl Drop for DataDir {
    fn drop(&mut self) {
        if let Some(ref path) = self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

pub struct WindowConfig {
    pub grow_above: AtomicBool,
    pub pinned: AtomicBool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            grow_above: AtomicBool::new(false),
            pinned: AtomicBool::new(false),
        }
    }
}

impl WindowConfig {
    pub fn set_grow_above(&self, value: bool) {
        self.grow_above.store(value, Ordering::Relaxed);
    }

    pub fn get_grow_above(&self) -> bool {
        self.grow_above.load(Ordering::Relaxed)
    }

    pub fn set_pinned(&self, value: bool) {
        self.pinned.store(value, Ordering::Relaxed);
    }

    pub fn is_pinned(&self) -> bool {
        self.pinned.load(Ordering::Relaxed)
    }
}

pub struct OnboardingState {
    pub complete: AtomicBool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            complete: AtomicBool::new(false),
        }
    }
}

impl OnboardingState {
    pub fn set_complete(&self, value: bool) {
        self.complete.store(value, Ordering::Relaxed);
    }

    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Relaxed)
    }
}
