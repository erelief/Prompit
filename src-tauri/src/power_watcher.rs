use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::OnceLock;
use std::thread;

use tauri::{AppHandle, Emitter, Manager};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetMessageW, RegisterClassW, HWND_MESSAGE, MSG,
    WM_POWERBROADCAST, WNDCLASSW, WS_EX_NOACTIVATE,
};

const PBT_APMRESUMEAUTOMATIC: WPARAM = 0x00000012;

static MAIN_HWND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Set true when the system resumed from sleep in this process session. The
/// frontend polls this on mount so a remount triggered by the wake itself
/// (which races ahead of the system-resumed event listener) can still detect
/// that a wake happened and force a layout recompute.
static WOKE_SINCE_PROCESS_START: AtomicBool = AtomicBool::new(false);

/// Whether the system has resumed from sleep at least once in this process.
pub fn woke_since_process_start() -> bool {
    WOKE_SINCE_PROCESS_START.load(Ordering::Relaxed)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_POWERBROADCAST && wparam == PBT_APMRESUMEAUTOMATIC {
        WOKE_SINCE_PROCESS_START.store(true, Ordering::Relaxed);
        let main = MAIN_HWND.load(Ordering::Relaxed);
        if !main.is_null() {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, ShowWindow, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
                SW_HIDE, SW_SHOW,
            };
            ShowWindow(main, SW_HIDE);
            ShowWindow(main, SW_SHOW);
            SetWindowPos(
                main,
                0 as HWND,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
        if let Some(app) = APP_HANDLE.get() {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.emit("system-resumed", ());
            }
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub fn start(main_hwnd: *mut c_void, app: AppHandle) {
    MAIN_HWND.store(main_hwnd, Ordering::Relaxed);
    let _ = APP_HANDLE.set(app);

    thread::spawn(move || unsafe {
        let class_name: Vec<u16> = "PrompitPowerWatcher\0".encode_utf16().collect();

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(ptr::null()),
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            ptr::null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            0 as HWND,
            0 as HWND,
            ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, hwnd, 0, 0) > 0 {}

        DestroyWindow(hwnd);
    });
}
