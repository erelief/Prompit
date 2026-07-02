use tauri::{AppHandle, Manager};

use crate::state::WindowConfig;

/// Fetch the main webview window or return a consistent error string.
fn main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("main").ok_or("Main window not found".to_string())
}

/// Make the window visible and give it keyboard focus. Shared by the show /
/// settings / onboarding / startup-reminder commands.
fn show_and_focus(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Effective DPI scale factor of the window's current monitor, defaulting to
/// 1.0 when no monitor can be resolved.
fn monitor_scale(window: &tauri::WebviewWindow) -> f64 {
    window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0)
}

/// Resize the main window to the given logical dimensions, center it on
/// screen, then show + focus it. Shared by the onboarding and startup-reminder
/// windows, which differ only in their target size.
fn resize_center_show(app: &AppHandle, logical_w: f64, logical_h: f64) -> Result<(), String> {
    let window = main_window(app)?;
    let scale = monitor_scale(&window);
    window
        .set_size(tauri::PhysicalSize {
            width: (logical_w * scale) as u32,
            height: (logical_h * scale) as u32,
        })
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;
    show_and_focus(&window)
}

#[tauri::command]
pub fn hide_main_window(app: AppHandle) -> Result<(), String> {
    main_window(&app)?.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    show_and_focus(&main_window(&app)?)
}

#[tauri::command]
pub fn resize_main_window(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    let window = main_window(&app)?;
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
///
/// On Windows and macOS the resize + reposition is done in a SINGLE atomic
/// platform call. This avoids the one-frame gap between Tauri's separate
/// `set_position` + `set_size` calls that produced a visible "jump" when the
/// window grew upward (position moved first, size updated a frame later, so
/// the bottom edge briefly slid).
///
/// On other platforms (Linux) the previous two-call behavior is kept as a
/// fallback.
#[tauri::command]
pub fn resize_and_reposition(
    app: AppHandle,
    height: f64,
    width: Option<f64>,
) -> Result<(), String> {
    let window = main_window(&app)?;

    let scale = monitor_scale(&window);

    let physical_h = (height * scale) as u32;
    let physical_w = ((width.unwrap_or(500.0)) * scale) as u32;

    let grow_above = app.state::<WindowConfig>().get_grow_above();

    #[cfg(target_os = "windows")]
    {
        // Win32 SetWindowPos sets position + size together in one call.
        // All values are physical pixels (Tauri is per-monitor DPI-aware v2),
        // which also fixes a prior logical/physical mismatch: set_position used
        // to divide by scale while set_size did not.
        //
        // IMPORTANT: tao's set_size treats the value as the CLIENT (inner) size
        // and compensates for the hidden offset of undecorated-with-shadow
        // windows (DWM extends the window rect into negative space for the
        // drop shadow). If we pass the client size directly to SetWindowPos as
        // the OUTER size, the client area ends up too small and content looks
        // "squished" (input too close to the edge). So we recompute the outer
        // size = client size + (window_rect - client_rect), mirroring tao.
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetClientRect, GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOCOPYBITS,
            SWP_NOOWNERZORDER, SWP_NOZORDER,
        };

        let hwnd = match window.hwnd() {
            Ok(h) => h,
            Err(_) => return Ok(()),
        };
        // windows (tao) HWND is windows::Win32::Foundation::HWND; its inner
        // field is the raw handle pointer windows-sys expects.
        let hwnd_raw = hwnd.0;

        // Convert desired client size -> outer window size, accounting for the
        // shadow offset the same way tao::set_inner_size does.
        let (outer_w, outer_h): (i32, i32) = unsafe {
            let mut client = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            let mut win = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd_raw, &mut client) == 0 || GetWindowRect(hwnd_raw, &mut win) == 0 {
                // Fallback: assume no shadow offset.
                (physical_w as i32, physical_h as i32)
            } else {
                let w_off = (win.right - win.left) - (client.right - client.left);
                let h_off = (win.bottom - win.top) - (client.bottom - client.top);
                (physical_w as i32 + w_off, physical_h as i32 + h_off)
            }
        };

        // outer_position / outer_size are physical px on Windows and include
        // the shadow region. When growing upward, keep the BOTTOM edge fixed:
        // new_top = current_bottom - new_outer_height.
        let (new_x, new_y): (i32, i32) = if grow_above {
            match (window.outer_position(), window.outer_size()) {
                (Ok(pos), Ok(size)) => {
                    let bottom = pos.y as i64 + size.height as i64;
                    (pos.x, (bottom - outer_h as i64) as i32)
                }
                _ => return Ok(()),
            }
        } else {
            match window.outer_position() {
                Ok(pos) => (pos.x, pos.y),
                Err(_) => return Ok(()),
            }
        };

        let flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOCOPYBITS;
        unsafe {
            SetWindowPos(hwnd_raw, 0 as _, new_x, new_y, outer_w, outer_h, flags);
        }
    }

    #[cfg(target_os = "macos")]
    {
        // NSWindow -setFrame:display: sets origin + size atomically in one
        // message. AppKit calls must run on the main thread, so dispatch via
        // run_on_main_thread.
        //
        // Cocoa's screen origin is bottom-left, and NSRect.origin is the
        // window's BOTTOM-left. So:
        //  - grow_above: keep origin.y → bottom edge fixed, window grows up.
        //  - otherwise : keep the TOP edge fixed → shift origin.y so the top
        //                stays put while the bottom moves (grow down).
        // height/width are logical points here (matches what the frontend
        // measures in CSS px); setFrame takes logical points.
        use objc2_app_kit::{NSView, NSWindow};
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let target_w = width.unwrap_or(500.0);
        let target_h = height;
        let app_clone = app.clone();
        app.run_on_main_thread(move || {
            let Some(w) = app_clone.get_webview_window("main") else {
                return;
            };
            let Ok(handle) = w.window_handle() else {
                return;
            };
            let RawWindowHandle::AppKit(a) = handle.as_raw() else {
                return;
            };
            unsafe {
                // a.ns_view is the content NSView; promote it to its NSWindow.
                let ns_view = &*(a.ns_view.as_ptr() as *mut NSView);
                let Some(ns_win) = ns_view.window() else {
                    return;
                };
                let frame = ns_win.frame();
                let original_h = frame.size.height;
                let mut new_frame = frame;
                new_frame.size.width = target_w;
                new_frame.size.height = target_h;
                if !grow_above {
                    // Top edge (screen y) = origin.y + height. Keep it constant.
                    new_frame.origin.y = frame.origin.y + original_h - target_h;
                }
                ns_win.setFrame_display(new_frame, true);
            }
        });
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // Fallback (Linux etc.): keep the previous two-call behavior.
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
    }

    Ok(())
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    show_and_focus(&main_window(&app)?)
}

#[tauri::command]
pub fn show_onboarding_window(app: AppHandle) -> Result<(), String> {
    resize_center_show(&app, 380.0, 560.0)
}

#[tauri::command]
pub fn show_startup_reminder_window(app: AppHandle) -> Result<(), String> {
    resize_center_show(&app, 380.0, 280.0)
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

#[tauri::command]
pub fn set_main_pinned(app: AppHandle, pinned: bool) {
    app.state::<WindowConfig>().set_pinned(pinned);
}
