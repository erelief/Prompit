use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

fn platform_paste_key() -> Key {
    if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    }
}

fn simulate_paste_keystrokes(enigo: &mut Enigo) -> Result<(), String> {
    let modifier = platform_paste_key();
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| format!("key press: {e}"))?;
    enigo
        .key(Key::V, Direction::Click)
        .map_err(|e| format!("v click: {e}"))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| format!("key release: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn simulate_paste(text: String) -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Failed to init enigo: {e}"))?;

    // Backup current clipboard
    let backup = arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .unwrap_or_default();

    // Write translation to clipboard
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("clipboard init: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("clipboard set: {e}"))?;
    }

    // Small delay to ensure clipboard is set
    thread::sleep(Duration::from_millis(50));

    // Simulate paste
    simulate_paste_keystrokes(&mut enigo)?;

    // Wait for paste to complete, then restore clipboard
    thread::sleep(Duration::from_millis(100));
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(backup);
    }

    Ok(())
}

/// Pinned-mode paste: hide the window, paste into the previously focused app,
/// then restore the window — all within a single command call so the JS side
/// only does one IPC round-trip (minimizing the visible hide/show gap).
///
/// Clipboard backup is restored on a background thread so it doesn't extend
/// the window's hidden window.
#[tauri::command]
pub fn paste_pinned(app: AppHandle, text: String) -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Failed to init enigo: {e}"))?;

    // Backup current clipboard
    let backup = arboard::Clipboard::new()
        .and_then(|mut c| c.get_text())
        .unwrap_or_default();

    // Write translation to clipboard before hiding, so it's ready immediately
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("clipboard init: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("clipboard set: {e}"))?;
    }

    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // Hide so OS focus returns to the previous app. A short wait is needed for
    // the focus handoff to settle before the simulated keystrokes.
    window.hide().map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(30));

    simulate_paste_keystrokes(&mut enigo)?;

    // Wait just long enough for the paste to be consumed, then show again.
    thread::sleep(Duration::from_millis(40));
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_focus();

    // Restore the original clipboard off the critical path.
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(backup);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_paste_key() {
        let key = platform_paste_key();
        if cfg!(target_os = "macos") {
            assert!(matches!(key, Key::Meta));
        } else {
            assert!(matches!(key, Key::Control));
        }
    }
}
