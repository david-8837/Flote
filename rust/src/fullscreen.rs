//! Full-screen app detection (Win32). Used to make the notch step aside when a
//! full-screen app is running.
//!
//! Heuristic: if the foreground window (not ours) covers the primary monitor's
//! work area, treat a full-screen app as active. This is the one place a light
//! check is acceptable — Win32 has no event for "another app went fullscreen".

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFOEXW, MONITOR_DEFAULTTONULL,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect};

/// Returns true if a full-screen app (other than our window) appears active.
pub fn fullscreen_active(our_hwnd: *mut core::ffi::c_void) -> bool {
    unsafe {
        let fg: HWND = GetForegroundWindow();
        if fg as *mut core::ffi::c_void == our_hwnd {
            return false;
        }
        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONULL);
        if monitor.is_null() {
            return false;
        }
        let mut mi: MONITORINFOEXW = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, &mut mi) == 0 {
            return false;
        }
        let rc: RECT = mi.rcMonitor;
        let mut win: RECT = std::mem::zeroed();
        if GetWindowRect(fg, &mut win) == 0 {
            return false;
        }
        let tol = 8i32;
        win.left <= rc.left + tol
            && win.top <= rc.top + tol
            && win.right >= rc.right - tol
            && win.bottom >= rc.bottom - tol
    }
}

/// Primary monitor size in physical pixels (width, height).
pub fn primary_monitor_size() -> (u32, u32) {
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MONITORINFOEXW,
        };
        use windows_sys::Win32::Foundation::POINT;
        let mut pt: POINT = std::mem::zeroed();
        // MonitorFromPoint with a zero point defaults to the nearest monitor.
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONULL);
        if monitor.is_null() {
            return (1920, 1080);
        }
        let mut mi: MONITORINFOEXW = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(monitor, &mut mi) == 0 {
            return (1920, 1080);
        }
        let rc = mi.rcMonitor;
        let w = (rc.right - rc.left).max(0) as u32;
        let h = (rc.bottom - rc.top).max(0) as u32;
        if w == 0 || h == 0 {
            (1920, 1080)
        } else {
            (w, h)
        }
    }
}
