//! Errors of the buffer.

use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, BufferError>;

#[derive(Debug)]
pub enum BufferError {
    /// The source is larger than the limit (ADR-0012).
    SourceTooLarge { len: u64, max: u64 },
    /// The text does not fit in the add buffer (offsets are 32 bits).
    TooLarge,
    /// Reading the source failed.
    Io(io::Error),
    /// The document refers to bytes that do not exist. A bug or a source that changed.
    Corrupt,
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceTooLarge { len, max } => write!(
                f,
                "the file is too large to open ({len} bytes; the limit is {max} bytes)"
            ),
            Self::TooLarge => f.write_str("the document has too much added text"),
            Self::Io(e) => write!(f, "cannot read the source: {e}"),
            Self::Corrupt => f.write_str("the document refers to data that does not exist"),
        }
    }
}

impl std::error::Error for BufferError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for BufferError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
