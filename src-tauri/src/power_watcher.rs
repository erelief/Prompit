use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetMessageW, RegisterClassW,
    HWND_MESSAGE, MSG, WNDCLASSW, WS_EX_NOACTIVATE, WM_POWERBROADCAST,
};

const PBT_APMRESUMEAUTOMATIC: WPARAM = 0x00000012;

static MAIN_HWND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_POWERBROADCAST && wparam == PBT_APMRESUMEAUTOMATIC {
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
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Spawn a background thread that listens for system resume events and forces
/// the main window to rebuild its composited surface. This works around a
/// WebView2 bug where transparent, undecorated windows lose their alpha surface
/// after lid-close / sleep / wake.
pub fn start(main_hwnd: *mut c_void) {
    MAIN_HWND.store(main_hwnd, Ordering::Relaxed);

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
        while GetMessageW(&mut msg, hwnd, 0, 0) > 0 {
            // wndproc is invoked directly by GetMessageW
        }

        DestroyWindow(hwnd);
    });
}
