//! Printing (ADR-0011): the page layout shared by the preview and the printers, the print
//! preview in GPUI, and printing on macOS. Windows and Linux have the preview only, for now.

pub mod layout;
#[cfg(target_os = "macos")]
pub mod mac;
pub mod preview;

use layout::{DocInfo, Paper, PrintSetting};

/// What printing needs, taken from the editor when it starts: the text, the setting, and what
/// the header shows of the document.
// Only macOS prints so far.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub struct PrintJob {
    pub lines: Vec<String>,
    pub setting: PrintSetting,
    pub doc: DocInfo,
}

/// Sakura prints in ＭＳ 明朝 by default. These are the Mincho fonts that each OS has.
#[cfg(target_os = "macos")]
const PRINT_FONT: &str = "Hiragino Mincho ProN";
#[cfg(windows)]
const PRINT_FONT: &str = "MS Mincho";
#[cfg(not(any(windows, target_os = "macos")))]
const PRINT_FONT: &str = "Noto Serif CJK JP";

/// Sakura's default print setting, in the Mincho font of the OS.
pub fn default_setting() -> PrintSetting {
    PrintSetting {
        font_han: PRINT_FONT.into(),
        font_zen: PRINT_FONT.into(),
        ..PrintSetting::default()
    }
}

/// The paper to lay the pages out on, and its name for the preview bar.
pub fn paper() -> (Paper, String) {
    #[cfg(target_os = "macos")]
    return mac::paper();
    // Sakura's default, with no printer to ask.
    #[cfg(not(target_os = "macos"))]
    (Paper::A4, "A4  縦".into())
}
