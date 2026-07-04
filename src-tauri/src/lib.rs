use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub mod commands;
pub mod config;
pub mod crypto;
pub mod kek;
#[cfg(target_os = "windows")]
mod power_watcher;
pub mod shortcut;
pub mod state;
pub mod vault;

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

    // Per-protocol form: "https=host:port;http=host:port;...".
    if server.contains('=') {
        let mut http_fallback = None;
        for part in server.split(';') {
            if let Some(addr) = part.strip_prefix("https=") {
                return Some(prefix_url(addr));
            }
            if http_fallback.is_none() {
                if let Some(addr) = part.strip_prefix("http=") {
                    http_fallback = Some(prefix_url(addr));
                }
            }
        }
        // Fall back to the whole ProxyServer string when no https/http part
        // matched (mirrors the original fall-through below the `=` branch).
        return http_fallback.or_else(|| Some(prefix_url(&server)));
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

#[tauri::command]
fn is_sandbox() -> bool {
    std::env::var("SANDBOX").is_ok()
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
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state::WindowConfig::default())
        .manage(state::OnboardingState::default())
        .manage(state::StartupReminderState::default())
        .manage(commands::http_proxy::InflightRegistry::default())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
            commands::window::resize_main_window,
            commands::window::resize_and_reposition,
            commands::window::show_onboarding_window,
            commands::window::show_startup_reminder_window,
            commands::window::get_grow_above,
            commands::window::set_main_pinned,
            commands::window::reset_app_data,
            commands::clipboard::simulate_paste,
            commands::clipboard::paste_pinned,
            commands::clipboard::copy_text,
            commands::config_cmd::read_config,
            commands::config_cmd::save_config,
            commands::config_cmd::get_config_dir,
            commands::config_cmd::set_onboarding_complete,
            commands::config_cmd::has_shown_startup_reminder,
            commands::config_cmd::mark_startup_reminder_shown,
            #[cfg(target_os = "windows")]
            commands::config_cmd::woke_since_process_start,
            commands::config_cmd::get_shortcut_label,
            shortcut::update_shortcut,
            shortcut::start_record_shortcut,
            shortcut::finish_record_shortcut,
            commands::secrets::save_secret,
            commands::secrets::read_secret,
            commands::secrets::delete_secret,
            commands::dictionary::read_dictionary,
            commands::dictionary::save_dictionary,
            commands::dictionary::import_dictionary_csv,
            commands::dictionary::export_dictionary_csv,
            commands::dictionary::clear_all_dictionaries,
            commands::persona::read_personas,
            commands::persona::save_personas,
            commands::skills_lite::read_skills_lites,
            commands::skills_lite::save_skills_lites,
            commands::history::read_history,
            commands::history::save_history,
            commands::history::clear_history,
            commands::presets::read_provider_presets,
            commands::presets::read_model_capabilities,
            commands::http_proxy::llm_http,
            commands::http_proxy::llm_http_abort,
            vault::export_data,
            vault::import_data,
            get_proxy_url,
            is_sandbox,
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

            // Unlock (or create) the Master Key vault before any encrypted data
            // is read. Must run after DataDir is registered, since the vault
            // lives in the data dir. A failure here is fatal: without a Master
            // Key, no encrypted file can be decrypted.
            vault::unlock_or_migrate(app.handle()).expect("fatal: failed to initialize vault");

            // Migrate the legacy skills-lite file (`sparkles.json` under scope
            // "sparkles") to the renamed `skills_lite.json` / scope
            // "skills_lite". Best-effort: failures are logged and swallowed
            // (load_skills_lites_encrypted also has a read-side fallback).
            commands::skills_lite::migrate_legacy_file(app.handle());

            let handle = app.handle().clone();
            let saved_shortcut = commands::config_cmd::read_config(app.handle().clone())
                .map(|c| c.shortcut)
                .unwrap_or_else(|_| "Alt+Y".to_string());
            shortcut::register(&handle, &saved_shortcut)?;

            // System tray
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let settings_item = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
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

            // Watch for system sleep/wake to fix transparent window surface loss
            #[cfg(target_os = "windows")]
            if let Some(main_win) = app.get_webview_window("main") {
                if let Ok(hwnd) = main_win.hwnd() {
                    let raw = hwnd.0;
                    if !raw.is_null() {
                        power_watcher::start(raw, app.handle().clone());
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
