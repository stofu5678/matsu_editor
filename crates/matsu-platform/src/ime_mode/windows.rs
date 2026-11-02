//! The IME at the start with IMM32 on Windows, as Sakura does (`CDocEditor::SetImeMode`,
//! `sakura_core/doc/CDocEditor.cpp`). Not run by CI, which has no IME; the user checks it.

// The Win32 calls are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::Ime::{
    IACE_DEFAULT, ImmAssociateContextEx, ImmGetContext, ImmGetConversionStatus, ImmReleaseContext,
    ImmSetConversionStatus, ImmSetOpenStatus,
};

use super::{Error, RawWindowHandle, Steps, convert};

fn hwnd(window: RawWindowHandle) -> Result<HWND, Error> {
    match window {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as HWND),
        _ => Err(Error::Failed("Win32 のウィンドウではない")),
    }
}

pub(super) fn unsupported() -> Option<&'static str> {
    None
}

pub(super) fn apply(window: RawWindowHandle, steps: Steps) -> Result<(), Error> {
    let hwnd = hwnd(window)?;
    // SAFETY: `hwnd` is a window of this process. GPUI takes the context off a window while the
    // focused element does not take text; then the default one is given back, as the
    // reconversion does (`reconvert::windows`). The context is released before returning, and
    // the two out parameters are `u32`s that live through the call.
    unsafe {
        let mut context = ImmGetContext(hwnd);
        if context.is_null() {
            ImmAssociateContextEx(hwnd, std::ptr::null_mut(), IACE_DEFAULT);
            context = ImmGetContext(hwnd);
        }
        if context.is_null() {
            return Err(Error::Failed("入力コンテキストを取得できない"));
        }
        // The order of Sakura: off, then the mode, then on.
        if steps.close {
            ImmSetOpenStatus(context, 0);
        }
        if let Some(conversion) = steps.conversion {
            let mut mode = 0u32;
            let mut sentence = 0u32;
            if ImmGetConversionStatus(context, &mut mode, &mut sentence) != 0 {
                ImmSetConversionStatus(context, convert(mode, conversion), sentence);
            }
        }
        if steps.open {
            ImmSetOpenStatus(context, 1);
        }
        ImmReleaseContext(hwnd, context);
    }
    Ok(())
}
