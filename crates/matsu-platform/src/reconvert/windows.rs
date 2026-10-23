//! Reconversion with IMM32 on Windows, as Sakura does (`Command_Reconvert`,
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`). Not run by CI, which has no IME; the user
//! checks it.

// The Win32 calls are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::Ime::{
    IACE_DEFAULT, ImmAssociateContextEx, ImmGetContext, ImmGetProperty, ImmReleaseContext,
    ImmSetCompositionStringW, SCS_CAP_SETRECONVERTSTRING, SCS_QUERYRECONVERTSTRING,
    SCS_SETRECONVERTSTRING, SET_COMPOSITION_STRING_TYPE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;

use super::{Error, RawWindowHandle, Reconvert};

/// `IGP_SETCOMPSTR` of `imm.h`: what the IME can do with `ImmSetCompositionString`.
const IGP_SETCOMPSTR: u32 = 0x0000_0014;

fn hwnd(window: RawWindowHandle) -> Result<HWND, Error> {
    match window {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
        _ => Err(Error::Failed("Win32 のウィンドウではない")),
    }
}

/// The IME context of the window. GPUI takes it off the window while the focused element does
/// not take text (`update_ime_enabled` in gpui-pre-windows, at every frame), which is so while a
/// menu is open; then the window has none. Give it the default context back. It stays, so that
/// the composition this starts is not cut off; GPUI associates it again by itself when the
/// editor takes text input, and its flag only differs for the moment in between.
///
/// # Safety
/// `hwnd` is a window of this thread. The caller releases a context that is not null.
unsafe fn context_of(hwnd: HWND) -> windows_sys::Win32::UI::Input::Ime::HIMC {
    // SAFETY: as the caller promises.
    unsafe {
        let context = ImmGetContext(hwnd);
        if !context.is_null() {
            return context;
        }
        ImmAssociateContextEx(hwnd, std::ptr::null_mut(), IACE_DEFAULT);
        ImmGetContext(hwnd)
    }
}

/// `ImmSetCompositionString( hIMC, index, bytes, size, nullptr, 0 )` on the context of the window,
/// after checking that the IME takes reconversion at all (Sakura does nothing when it does not).
/// `SCS_QUERYRECONVERTSTRING` writes the adjusted range into `bytes`.
fn call(
    window: RawWindowHandle,
    index: SET_COMPOSITION_STRING_TYPE,
    bytes: *mut u8,
    len: usize,
) -> Result<(), Error> {
    let hwnd = hwnd(window)?;
    // SAFETY: it only reads the state of the calling thread's keyboard layout.
    let capabilities = unsafe { ImmGetProperty(GetKeyboardLayout(0), IGP_SETCOMPSTR) };
    if capabilities & SCS_CAP_SETRECONVERTSTRING == 0 {
        return Err(Error::Unsupported(
            "使っている日本語入力は、再変換を受け付けない",
        ));
    }
    // SAFETY: `hwnd` is a window of this process (the caller's) and the context is released
    // below. `bytes` points to `len` bytes that the caller owns for the whole call, and the
    // structure in them says that size in `dwSize`; it is writable where the caller passes a
    // `&mut` (the query), and only read by the other index.
    unsafe {
        let context = context_of(hwnd);
        if context.is_null() {
            return Err(Error::Failed("入力コンテキストを取得できない"));
        }
        let ok = ImmSetCompositionStringW(
            context,
            index,
            bytes.cast(),
            len as u32,
            std::ptr::null(),
            0,
        );
        ImmReleaseContext(hwnd, context);
        if ok == 0 {
            return Err(Error::Failed("日本語入力が再変換を受け付けなかった"));
        }
    }
    Ok(())
}

pub(super) fn query(window: RawWindowHandle, reconvert: &mut Reconvert) -> Result<(), Error> {
    let bytes = reconvert.as_bytes_mut();
    call(
        window,
        SCS_QUERYRECONVERTSTRING,
        bytes.as_mut_ptr(),
        bytes.len(),
    )
}

pub(super) fn set(window: RawWindowHandle, reconvert: &Reconvert) -> Result<(), Error> {
    // The IME reads this one only; the pointer is `*mut` for the API's sake.
    let bytes = reconvert.as_bytes();
    call(
        window,
        SCS_SETRECONVERTSTRING,
        bytes.as_ptr().cast_mut(),
        bytes.len(),
    )
}

#[cfg(test)]
mod tests {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow};

    use super::*;

    /// A window of a class every Windows has (the check runs without an IME).
    fn window() -> HWND {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        // SAFETY: a system class, no parent, no menu; destroyed by the test.
        unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                class.as_ptr(),
                0,
                0,
                0,
                10,
                10,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        }
    }

    #[test]
    fn a_window_that_lost_its_ime_context_gets_the_default_one_back() {
        let hwnd = window();
        assert!(!hwnd.is_null());
        // SAFETY: the window is ours and lives until `DestroyWindow` below.
        unsafe {
            // what GPUI does while the focused element takes no text
            ImmAssociateContextEx(hwnd, std::ptr::null_mut(), 0);
            assert!(
                ImmGetContext(hwnd).is_null(),
                "no context after the removal"
            );
            let context = context_of(hwnd);
            assert!(!context.is_null(), "the default context is back");
            ImmReleaseContext(hwnd, context);
            // and when it has one, it is left as it is
            let again = context_of(hwnd);
            assert_eq!(again, context);
            ImmReleaseContext(hwnd, again);
            DestroyWindow(hwnd);
        }
    }
}
