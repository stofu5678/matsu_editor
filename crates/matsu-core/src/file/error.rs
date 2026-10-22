//! Errors of opening and saving.

use std::fmt;
use std::io;

use super::save::UnmappableReport;
use crate::buffer::BufferError;

pub type Result<T> = std::result::Result<T, FileError>;

#[derive(Debug)]
pub enum FileError {
    /// The file is larger than the limit the caller set (or than the address space). Nothing was
    /// read.
    TooLarge { len: u64, max: u64 },
    /// The progress callback asked to stop. A cancelled save leaves the target as it was.
    Cancelled,
    /// Saving was refused because characters cannot be written in the encoding. Nothing was
    /// written. Save again with `allow_unmappable` to write `?` for them (Sakura's "yes").
    Unmappable(UnmappableReport),
    /// The document has no path yet: use `save_as`.
    NoPath,
    /// The target is read-only. Nothing was written.
    ReadOnly,
    /// Reading or writing failed. A temporary file is removed.
    Io(io::Error),
    /// The document could not be built or read.
    Buffer(BufferError),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { len, max } => write!(
                f,
                "the file is too large to open ({len} bytes; the limit is {max} bytes)"
            ),
            Self::Cancelled => f.write_str("cancelled"),
            Self::Unmappable(r) => write!(
                f,
                "{} character(s) cannot be written in the chosen encoding",
                r.total
            ),
            Self::NoPath => f.write_str("the document has no file name yet"),
            Self::ReadOnly => f.write_str("the file is read-only"),
            Self::Io(e) => write!(f, "file error: {e}"),
            Self::Buffer(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Buffer(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for FileError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<BufferError> for FileError {
    fn from(e: BufferError) -> Self {
        match e {
            // Report the size problem the same way whichever layer found it.
            BufferError::SourceTooLarge { len, max } => Self::TooLarge { len, max },
            other => Self::Buffer(other),
        }
    }
}
