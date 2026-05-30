use tauri::Manager;

pub mod commands;
pub mod config;
pub mod shortcut;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state::WindowConfig::default())
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
            commands::secrets::save_secret,
            commands::secrets::read_secret,
            commands::secrets::delete_secret,
            commands::dictionary::read_dictionary,
            commands::dictionary::save_dictionary,
            commands::dictionary::import_dictionary_csv,
            commands::dictionary::export_dictionary_csv,
        ])
        .setup(|app| {
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
