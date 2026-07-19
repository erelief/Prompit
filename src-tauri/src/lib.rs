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

#[cfg(target_os = "windows")]
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

/// Whether the app is running in sandbox mode (`SANDBOX` env var set). Sandbox
/// mode redirects all persistent data to a temp dir and enables the built-in
/// mock provider / fake-update behaviour so the full UI flow can be exercised
/// without real credentials or network.
pub fn sandbox_enabled() -> bool {
    std::env::var("SANDBOX").is_ok()
}

#[tauri::command]
fn is_sandbox() -> bool {
    sandbox_enabled()
}

/// URL of the built-in sandbox mock provider. Every request to this host is
/// intercepted inside `llm_http` and answered with a canned response, so the
/// provider can be added, tested, and chatted with — all without a network.
const SANDBOX_MOCK_BASE_URL: &str = "https://sandbox-mock.local/v1";

/// Returns the built-in mock provider when running in sandbox mode, and
/// `None` otherwise. The frontend uses this in onboarding to offer a one-click
/// "use sandbox mock provider" shortcut.
#[tauri::command]
fn sandbox_mock_provider() -> Option<config::ProviderConfig> {
    if !sandbox_enabled() {
        return None;
    }
    Some(config::ProviderConfig {
        name: "Sandbox Mock".to_string(),
        api_key: "sandbox-key".to_string(),
        base_url: SANDBOX_MOCK_BASE_URL.to_string(),
        models: vec![config::ProviderModel {
            id: "sandbox-mock-model".to_string(),
            input_capabilities: None,
        }],
        temperature: None,
        max_tokens: None,
        preset: None,
        api_format: None,
    })
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
    // Sandbox: give WebView2 its own user-data folder so a sandbox instance
    // can run side-by-side with a real install without contending for the
    // shared WebView2 cache (which is keyed by app identifier and would
    // otherwise cause startup races / lock errors). Uses the same per-run
    // temp location as the rest of the sandbox data. On macOS/Linux this env
    // var is simply ignored by the respective webviews.
    if sandbox_enabled() {
        let webview_dir = std::env::temp_dir().join(format!(
            "prompit-sandbox-webview-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &webview_dir);
    }

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
        .manage(state::FrontendReady::default())
        .manage(commands::http_proxy::InflightRegistry::default())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
            commands::window::set_webview_bg,
            commands::window::prepare_webview_size,
            commands::window::resize_main_window,
            commands::window::resize_and_reposition,
            commands::window::show_onboarding_window,
            commands::window::show_startup_reminder_window,
            commands::window::set_tray_visible,
            commands::window::get_grow_above,
            commands::window::set_main_pinned,
            commands::window::set_main_resizable,
            commands::window::reset_app_data,
            commands::window::delete_categories,
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
            commands::providers::read_providers,
            commands::providers::read_providers_resolved,
            commands::providers::save_providers,
            commands::websearch::read_websearch,
            commands::websearch::save_websearch,
            commands::dictionary::read_dictionary,
            commands::dictionary::save_dictionary,
            commands::dictionary::import_dictionary_csv,
            commands::dictionary::export_dictionary_csv,
            commands::dictionary::clear_all_dictionaries,
            commands::persona::read_personas,
            commands::persona::save_personas,
            commands::skills_lite::read_skills_lites,
            commands::skills_lite::save_skills_lites,
            commands::skills_lite::export_skills_lite_markdown,
            commands::skills_lite::import_skills_lite_markdown,
            commands::history::read_history,
            commands::history::save_history,
            commands::history::clear_history,
            commands::presets::read_provider_presets,
            commands::presets::read_model_capabilities,
            commands::http_proxy::llm_http,
            commands::http_proxy::llm_http_abort,
            vault::export_data,
            vault::import_data,
            vault::inspect_bundle,
            commands::webdav::webdav_test_connection,
            commands::webdav::webdav_save_password,
            commands::webdav::webdav_has_password,
            commands::webdav::webdav_list_files,
            commands::webdav::webdav_export,
            commands::webdav::webdav_inspect_file,
            commands::webdav::webdav_restore_file,
            get_proxy_url,
            is_sandbox,
            sandbox_mock_provider,
        ])
        .setup(|app| {
            // Sandbox mode: redirect all persistent data to a temp directory
            if sandbox_enabled() {
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

            let handle = app.handle().clone();
            let saved_shortcut = commands::config_cmd::read_config(app.handle().clone())
                .map(|c| c.shortcut)
                .unwrap_or_else(|_| "Alt+Y".to_string());
            // Sandbox: skip OS-global hotkey registration. Two reasons:
            //   1. Global hotkeys are mutually exclusive at the OS level — a
            //      second instance (e.g. sandbox alongside a real install)
            //      would fail to register, or worse, steal the binding from
            //      the real instance.
            //   2. Letting two instances coexist is a sandbox use case.
            if !sandbox_enabled() {
                shortcut::register(&handle, &saved_shortcut)?;
            } else {
                eprintln!("[sandbox] global shortcut registration skipped");
            }

            // System tray
            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let settings_item = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
            let exit_item = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&settings_item, &exit_item])
                .build()?;

            let tray = TrayIconBuilder::with_id("main-tray")
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
                        // Belt-and-suspenders: the tray is also kept hidden
                        // until the frontend is ready, but in case a click
                        // races the visibility flip (or a sleep/wake WebView
                        // reload resets readiness), never show an unmounted
                        // window.
                        let ready = app
                            .try_state::<state::FrontendReady>()
                            .map(|s| s.is())
                            .unwrap_or(false);
                        if !ready {
                            return;
                        }
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
            // Start hidden: the frontend flips this to visible via
            // `set_tray_visible(true)` once Vue has mounted, so the user can
            // never interact with a half-initialized window. See
            // `FrontendReady` in state.rs.
            let _ = tray.set_visible(false);

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
