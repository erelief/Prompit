pub mod commands;
pub mod config;
pub mod shortcut;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
            commands::window::resize_main_window,
            commands::clipboard::simulate_paste,
            commands::config_cmd::read_config,
            commands::config_cmd::save_config,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            shortcut::register(&handle)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
