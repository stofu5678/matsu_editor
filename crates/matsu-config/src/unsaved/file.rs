//! The file of one document: `MATSU-UNSAVED 1`, the length of the information, the information, the text.
//!
//! The information and the text are in one file so that one rename replaces both. Two files could be
//! left as new text with old information (a document switched from Shift_JIS to UTF-8 would come back
//! with the old encoding).

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};

use super::info::DocInfo;

/// The first line of every document file. The number is the version of the layout.
pub(super) const MAGIC: &str = "MATSU-UNSAVED 1";

/// The most bytes the information part may have. It holds a path and some numbers, so a larger one is
/// a damaged file; this keeps a damaged length from asking for a huge buffer.
const MAX_INFO_LEN: usize = 1 << 20;

/// Why a document file could not be read.
#[derive(Debug)]
pub enum FormatError {
    /// The file could not be read.
    Io(io::Error),
    /// The file is not a document file of this version, or is cut short.
    Damaged(&'static str),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Io(e) => write!(f, "{e}"),
            FormatError::Damaged(why) => write!(f, "damaged document file: {why}"),
        }
    }
}

impl std::error::Error for FormatError {}

impl From<io::Error> for FormatError {
    fn from(e: io::Error) -> Self {
        FormatError::Io(e)
    }
}

/// An opened document file: the information is read, the text is left to be read from [`DocFile::text`].
#[derive(Debug)]
pub struct DocFile {
    pub info: DocInfo,
    /// The text, as the bytes of UTF-8 (without a BOM), positioned at its start. Its length is `text_len`.
    pub text: BufReader<File>,
    /// The length of the text in bytes.
    pub text_len: u64,
}

/// The bytes before the text: the magic line, the length line and the information.
pub(super) fn header_bytes(info: &DocInfo) -> io::Result<Vec<u8>> {
    let info_bytes = info
        .to_bytes()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let mut out = Vec::with_capacity(info_bytes.len() + 32);
    out.extend_from_slice(MAGIC.as_bytes());
    out.push(b'\n');
    out.extend_from_slice(info_bytes.len().to_string().as_bytes());
    out.push(b'\n');
    out.extend_from_slice(&info_bytes);
    Ok(out)
}

pub(super) fn open(file: File) -> Result<DocFile, FormatError> {
    let total = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.trim_end() != MAGIC {
        return Err(FormatError::Damaged("the first line is not the magic"));
    }
    line.clear();
    reader.read_line(&mut line)?;
    let info_len: usize = line
        .trim_end()
        .parse()
        .map_err(|_| FormatError::Damaged("the length of the information is not a number"))?;
    if info_len > MAX_INFO_LEN {
        return Err(FormatError::Damaged("the information is too long"));
    }
    let mut info_bytes = vec![0; info_len];
    reader
        .read_exact(&mut info_bytes)
        .map_err(|_| FormatError::Damaged("the information is cut short"))?;
    let head = (MAGIC.len() + 1 + line.len() + info_len) as u64;
    let text_len = total
        .checked_sub(head)
        .ok_or(FormatError::Damaged("the file is shorter than its header"))?;
    Ok(DocFile {
        info: DocInfo::from_bytes(&info_bytes),
        text: reader,
        text_len,
    })
}
