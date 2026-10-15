//! Where a document is read from a file: the one place to switch when the file I/O PR (#H)
//! brings real encodings and a file-backed buffer.
//!
//! For now the file is read into memory and taken as UTF-8, with `Document::from_utf8`.

use std::fmt;
use std::path::Path;

use ume_core::buffer::{BufferError, Document};

/// Why a file could not be opened.
#[derive(Debug)]
pub enum OpenError {
    Io(std::io::Error),
    Buffer(BufferError),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Buffer(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OpenError {}

impl From<std::io::Error> for OpenError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<BufferError> for OpenError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

/// Reads `path` as a UTF-8 document.
pub fn open_document(path: &Path) -> Result<Document, OpenError> {
    let bytes = std::fs::read(path)?;
    Ok(Document::from_utf8(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_an_io_error() {
        let err = open_document(Path::new("/nonexistent/ume-test-file")).unwrap_err();
        assert!(matches!(err, OpenError::Io(_)));
    }

    #[test]
    fn reads_a_utf8_file() {
        let dir = std::env::temp_dir().join(format!("ume-ui-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.txt");
        std::fs::write(&path, "日本語\r\nabc").unwrap();
        let doc = open_document(&path).unwrap();
        assert_eq!(doc.line_count(), 2);
        assert_eq!(doc.line(0).unwrap().unwrap().text, "日本語");
        std::fs::remove_dir_all(&dir).ok();
    }
}
