//! Printing (ADR-0011). The page layout is `matsu-print`'s; the modules here put the pages on
//! paper, each in the way of its OS:
//!
//! * [`draw`]: what every drawer draws, as a list of characters and lines per page. No OS.
//! * `windows`: the print dialog (`PrintDlgW`) and GDI, as Sakura does
//!   (`sakura_core/print/CPrint.cpp`, `CPrintPreview.cpp`).
//! * `mac`: AppKit's `NSPrintOperation`, with the pages drawn by Core Text.
//! * `linux`: a PDF ([`pdf`]) handed to the print portal (`org.freedesktop.portal.Print`),
//!   which shows the desktop's print dialog. GTK's own dialog is LGPL and is not used.

pub mod draw;
#[cfg(not(any(windows, target_os = "macos")))]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(not(target_os = "macos"))]
pub mod pdf;
#[cfg(windows)]
pub mod windows;

use std::fmt;
use std::sync::Arc;

use matsu_print::{
    HeaderContext, Layout, LineSource, Paper, PrintError, PrintSetting, TypeOptions,
};

/// What printing needs, taken from the editor when the user prints. It owns everything, so that
/// it can go to the thread that shows the print dialog.
#[derive(Clone)]
pub struct PrintJob {
    /// The lines of the document. A snapshot: editing goes on while the dialog is open.
    pub source: Arc<dyn LineSource + Send + Sync>,
    pub setting: PrintSetting,
    pub options: TypeOptions,
    /// What `$f` and the other variables of the header and the footer show.
    pub header: HeaderContext,
    /// The name of the print job in the print queue.
    pub title: String,
}

impl fmt::Debug for PrintJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PrintJob")
            .field("lines", &self.source.line_count())
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}

impl PrintJob {
    /// Lays the document out on `paper`.
    pub fn layout(&self, paper: Paper) -> Result<Layout, PrintError> {
        Layout::new(&*self.source, &self.setting, paper, &self.options)
    }

    /// The paper of the setting, or Sakura's A4 portrait when it has none
    /// (`CPrintPreview::OnChangePrintSetting`).
    pub fn setting_paper(&self) -> Paper {
        Paper::of_setting(&self.setting).unwrap_or(Paper::A4)
    }
}

/// A print that did not happen, with the reason for the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self(message.to_owned())
    }
}

impl From<PrintError> for Error {
    fn from(err: PrintError) -> Self {
        Self(err.to_string())
    }
}

/// Sakura prints in ＭＳ 明朝 by default. These are the Mincho fonts that each OS has
/// (the same choice as the PoC of 2026-10-11).
#[cfg(target_os = "macos")]
pub const PRINT_FONT: &str = "Hiragino Mincho ProN";
#[cfg(windows)]
pub const PRINT_FONT: &str = "MS Mincho";
#[cfg(not(any(windows, target_os = "macos")))]
pub const PRINT_FONT: &str = "Noto Serif CJK JP";

/// Sakura's default print setting (`CPrint::SettingInitialize`), in the Mincho font of the OS.
pub fn default_setting() -> PrintSetting {
    PrintSetting {
        print_font_face_han: PRINT_FONT.into(),
        print_font_face_zen: PRINT_FONT.into(),
        ..PrintSetting::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_setting_is_sakuras_in_the_font_of_the_os() {
        let s = default_setting();
        let sakura = PrintSetting::default();
        assert_eq!(s.print_font_face_han, PRINT_FONT);
        assert_eq!(s.print_font_face_zen, PRINT_FONT);
        // Everything else is Sakura's.
        let same = PrintSetting {
            print_font_face_han: sakura.print_font_face_han.clone(),
            print_font_face_zen: sakura.print_font_face_zen.clone(),
            ..s
        };
        assert_eq!(same, sakura);
    }

    #[test]
    fn a_job_lays_out_on_the_paper_of_its_setting() {
        let job = PrintJob {
            source: Arc::new(vec!["a".to_string(), String::new()]),
            setting: default_setting(),
            options: TypeOptions::default(),
            header: HeaderContext::default(),
            title: "t".into(),
        };
        assert_eq!(job.setting_paper(), Paper::A4);
        assert_eq!(job.layout(job.setting_paper()).unwrap().page_count(), 1);
    }
}
