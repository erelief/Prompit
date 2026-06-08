use tauri::{AppHandle, Manager};

use crate::state::WindowConfig;

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn resize_main_window(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize {
            width: width as u32,
            height: height as u32,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Resize the window to fit content. When grow_above is true, keeps the
/// bottom edge fixed so the window expands upward.
#[tauri::command]
pub fn resize_and_reposition(app: AppHandle, height: f64, width: Option<f64>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let scale = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);

    let physical_h = (height * scale) as u32;
    let physical_w = ((width.unwrap_or(480.0)) * scale) as u32;

    let grow_above = app.state::<WindowConfig>().get_grow_above();

    if grow_above {
        if let Ok(current_pos) = window.outer_position() {
            if let Ok(current_size) = window.outer_size() {
                let dy = current_size.height as i32 - physical_h as i32;
                // outer_position is physical; convert to logical for set_position
                let new_x = current_pos.x as f64 / scale;
                let new_y = (current_pos.y as f64 + dy as f64) / scale;
                let _ = window.set_position(tauri::LogicalPosition::new(new_x, new_y));
            }
        }
    }

    let _ = window.set_size(tauri::PhysicalSize {
        width: physical_w,
        height: physical_h,
    });

    Ok(())
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn show_onboarding_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let scale = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);

    let w = (380.0 * scale) as u32;
    let h = (560.0 * scale) as u32;

    window
        .set_size(tauri::PhysicalSize { width: w, height: h })
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn show_startup_reminder_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let scale = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);

    let w = (380.0 * scale) as u32;
    let h = (280.0 * scale) as u32;

    window
        .set_size(tauri::PhysicalSize { width: w, height: h })
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reset_app_data(app: AppHandle) -> Result<(), String> {
    let dir = crate::get_data_dir(&app)?;

    // Safety check: if SANDBOX env is set, the data dir must be the sandbox
    // temp dir, not the real app config dir. If they don't match, abort.
    if std::env::var("SANDBOX").is_ok() {
        let real_dir = app
            .path()
            .app_config_dir()
            .map_err(|e| format!("config dir: {e}"))?;
        if dir == real_dir {
            return Err(
                "Sandbox safety check failed: data dir is real config dir. Aborting reset.".into(),
            );
        }
    }

    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("delete: {e}"))?;
    }
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn get_grow_above(app: AppHandle) -> bool {
    app.state::<WindowConfig>().get_grow_above()
}
