use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct AifwState {
    child: Mutex<Option<Child>>,
}

impl AifwState {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *guard {
            match child.try_wait() {
                Ok(Some(_)) => {
                    *guard = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn start(&self, exe_path: &str) -> Result<(), String> {
        if self.is_running() {
            return Ok(());
        }

        let child = Command::new(exe_path)
            .spawn()
            .map_err(|e| format!("Failed to start AIFW: {e}"))?;

        let mut guard = self.child.lock().unwrap();
        *guard = Some(child);
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref mut child) = *guard {
            child
                .kill()
                .map_err(|e| format!("Failed to kill AIFW: {e}"))?;
            *guard = None;
        }
        Ok(())
    }
}

#[tauri::command]
pub fn start_aifw(app: AppHandle, exe_path: String) -> Result<(), String> {
    let state = app.state::<AifwState>();
    state.start(&exe_path)
}

#[tauri::command]
pub fn stop_aifw(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AifwState>();
    state.stop()
}

#[tauri::command]
pub fn aifw_status(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<AifwState>();
    Ok(state.is_running())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aifw_state_initial_not_running() {
        let state = AifwState::new();
        assert!(!state.is_running());
    }

    #[test]
    fn test_aifw_stop_when_not_running() {
        let state = AifwState::new();
        assert!(state.stop().is_ok());
    }

    #[test]
    fn test_aifw_start_nonexistent_exe() {
        let state = AifwState::new();
        let result = state.start("nonexistent_aifw_server.exe");
        assert!(result.is_err());
        assert!(!state.is_running());
    }
}
