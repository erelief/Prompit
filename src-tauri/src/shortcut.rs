use tauri::{AppHandle, Emitter, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

use crate::state::WindowConfig;

const WINDOW_WIDTH: f64 = 480.0;
const WINDOW_INIT_HEIGHT: f64 = 120.0;
const OFFSET: f64 = 12.0;

/// Returns (logical_position, grow_above).
fn compute_position(window: &tauri::WebviewWindow) -> Option<(tauri::Position, bool)> {
    // cursor_position returns physical pixels
    let cursor = window.cursor_position().ok()?;

    // Get scale from current or first available monitor
    let scale = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.available_monitors().ok().and_then(|m| m.into_iter().next()))
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

pub fn register(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyY);

    let app_handle = app.clone();
    app.global_shortcut().on_shortcut(
        shortcut,
        move |_app, _event, event: ShortcutEvent| {
            if event.state() == ShortcutState::Pressed {
                let main_window = app_handle
                    .get_webview_window("main")
                    .expect("main window not found");

                if main_window.is_visible().unwrap_or(false) {
                    let _ = main_window.hide();
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
        },
    )?;

    Ok(())
}
