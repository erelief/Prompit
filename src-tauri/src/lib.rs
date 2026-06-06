use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub mod commands;
pub mod config;
pub mod shortcut;
pub mod state;

#[cfg(target_os = "windows")]
fn read_windows_system_proxy() -> Option<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let settings = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
        .ok()?;

    let enabled: u32 = settings.get_value("ProxyEnable").ok()?;
    if enabled == 0 {
        return None;
    }

    let server: String = settings.get_value("ProxyServer").ok()?;
    if server.is_empty() {
        return None;
    }

    if server.contains('=') {
        for prefix in ["https=", "http="] {
            for part in server.split(';') {
                if let Some(addr) = part.strip_prefix(prefix) {
                    return Some(prefix_url(addr));
                }
            }
        }
    }

    Some(prefix_url(&server))
}

#[cfg(not(target_os = "windows"))]
fn read_windows_system_proxy() -> Option<String> {
    None
}

fn prefix_url(addr: &str) -> String {
    if addr.starts_with("http") {
        addr.to_string()
    } else {
        format!("http://{}", addr)
    }
}

fn read_proxy_url() -> Option<String> {
    std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
        .or_else(|_| std::env::var("ALL_PROXY"))
        .or_else(|_| std::env::var("all_proxy"))
        .ok()
        .or_else(read_windows_system_proxy)
}

#[tauri::command]
fn get_proxy_url() -> Option<String> {
    read_proxy_url()
}

/// Returns the data directory to use for persistent storage.
/// In sandbox mode, returns the temp dir; otherwise falls back to the default.
pub fn get_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(dir) = app.try_state::<state::DataDir>() {
        if let Some(ref path) = dir.0 {
            return Ok(path.clone());
        }
    }
    app.path()
        .app_config_dir()
        .map_err(|e| format!("config dir: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state::WindowConfig::default())
        .manage(state::OnboardingState::default())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
            commands::window::resize_main_window,
            commands::window::resize_and_reposition,
            commands::window::get_grow_above,
            commands::clipboard::simulate_paste,
            commands::config_cmd::read_config,
            commands::config_cmd::save_config,
            commands::config_cmd::get_config_dir,
            commands::config_cmd::set_onboarding_complete,
            commands::secrets::save_secret,
            commands::secrets::read_secret,
            commands::secrets::delete_secret,
            commands::dictionary::read_dictionary,
            commands::dictionary::save_dictionary,
            commands::dictionary::import_dictionary_csv,
            commands::dictionary::export_dictionary_csv,
            commands::persona::read_personas,
            commands::persona::save_personas,
            commands::presets::read_provider_presets,
            get_proxy_url,
        ])
        .setup(|app| {
            // Sandbox mode: redirect all persistent data to a temp directory
            if std::env::var("SANDBOX").is_ok() {
                let temp_dir = std::env::temp_dir().join(format!(
                    "prompit-sandbox-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ));
                std::fs::create_dir_all(&temp_dir).expect("create sandbox dir");
                eprintln!("[sandbox] using temp dir: {:?}", temp_dir);
                app.manage(state::DataDir(Some(temp_dir)));
            } else {
                app.manage(state::DataDir(None));
            }

            let handle = app.handle().clone();
            shortcut::register(&handle)?;

            // System tray
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let settings_item =
                MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
            let exit_item = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&settings_item, &exit_item])
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                            let _ = w.eval("window.location.hash = '/settings'");
                        }
                    }
                    "exit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.eval("window.location.hash = '/'");
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
