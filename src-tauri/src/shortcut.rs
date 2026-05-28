use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

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
                    let _ = main_window.show();
                    let _ = main_window.set_focus();
                    let _ = main_window.emit("shortcut-triggered", ());
                }
            }
        },
    )?;

    Ok(())
}
