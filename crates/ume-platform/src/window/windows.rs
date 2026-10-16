//! Placing windows on Windows, as Sakura does (`SetWindowPos`, `GetMonitorWorkRect`,
//! `WindowTopMost`). Not run by CI, which has no desktop; the user checks it.

// The Win32 calls are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, HWND_NOTOPMOST, HWND_TOPMOST,
    IsIconic, IsZoomed, SM_CXSIZE, SM_CXSIZEFRAME, SM_CYSIZE, SM_CYSIZEFRAME, SW_RESTORE,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    ShowWindow, WS_EX_TOPMOST,
};

use super::{Error, RawWindowHandle, Rect};

fn hwnd(window: RawWindowHandle) -> Result<HWND, Error> {
    match window {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
        _ => Err(Error::Failed("Win32 のウィンドウではない")),
    }
}

fn rect(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right, r.bottom)
}

pub(super) fn unsupported() -> Option<&'static str> {
    None
}

pub(super) fn work_area(window: RawWindowHandle) -> Result<Rect, Error> {
    let hwnd = hwnd(window)?;
    // SAFETY: `info` is initialised with its size, as GetMonitorInfoW requires; `hwnd` is a window
    // of this process, and the monitor is the nearest one if the window is gone.
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err(Error::Failed("作業領域の取得"));
        }
        Ok(rect(info.rcWork))
    }
}

pub(super) fn frame(window: RawWindowHandle) -> Result<Rect, Error> {
    let hwnd = hwnd(window)?;
    let mut r: RECT = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: `r` is a RECT to write into.
    if unsafe { GetWindowRect(hwnd, &mut r) } == 0 {
        return Err(Error::Failed("ウィンドウの位置の取得"));
    }
    Ok(rect(r))
}

pub(super) fn is_minimized(window: RawWindowHandle) -> Result<bool, Error> {
    let hwnd = hwnd(window)?;
    // SAFETY: IsIconic only reads the state of the window.
    Ok(unsafe { IsIconic(hwnd) } != 0)
}

pub(super) fn place(window: RawWindowHandle, to: Rect) -> Result<(), Error> {
    let hwnd = hwnd(window)?;
    // SAFETY: plain calls with a window of this process.
    unsafe {
        // A maximized window would keep its state and be moved as a maximized one.
        if IsZoomed(hwnd) != 0 || IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        let ok = SetWindowPos(
            hwnd,
            null_mut(),
            to.left,
            to.top,
            to.width(),
            to.height(),
            SWP_NOOWNERZORDER | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        if ok == 0 {
            return Err(Error::Failed("ウィンドウの移動"));
        }
    }
    Ok(())
}

pub(super) fn cascade_step() -> (i32, i32) {
    // SAFETY: GetSystemMetrics has no requirements.
    unsafe {
        (
            GetSystemMetrics(SM_CXSIZEFRAME) + GetSystemMetrics(SM_CXSIZE),
            GetSystemMetrics(SM_CYSIZEFRAME) + GetSystemMetrics(SM_CYSIZE),
        )
    }
}

pub(super) fn is_topmost(window: RawWindowHandle) -> Result<bool, Error> {
    let hwnd = hwnd(window)?;
    // SAFETY: only reads the extended style.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    Ok(style as u32 & WS_EX_TOPMOST != 0)
}

pub(super) fn set_topmost(window: RawWindowHandle, topmost: bool) -> Result<(), Error> {
    let hwnd = hwnd(window)?;
    let after = if topmost {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    // SAFETY: plain call with a window of this process.
    let ok = unsafe {
        SetWindowPos(
            hwnd,
            after,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
    if ok == 0 {
        return Err(Error::Failed("常に手前に表示の切り替え"));
    }
    Ok(())
}
