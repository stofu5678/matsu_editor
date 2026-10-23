//! Reconversion with IMM32 on Windows, as Sakura does (`Command_Reconvert`,
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`). Not run by CI, which has no IME; the user
//! checks it.

// The Win32 calls are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::Ime::{
    ImmGetContext, ImmGetProperty, ImmReleaseContext, ImmSetCompositionStringW,
    SCS_CAP_SETRECONVERTSTRING, SCS_QUERYRECONVERTSTRING, SCS_SETRECONVERTSTRING,
    SET_COMPOSITION_STRING_TYPE,
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
        let context = ImmGetContext(hwnd);
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
