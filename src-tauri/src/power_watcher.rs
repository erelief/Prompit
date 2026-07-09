use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::OnceLock;
use std::thread;

use tauri::{AppHandle, Emitter, Manager};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Power::{
    RegisterPowerSettingNotification, UnregisterPowerSettingNotification, POWERBROADCAST_SETTING,
};
use windows_sys::Win32::System::SystemServices::GUID_CONSOLE_DISPLAY_STATE;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetMessageW, IsWindowVisible, RegisterClassW,
    SetWindowPos, ShowWindow, HWND_MESSAGE, MSG, WM_POWERBROADCAST, WNDCLASSW, WS_EX_NOACTIVATE,
    DEVICE_NOTIFY_WINDOW_HANDLE, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_HIDE,
    SW_SHOW,
};

// PBT_APMRESUMEAUTOMATIC exists in windows-sys, but the other two resume
// events (RESUMESUSPEND / RESUMECRITICAL) are not exposed by 0.59's bindings,
// so define all three by value here. Per WinUser.h:
//   PBT_APMRESUMECRITICAL   = 0x0006  (resumed from critical sleep — battery died)
//   PBT_APMRESUMESUSPEND    = 0x0007  (resumed by user action: lid open / key / mouse)
//   PBT_APMRESUMEAUTOMATIC  = 0x0012  (resumed automatically: wake timer / update / WOL)
// The lid-close/open path users hit most goes through RESUMESUSPEND, which the
// previous code did NOT handle — only 0x12 was wired up, so opening the lid
// left the composited window surface transparent/white until a manual Ctrl+R.
const PBT_APMRESUMECRITICAL: WPARAM = 0x00000006;
const PBT_APMRESUMESUSPEND: WPARAM = 0x00000007;
const PBT_APMRESUMEAUTOMATIC: WPARAM = 0x00000012;
// `PBT_POWERSETTINGCHANGE` is exposed by windows-sys as a `u32`, but `wparam`
// is `WPARAM` (`usize`); define it here as a `WPARAM` so the match arms stay
// guard-free and uniform with the resume constants above.
const PBT_POWERSETTINGCHANGE: WPARAM = 32787;

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

/// Repair the main window's composited surface after the GPU/DWM context was
/// torn down during sleep. This is the fix for the transparent/white window
/// bug. Only acts when the window is currently visible — the main window
/// defaults to hidden and is summoned on demand, so blindly doing
/// ShowWindow(SW_HIDE)→SW_SHOW would *pop up* a window the user had closed.
/// Always emits `system-resumed` so the frontend can recompute geometry
/// regardless of visibility.
unsafe fn repair_surface() {
    let main = MAIN_HWND.load(Ordering::Relaxed);
    if !main.is_null() && IsWindowVisible(main) != 0 {
        // Force DWM to rebuild the composited surface for the transparent
        // window: a hide/show cycle plus a frame-change notification.
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

/// Handle `PBT_POWERSETTINGCHANGE` carrying `GUID_CONSOLE_DISPLAY_STATE`.
/// On Modern Standby (lid close turns the screen off without a full sleep),
/// no RESUME event fires; instead the display-off/on transition arrives here.
/// `Data[0] != 0` means the console display just turned back on.
unsafe fn handle_display_state_change(lparam: LPARAM) {
    if lparam == 0 {
        return;
    }
    let setting = &*(lparam as *const POWERBROADCAST_SETTING);
    if guid_eq(&setting.PowerSetting, &GUID_CONSOLE_DISPLAY_STATE)
        && setting.DataLength >= 1
        && setting.Data[0] != 0
    {
        // Display turned on — but the system may not have actually slept, so
        // do NOT touch WOKE_SINCE_PROCESS_START (that flag means "resumed from
        // sleep", which is a strictly stronger condition than "screen lit").
        repair_surface();
    }
}

/// Field-wise equality for `windows_sys::core::GUID`, which does not derive
/// `PartialEq`. All four fields (u32, u16, u16, [u8; 8]) derive it, so we
/// compare them as a tuple.
fn guid_eq(a: &windows_sys::core::GUID, b: &windows_sys::core::GUID) -> bool {
    (a.data1, a.data2, a.data3, a.data4) == (b.data1, b.data2, b.data3, b.data4)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_POWERBROADCAST {
        match wparam {
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND | PBT_APMRESUMECRITICAL => {
                WOKE_SINCE_PROCESS_START.store(true, Ordering::Relaxed);
                repair_surface();
            }
            PBT_POWERSETTINGCHANGE => {
                handle_display_state_change(lparam);
            }
            _ => {}
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

        // Subscribe to console display state changes so we catch the
        // Modern-Standby case (lid close turns the screen off without a real
        // sleep → no PBT_APMRESUME* event). HWND_MESSAGE windows are valid
        // recipients for window-handle notifications.
        let notify = RegisterPowerSettingNotification(
            hwnd as windows_sys::Win32::Foundation::HANDLE,
            &GUID_CONSOLE_DISPLAY_STATE,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        );

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, hwnd, 0, 0) > 0 {}

        if notify != 0 {
            UnregisterPowerSettingNotification(notify);
        }

        DestroyWindow(hwnd);
    });
}
