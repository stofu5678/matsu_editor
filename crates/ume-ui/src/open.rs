//! Where a file is read into a document: [`ume_core::file::open_file_with`] (detection of the
//! encoding, the BOM, the size limit of ADR-0012, progress and cancel).
//!
//! A path that does not exist opens as an empty document that has that name, as Sakura does
//! (`CLoadAgent::OnLoad`, `sakura_core/agent/CLoadAgent.cpp`): the file is made when it is saved.
//!
//! The encoding, BOM and line ending of a file that does not say (and of a new file) come from the
//! settings of its document type ([`NewDocumentDefaults`]); without a setting, the line ending is
//! the one of the OS.

use std::path::Path;

use ume_core::encoding::Encoding;
use ume_core::file::{FileError, OpenOptions, Progress, TextFile, open_file_with};

use crate::settings::NewDocumentDefaults;

/// Why a file could not be opened.
pub type OpenError = FileError;

/// Opens `path`. `encoding` forces the encoding instead of detecting it. A missing file is an
/// empty document with that path. `defaults` are the settings of the type of the file: the
/// encoding used when detection has no clue, and the line ending of a file without one.
pub fn open_text_file(
    path: &Path,
    encoding: Option<Encoding>,
    defaults: &NewDocumentDefaults,
    progress: Progress<'_>,
) -> Result<TextFile, OpenError> {
    let mut options = OpenOptions {
        encoding,
        default_eol: defaults.eol,
        mime_decode: defaults.mime_decode,
        ext_eol: ume_commands::edit_options::get().ext_eol,
        ..OpenOptions::default()
    };
    options.detect.default_encoding = defaults.encoding;
    match open_file_with(path, &options, progress) {
        Err(FileError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(new_named(path, encoding, defaults))
        }
        other => other,
    }
}

/// An empty document that has a name but no file yet.
fn new_named(path: &Path, encoding: Option<Encoding>, defaults: &NewDocumentDefaults) -> TextFile {
    // A forced encoding gets the BOM that is usual for it; the default one gets the setting.
    let (encoding, bom) = match encoding {
        Some(encoding) if encoding != defaults.encoding => (encoding, encoding.bom_default_on()),
        _ => (defaults.encoding, defaults.bom),
    };
    let mut file = TextFile::new_untitled(encoding, bom, defaults.eol);
    file.path = Some(path.to_owned());
    file
}

#[cfg(test)]
mod tests {
    use ume_core::buffer::Eol;
    use ume_core::file::no_progress;

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ume-ui-open-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn defaults() -> NewDocumentDefaults {
        NewDocumentDefaults::default()
    }

    fn os_eol() -> Eol {
        if cfg!(windows) { Eol::CrLf } else { Eol::Lf }
    }

    #[test]
    fn a_missing_file_gets_the_line_ending_of_the_os() {
        let path = Path::new("/nonexistent/ume-test-file");
        let file = open_text_file(path, None, &defaults(), &mut no_progress).unwrap();
        assert_eq!(file.new_line, os_eol());
        assert!(!file.bom);
    }

    #[test]
    fn a_missing_file_takes_the_settings_of_its_type() {
        let path = Path::new("/nonexistent/ume-test-file");
        let settings = NewDocumentDefaults {
            encoding: Encoding::Utf16Le,
            bom: false,
            eol: Eol::Cr,
            mime_decode: false,
        };
        let file = open_text_file(path, None, &settings, &mut no_progress).unwrap();
        assert_eq!(file.encoding, Encoding::Utf16Le);
        assert!(!file.bom);
        assert_eq!(file.new_line, Eol::Cr);
        // A forced encoding has its own usual BOM.
        let file = open_text_file(path, Some(Encoding::Utf8), &settings, &mut no_progress).unwrap();
        assert_eq!(file.encoding, Encoding::Utf8);
        assert!(!file.bom);
    }

    #[test]
    fn mime_words_of_a_jis_file_are_decoded_when_the_settings_say_so() {
        let dir = temp_dir("mime");
        let path = dir.join("mail.txt");
        std::fs::write(&path, b"=?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?=\r\n").unwrap();
        let text = |defaults: &NewDocumentDefaults| {
            let file =
                open_text_file(&path, Some(Encoding::Jis), defaults, &mut no_progress).unwrap();
            file.document
                .text_between(
                    ume_core::buffer::Pos::new(0, 0),
                    ume_core::buffer::Pos::new(usize::MAX, 0),
                )
                .unwrap()
        };
        assert_eq!(text(&defaults()), "=?ISO-2022-JP?B?GyRCRnxLXDhsGyhC?=\r\n");
        let on = NewDocumentDefaults {
            mime_decode: true,
            ..defaults()
        };
        assert_eq!(text(&on), "日本語\r\n");
    }

    #[test]
    fn a_file_without_a_line_ending_takes_the_default_and_one_with_keeps_its_own() {
        let dir = temp_dir("eol");
        let lf = NewDocumentDefaults {
            eol: Eol::Lf,
            ..defaults()
        };
        let crlf = NewDocumentDefaults {
            eol: Eol::CrLf,
            ..defaults()
        };
        let none = dir.join("none.txt");
        std::fs::write(&none, b"abc").unwrap();
        assert_eq!(
            open_text_file(&none, None, &lf, &mut no_progress)
                .unwrap()
                .new_line,
            Eol::Lf
        );
        assert_eq!(
            open_text_file(&none, None, &crlf, &mut no_progress)
                .unwrap()
                .new_line,
            Eol::CrLf
        );
        let with = dir.join("with.txt");
        std::fs::write(&with, b"a\r\nb\n").unwrap();
        assert_eq!(
            open_text_file(&with, None, &lf, &mut no_progress)
                .unwrap()
                .new_line,
            Eol::CrLf
        );
        let unix = dir.join("unix.txt");
        std::fs::write(&unix, b"a\nb\r\n").unwrap();
        assert_eq!(
            open_text_file(&unix, None, &crlf, &mut no_progress)
                .unwrap()
                .new_line,
            Eol::Lf
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_file_is_an_empty_document_with_its_name() {
        let path = Path::new("/nonexistent/ume-test-file");
        let file = open_text_file(path, None, &defaults(), &mut no_progress).unwrap();
        assert_eq!(file.path.as_deref(), Some(path));
        assert_eq!(file.encoding, Encoding::Utf8);
        assert_eq!(file.document.line_count(), 0);
    }

    #[test]
    fn reads_a_file_in_the_forced_encoding() {
        let dir = temp_dir("sjis");
        let path = dir.join("a.txt");
        // "日本語" in Shift_JIS, then a CR LF.
        std::fs::write(&path, b"\x93\xFA\x96\x7B\x8C\xEA\r\nabc").unwrap();
        let file = open_text_file(
            &path,
            Some(Encoding::ShiftJis),
            &defaults(),
            &mut no_progress,
        )
        .unwrap();
        assert_eq!(file.encoding, Encoding::ShiftJis);
        assert_eq!(file.document.line(0).unwrap().unwrap().text, "日本語");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_directory_is_an_error() {
        let dir = temp_dir("dir");
        assert!(open_text_file(&dir, None, &defaults(), &mut no_progress).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
