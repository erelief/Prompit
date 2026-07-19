use tauri::{AppHandle, Emitter, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::state::WindowConfig;

const WINDOW_WIDTH: f64 = 480.0;
const WINDOW_INIT_HEIGHT: f64 = 120.0;
const OFFSET: f64 = 12.0;

/// Fallback shortcut string used whenever the saved config is unreadable.
const DEFAULT_SHORTCUT: &str = "Alt+Y";

/// Returns (logical_position, grow_above).
fn compute_position(window: &tauri::WebviewWindow) -> Option<(tauri::Position, bool)> {
    // cursor_position returns physical pixels
    let cursor = window.cursor_position().ok()?;

    // Get scale from current or first available monitor
    let scale = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| {
            window
                .available_monitors()
                .ok()
                .and_then(|m| m.into_iter().next())
        })
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);

    // Convert to logical
    let cx = cursor.x / scale;
    let cy = cursor.y / scale;

    // Find which monitor the cursor is on
    let monitors = window.available_monitors().ok()?;
    let monitor = monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        let s = m.scale_factor();
        let left = pos.x as f64 / s;
        let top = pos.y as f64 / s;
        let right = left + size.width as f64 / s;
        let bottom = top + size.height as f64 / s;
        cx >= left && cx < right && cy >= top && cy < bottom
    });

    let m = match monitor {
        Some(m) => m.clone(),
        None => window.current_monitor().ok().flatten()?,
    };

    let ms = m.scale_factor();
    let m_left = m.position().x as f64 / ms;
    let m_top = m.position().y as f64 / ms;
    let m_width = m.size().width as f64 / ms;
    let m_height = m.size().height as f64 / ms;

    // Horizontal: center on cursor, clamp to screen edges
    let mut x = cx - WINDOW_WIDTH / 2.0;
    x = x.max(m_left).min(m_left + m_width - WINDOW_WIDTH);

    // Vertical: smart above/below based on screen half
    let mid_y = m_top + m_height / 2.0;
    let (y, grow_above) = if cy < mid_y {
        (cy + OFFSET, false)
    } else {
        (cy - WINDOW_INIT_HEIGHT - OFFSET, true)
    };

    Some((LogicalPosition::new(x, y).into(), grow_above))
}

/// Parse and validate a shortcut string like "Ctrl+Shift+P" or "Alt+Y".
///
/// Delegates the token→`Code`/`Modifiers` mapping to the plugin's own parser
/// (accepts Alt/Option, Control/Ctrl, Command/Cmd/Super, Shift + keys like
/// A–Z, 0–9, F1–F24, Space, Enter, …), then enforces that at least one
/// modifier is present so a bare key can't become a global hotkey.
pub fn parse_shortcut(s: &str) -> Result<Shortcut, String> {
    use std::str::FromStr;
    let shortcut = Shortcut::from_str(s).map_err(|e| format!("invalid shortcut \"{s}\": {e}"))?;
    if shortcut.mods.is_empty() {
        return Err(format!(
            "shortcut must include at least one modifier (Alt/Ctrl/Shift/Cmd): \"{s}\""
        ));
    }
    Ok(shortcut)
}

/// Re-register the global shortcut with a new binding at runtime.
/// Persisted separately by the frontend (via `save_config`) after success.
/// On any failure the previous binding is restored so the app never ends up
/// with no shortcut registered.
#[tauri::command]
pub fn update_shortcut(app: AppHandle, shortcut: String) -> Result<(), String> {
    // Sandbox never registers OS-global hotkeys (see lib.rs setup). Treat any
    // update as a no-op success so the frontend config still saves.
    if crate::sandbox_enabled() {
        return Ok(());
    }
    parse_shortcut(&shortcut)?;
    let previous = crate::commands::config_cmd::read_config(app.clone())
        .map(|c| c.shortcut)
        .unwrap_or_else(|_| DEFAULT_SHORTCUT.to_string());
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("unregister: {e}"))?;
    if let Err(e) = register(&app, &shortcut) {
        // Roll back to the previous binding instead of leaving nothing registered.
        let _ = register(&app, &previous);
        return Err(format!("register: {e}"));
    }
    Ok(())
}

/// Temporarily release the global hotkey so the webview can capture raw
/// key presses during shortcut recording. Pairs with `finish_record_shortcut`.
#[tauri::command]
pub fn start_record_shortcut(app: AppHandle) -> Result<(), String> {
    if crate::sandbox_enabled() {
        return Ok(());
    }
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("unregister: {e}"))
}

/// Restore the previously stored shortcut after recording ends.
/// Use when the user cancels without choosing a new binding.
#[tauri::command]
pub fn finish_record_shortcut(app: AppHandle) -> Result<(), String> {
    if crate::sandbox_enabled() {
        return Ok(());
    }
    let saved = crate::commands::config_cmd::read_config(app.clone())
        .map(|c| c.shortcut)
        .unwrap_or_else(|_| DEFAULT_SHORTCUT.to_string());
    register(&app, &saved).map_err(|e| format!("register: {e}"))
}

pub fn register(app: &AppHandle, shortcut_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut =
        parse_shortcut(shortcut_str).unwrap_or(Shortcut::new(Some(Modifiers::ALT), Code::KeyY));

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _event, event: ShortcutEvent| {
            if event.state() == ShortcutState::Pressed {
                // Skip shortcut during onboarding
                if let Some(state) = app_handle.try_state::<crate::state::OnboardingState>() {
                    if !state.is_complete() {
                        return;
                    }
                }

                // Don't surface the window before the frontend has mounted:
                // the startup sequence (Vite cold start in dev + sequential
                // vault reads) leaves the transparent undecorated window empty
                // for a few seconds, and showing it then would render only a
                // border. Mirrors the tray-click guard in lib.rs.
                let ready = app_handle
                    .try_state::<crate::state::FrontendReady>()
                    .map(|s| s.is())
                    .unwrap_or(false);
                if !ready {
                    return;
                }

                let main_window = app_handle
                    .get_webview_window("main")
                    .expect("main window not found");

                if main_window.is_visible().unwrap_or(false) {
                    // If on startup-reminder, transition to input view instead of hiding
                    let is_startup_reminder = main_window
                        .url()
                        .map(|u| u.as_str().contains("#/startup-reminder"))
                        .unwrap_or(false);

                    if is_startup_reminder {
                        let _ = main_window.eval("window.location.hash = '/'");
                    } else {
                        // Pinned: keep window open and re-capture input state instead of hiding.
                        let pinned = app_handle
                            .try_state::<crate::state::WindowConfig>()
                            .map(|c| c.is_pinned())
                            .unwrap_or(false);
                        if pinned {
                            let _ = main_window.eval("window.location.hash = '/'");
                            let _ = main_window.set_focus();
                            let _ = main_window.emit("shortcut-triggered", ());
                        } else {
                            let _ = main_window.hide();
                        }
                    }
                } else {
                    let grow_above = match compute_position(&main_window) {
                        Some((pos, ga)) => {
                            let _ = main_window.set_position(pos);
                            ga
                        }
                        None => {
                            let _ = main_window.center();
                            false
                        }
                    };

                    if let Some(cfg) = app_handle.try_state::<WindowConfig>() {
                        cfg.set_grow_above(grow_above);
                    }

                    let _ = main_window.eval("window.location.hash = '/'");
                    let _ = main_window.show();
                    let _ = main_window.set_focus();
                    let _ = main_window.emit("shortcut-triggered", ());
                    let _ = main_window.emit("window-config", grow_above);
                }
            }
        })?;

    Ok(())
}
