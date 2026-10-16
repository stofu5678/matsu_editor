//! Where a file is read into a document: [`ume_core::file::open_file_with`] (detection of the
//! encoding, the BOM, the size limit of ADR-0012, progress and cancel).
//!
//! A path that does not exist opens as an empty document that has that name, as Sakura does
//! (`CLoadAgent::OnLoad`, `sakura_core/agent/CLoadAgent.cpp`): the file is made when it is saved.

use std::path::Path;

use ume_core::buffer::Eol;
use ume_core::encoding::Encoding;
use ume_core::file::{FileError, OpenOptions, Progress, TextFile, open_file_with};

/// Why a file could not be opened.
pub type OpenError = FileError;

/// Opens `path`. `encoding` forces the encoding instead of detecting it. A missing file is an
/// empty document with that path.
pub fn open_text_file(
    path: &Path,
    encoding: Option<Encoding>,
    progress: Progress<'_>,
) -> Result<TextFile, OpenError> {
    let options = OpenOptions {
        encoding,
        ..OpenOptions::default()
    };
    match open_file_with(path, &options, progress) {
        Err(FileError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(new_named(path, encoding))
        }
        other => other,
    }
}

/// An empty document that has a name but no file yet.
fn new_named(path: &Path, encoding: Option<Encoding>) -> TextFile {
    let options = OpenOptions::default();
    let encoding = encoding.unwrap_or(options.detect.default_encoding);
    let mut file = TextFile::new_untitled(encoding, encoding.bom_default_on(), Eol::CrLf);
    file.path = Some(path.to_owned());
    file
}

#[cfg(test)]
mod tests {
    use ume_core::file::no_progress;

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ume-ui-open-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_is_an_empty_document_with_its_name() {
        let path = Path::new("/nonexistent/ume-test-file");
        let file = open_text_file(path, None, &mut no_progress).unwrap();
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
        let file = open_text_file(&path, Some(Encoding::ShiftJis), &mut no_progress).unwrap();
        assert_eq!(file.encoding, Encoding::ShiftJis);
        assert_eq!(file.document.line(0).unwrap().unwrap().text, "日本語");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_directory_is_an_error() {
        let dir = temp_dir("dir");
        assert!(open_text_file(&dir, None, &mut no_progress).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
