pub mod commands;
pub mod config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::window::hide_main_window,
            commands::window::show_main_window,
            commands::window::open_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
