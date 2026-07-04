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

/// Process-level flag for whether the startup reminder has already been shown
/// in this process session. Unlike `show_startup_reminder` (a persisted user
/// preference), this lives only in memory, so it survives WebView reloads
/// triggered by sleep/wake (lid close/open) — which would otherwise re-run the
/// frontend's startup routing and re-show the reminder. Resets naturally when
/// the process exits.
pub struct StartupReminderState {
    pub shown: AtomicBool,
}

impl Default for StartupReminderState {
    fn default() -> Self {
        Self {
            shown: AtomicBool::new(false),
        }
    }
}

impl StartupReminderState {
    pub fn mark_shown(&self) {
        self.shown.store(true, Ordering::Relaxed);
    }

    pub fn has_shown(&self) -> bool {
        self.shown.load(Ordering::Relaxed)
    }
}

/// Whether the frontend (Vue app) has finished mounting and is ready to render
/// window content. Until this flips to true, the system tray icon is kept
/// hidden and tray-click / global-shortcut show-paths are suppressed, so the
/// user can never interact with a half-initialized window (transparent border
/// only, no rendered content).
///
/// The flag is process-scoped: it is set from the frontend right after
/// `app.mount("#app")` (see `src/main.ts`). On a sleep/wake-triggered WebView
/// reload the frontend re-runs its startup sequence and re-sets this; in the
/// meantime the tray stays interactive because only the webview reloaded, not
/// the process — but the show-paths still consult this flag, so a click that
/// races the remount is dropped instead of showing an empty window.
pub struct FrontendReady(pub AtomicBool);

impl Default for FrontendReady {
    fn default() -> Self {
        Self(AtomicBool::new(false))
    }
}

impl FrontendReady {
    pub fn set(&self, value: bool) {
        self.0.store(value, Ordering::Relaxed);
    }

    pub fn is(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
