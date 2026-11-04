//! The IME at the start (起動時のIME, `nImeState` of the type): turning the IME on or off and
//! choosing its input mode when a document is opened.
//!
//! Sakura: `CDocEditor::SetImeMode` (`sakura_core/doc/CDocEditor.cpp`) takes the context of the
//! view (`ImmGetContext`) and, in this order: closes the IME when the state asks for off
//! (`ImmSetOpenStatus(FALSE)`), sets the conversion mode (`ImmGetConversionStatus` and
//! `ImmSetConversionStatus`) when the input mode is not "as it is", and opens the IME when the
//! state asks for on. The state is `on/off | input mode << 2`.
//!
//! What each OS allows:
//!
//! * Windows: IMM32, as Sakura does (`windows.rs`).
//! * macOS: a program cannot open or close the input method. It can only pick one of the
//!   installed input sources (`TISSelectInputSource`), which changes the keyboard layout of the
//!   whole system and is not "the IME of this document".
//! * Linux: IBus and Fcitx have a switch of their own over D-Bus, for the whole session and not
//!   per window; Wayland's text input protocol gives the program no way to ask for it.
//!
//! Both of the last two return [`Error::Unsupported`]. [`steps`], which says what to ask of the
//! IME, is the same on every OS and is tested everywhere; the calls into the OS are not run by
//! CI (it has no IME), the user checks them.

pub use raw_window_handle::RawWindowHandle;

#[cfg(not(windows))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use unsupported as os;
#[cfg(windows)]
use windows as os;

/// `IME_CMODE_NATIVE` (`imm.h`): Japanese input (hiragana) instead of alphanumeric.
pub const CMODE_NATIVE: u32 = 0x0001;
/// `IME_CMODE_KATAKANA`.
pub const CMODE_KATAKANA: u32 = 0x0002;
/// `IME_CMODE_FULLSHAPE`: full-width characters.
pub const CMODE_FULLSHAPE: u32 = 0x0008;
/// `IME_CMODE_NOCONVERSION`: the IME does not convert.
pub const CMODE_NOCONVERSION: u32 = 0x0100;

/// Why the IME could not be set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// This OS cannot do it. The text says why.
    Unsupported(&'static str),
    /// The OS refused, or the handle is not a window of this OS.
    Failed(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unsupported(why) | Error::Failed(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {}

/// What to ask of the IME for a state (`SetImeMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Steps {
    /// Close the IME first (the state says off).
    pub close: bool,
    /// Change the conversion mode: the bits to set and the bits to clear. `None` for "as it is".
    pub conversion: Option<(u32, u32)>,
    /// Open the IME last (the state says on).
    pub open: bool,
}

/// The steps of `nImeState`: the low two bits are the on/off state (1 on, 2 off, else as it is),
/// the bits above them the input mode (1 full-width, 2 full-width hiragana, 3 full-width
/// katakana, 4 no conversion, else as it is).
pub fn steps(state: i32) -> Steps {
    let conversion = match state >> 2 {
        1 => Some((CMODE_FULLSHAPE, CMODE_NOCONVERSION)),
        2 => Some((
            CMODE_FULLSHAPE | CMODE_NATIVE,
            CMODE_KATAKANA | CMODE_NOCONVERSION,
        )),
        3 => Some((
            CMODE_FULLSHAPE | CMODE_NATIVE | CMODE_KATAKANA,
            CMODE_NOCONVERSION,
        )),
        4 => Some((CMODE_NOCONVERSION, 0)),
        _ => None,
    };
    Steps {
        close: state & 3 == 2,
        conversion,
        open: state & 3 == 1,
    }
}

/// The conversion mode after a step: `mode` with the bits of `conversion` set and cleared. Sakura
/// sets first and clears after (`conv |= ..; conv &= ~..`).
pub fn convert(mode: u32, conversion: (u32, u32)) -> u32 {
    (mode | conversion.0) & !conversion.1
}

/// Why this OS cannot set the IME, or `None` when it can.
pub fn unsupported() -> Option<&'static str> {
    os::unsupported()
}

/// Sets the IME of the window as the state says (`SetImeMode`). A state that asks for nothing
/// does nothing, and succeeds.
pub fn apply(window: RawWindowHandle, state: i32) -> Result<(), Error> {
    let steps = steps(state);
    if steps == Steps::default() {
        return Ok(());
    }
    os::apply(window, steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_state_asks_for_nothing() {
        // `IME_CMODE_NOCONVERSION` (0x100) is the file's default: its mode is not one of the list.
        assert_eq!(steps(0x0100), Steps::default());
        assert_eq!(steps(0), Steps::default());
        assert_eq!(steps(3), Steps::default());
    }

    #[test]
    fn on_and_off_are_the_low_two_bits() {
        assert!(steps(1).open && !steps(1).close);
        assert!(steps(2).close && !steps(2).open);
        assert_eq!(steps(1).conversion, None);
    }

    #[test]
    fn the_input_mode_is_above_them() {
        assert_eq!(
            steps(1 << 2).conversion,
            Some((CMODE_FULLSHAPE, CMODE_NOCONVERSION))
        );
        let hira = steps(2 << 2).conversion.unwrap();
        assert_eq!(
            convert(CMODE_KATAKANA | CMODE_NOCONVERSION, hira),
            CMODE_FULLSHAPE | CMODE_NATIVE
        );
        let kata = steps(3 << 2).conversion.unwrap();
        assert_eq!(
            convert(0, kata),
            CMODE_FULLSHAPE | CMODE_NATIVE | CMODE_KATAKANA
        );
        let none = steps(4 << 2).conversion.unwrap();
        assert_eq!(
            convert(CMODE_NATIVE, none),
            CMODE_NATIVE | CMODE_NOCONVERSION
        );
        // both parts at once: on, full-width katakana
        let both = steps((3 << 2) | 1);
        assert!(both.open && both.conversion.is_some());
    }

    #[cfg(not(windows))]
    #[test]
    fn where_the_os_cannot_the_state_is_refused_with_a_reason() {
        assert!(unsupported().is_some());
        let handle = RawWindowHandle::Xlib(raw_window_handle::XlibWindowHandle::new(1));
        assert!(matches!(apply(handle, 1), Err(Error::Unsupported(_))));
        // nothing to ask is nothing to refuse
        assert_eq!(apply(handle, 0), Ok(()));
    }
}
