//! Reconversion (再変換, `F_RECONVERT`): asking the IME to convert the text again.
//!
//! Sakura: `CViewCommander::Command_Reconvert` (`sakura_core/cmd/CViewCommander_Edit_advanced.cpp`)
//! hands the IME a `RECONVERTSTRING` (the text around the caret and the range to convert) with
//! `ImmSetCompositionString`: `SCS_QUERYRECONVERTSTRING` lets the IME widen the range to a word,
//! the editor selects that range, and `SCS_SETRECONVERTSTRING` starts the conversion.
//!
//! What each OS allows:
//!
//! * Windows: IMM32, as Sakura does (`windows.rs`). The IME may not support it
//!   (`IGP_SETCOMPSTR` without `SCS_CAP_SETRECONVERTSTRING`); then Sakura does nothing and this
//!   returns [`Error::Unsupported`].
//! * macOS: the program cannot start a reconversion. The input method asks the text view for the
//!   text around the selection itself (`NSTextInputClient`) when the user presses its key.
//! * Linux: IBus and Fcitx give the program a way to tell the surrounding text, not to start a
//!   reconversion.
//!
//! Both of the last two return [`Error::Unsupported`]. The structure of `RECONVERTSTRING` is built
//! here for every OS, so that it is tested everywhere; the calls into the OS are not run by CI (it
//! has no IME), the user checks them.

pub use raw_window_handle::RawWindowHandle;

#[cfg(not(windows))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use unsupported as os;
#[cfg(windows)]
use windows as os;

/// Why the IME was not asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// This OS, or the IME in use, cannot start a reconversion. The text says why.
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

/// What to convert: some text and a range of it, in UTF-16 units (the unit of `RECONVERTSTRING`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub text: Vec<u16>,
    /// Where the range starts, in units of `text`.
    pub start: usize,
    /// How long the range is. 0 for the caret alone (the IME takes the word before it).
    pub len: usize,
}

/// The size of `RECONVERTSTRING`: eight `DWORD`s.
const HEADER_LEN: usize = 32;

/// A `RECONVERTSTRING` with its text, in the bytes the IME reads and writes.
///
/// Sakura: `CEditView::SetReconvertStruct` fills it, `Command_Reconvert` passes the same buffer
/// to both calls, so what the IME changed in the first (the range) is kept for the second.
#[derive(Clone, Debug)]
pub struct Reconvert {
    bytes: Vec<u8>,
}

impl Reconvert {
    /// Builds the structure: version 0, the string after the header and ending with NUL, the
    /// range as both the composition string and the target (`dwCompStr*`, `dwTargetStr*`).
    /// A NUL inside the text becomes a space ("it may stop answering", `SetReconvertStruct`).
    pub fn new(request: &Request) -> Self {
        let units = request.text.len();
        let start = request.start.min(units);
        let len = request.len.min(units - start);
        let size = HEADER_LEN + (units + 1) * 2;
        let mut bytes = Vec::with_capacity(size);
        let mut put = |value: usize| bytes.extend_from_slice(&(value as u32).to_le_bytes());
        put(size); // dwSize
        put(0); // dwVersion
        put(units); // dwStrLen, in units
        put(HEADER_LEN); // dwStrOffset
        put(len); // dwCompStrLen, in units
        put(start * 2); // dwCompStrOffset, in bytes
        put(len); // dwTargetStrLen
        put(start * 2); // dwTargetStrOffset
        for &unit in &request.text {
            bytes.extend_from_slice(&(if unit == 0 { 0x20 } else { unit }).to_le_bytes());
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
        Self { bytes }
    }

    /// The bytes for `ImmSetCompositionString`.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn word(&self, index: usize) -> usize {
        let at = index * 4;
        u32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap()) as usize
    }

    /// The range of the composition string as it is now, `(start, len)` in UTF-16 units of the
    /// text: what `SCS_QUERYRECONVERTSTRING` left in the structure. `None` when the IME wrote a
    /// version other than 0 or a range outside the text (`SetSelectionFromReonvert` refuses
    /// the first; the second would not be a range of this text).
    pub fn range(&self) -> Option<(usize, usize)> {
        if self.word(1) != 0 {
            return None;
        }
        let (start, len) = (self.word(5) / 2, self.word(4));
        (start.checked_add(len)? <= self.word(2)).then_some((start, len))
    }

    /// Writes a range as the IME does after `SCS_QUERYRECONVERTSTRING` (the composition string
    /// and the target): for a stand-in IME in tests.
    pub fn set_range(&mut self, start: usize, len: usize) {
        let bytes = self.as_bytes_mut();
        for (word, value) in [(4, len), (5, start * 2), (6, len), (7, start * 2)] {
            bytes[word * 4..word * 4 + 4].copy_from_slice(&(value as u32).to_le_bytes());
        }
    }

    /// The bytes the IME may write into (`SCS_QUERYRECONVERTSTRING`); the tests stand in for the
    /// IME with it.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

/// Lets the IME widen the range (`SCS_QUERYRECONVERTSTRING`), then read it back with
/// [`Reconvert::range`]. Fails with [`Error::Unsupported`] when the IME cannot reconvert.
pub fn query(window: RawWindowHandle, reconvert: &mut Reconvert) -> Result<(), Error> {
    os::query(window, reconvert)
}

/// Starts the conversion (`SCS_SETRECONVERTSTRING`). The IME then sends the window the messages of
/// a composition, so call this where the application is not borrowed.
pub fn set(window: RawWindowHandle, reconvert: &Reconvert) -> Result<(), Error> {
    os::set(window, reconvert)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn word(bytes: &[u8], index: usize) -> u32 {
        u32::from_le_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap())
    }

    #[test]
    fn the_structure_has_the_layout_of_reconvertstring() {
        let request = Request {
            text: units("日本語を入力"),
            start: 3,
            len: 2,
        };
        let r = Reconvert::new(&request);
        let b = r.as_bytes();
        assert_eq!(b.len(), 32 + 7 * 2);
        assert_eq!(word(b, 0) as usize, b.len(), "dwSize");
        assert_eq!(word(b, 1), 0, "dwVersion");
        assert_eq!(word(b, 2), 6, "dwStrLen in units");
        assert_eq!(word(b, 3), 32, "dwStrOffset");
        assert_eq!((word(b, 4), word(b, 5)), (2, 6), "the range: units, bytes");
        assert_eq!((word(b, 6), word(b, 7)), (2, 6), "the target is the range");
        // the text follows the header and ends with NUL
        let text: Vec<u16> = b[32..]
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        assert_eq!(text[..6], units("日本語を入力")[..]);
        assert_eq!(text[6], 0);
        assert_eq!(r.range(), Some((3, 2)));
    }

    #[test]
    fn a_nul_in_the_text_becomes_a_space() {
        let r = Reconvert::new(&Request {
            text: vec![0x61, 0, 0x62],
            start: 0,
            len: 0,
        });
        assert_eq!(&r.as_bytes()[32..38], [0x61, 0, 0x20, 0, 0x62, 0]);
        assert_eq!(&r.as_bytes()[38..40], [0, 0], "the end is still NUL");
    }

    #[test]
    fn a_range_outside_the_text_is_cut() {
        let r = Reconvert::new(&Request {
            text: units("ab"),
            start: 9,
            len: 9,
        });
        assert_eq!(r.range(), Some((2, 0)));
        let r = Reconvert::new(&Request {
            text: units("abc"),
            start: 1,
            len: 9,
        });
        assert_eq!(r.range(), Some((1, 2)));
    }

    #[test]
    fn the_range_the_ime_wrote_is_read_back() {
        let mut r = Reconvert::new(&Request {
            text: units("abcdef"),
            start: 2,
            len: 0,
        });
        // the IME widens the range to "bcd"
        let b = r.as_bytes_mut();
        b[16..20].copy_from_slice(&3u32.to_le_bytes());
        b[20..24].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(r.range(), Some((1, 3)));
        // outside the text: refused
        let b = r.as_bytes_mut();
        b[16..20].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(r.range(), None);
        // another version: refused (`pReconv->dwVersion`)
        let b = r.as_bytes_mut();
        b[16..20].copy_from_slice(&1u32.to_le_bytes());
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(r.range(), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn other_systems_cannot_start_a_reconversion() {
        use raw_window_handle::{RawWindowHandle, WebWindowHandle};
        let window = RawWindowHandle::Web(WebWindowHandle::new(1));
        let mut r = Reconvert::new(&Request {
            text: units("a"),
            start: 0,
            len: 0,
        });
        assert!(matches!(query(window, &mut r), Err(Error::Unsupported(_))));
        assert!(matches!(set(window, &r), Err(Error::Unsupported(_))));
    }
}
