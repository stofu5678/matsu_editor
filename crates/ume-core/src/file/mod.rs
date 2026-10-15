//! Opening and saving files in their own encoding.
//!
//! [`open_file`] reads a file, finds its encoding and BOM and builds a [`Document`] over the
//! file's own bytes (ADR-0005). [`TextFile::save`] and [`TextFile::save_as`] write the document
//! to a temporary file next to the target and replace the target with it. Saving is where user
//! data can be lost, so the code is conservative; the risks are listed at the top of `save.rs`.
//!
//! # What Sakura does, and where we differ
//!
//! * **Size limit.** 64-bit Sakura has none (`CFileLoad::GetLimitSize` returns `ULLONG_MAX` on
//!   `_WIN64`, `CFileLoad::IsLoadableSize`, `sakura_core/io/CFileLoad.cpp`), and neither do we
//!   (ADR-0012): memory is the bound, and a file that does not fit fails with an I/O error
//!   (`OutOfMemory`). A caller may set [`OpenOptions::max_len`]; the size is then checked before
//!   the file is read ([`FileError::TooLarge`]). Where `usize` is 32 bits, a file over 4 GiB is
//!   refused with `TooLarge`.
//! * **Encoding.** The caller may force an encoding ("文字コード指定" of the open dialog), else it is
//!   detected ([`crate::encoding::detect`], `CCodeMediator::CheckKanjiCodeOfFile`,
//!   `sakura_core/env/CReadManager.cpp`).
//! * **BOM.** `CFileLoad::FileOpen` decodes the first bytes and looks for U+FEFF. The BOM is removed
//!   from the text and remembered ([`TextFile::bom`]); saving writes it again only when asked and
//!   only for encodings that have one (`CWriteManager::WriteFile_From_CDocLineMgr`).
//! * **Line ending for new lines.** `CDocEditor::OnAfterLoad` takes the line ending of the first
//!   line when it has one. Otherwise it is the default of the document type when the file has the
//!   default encoding, and the line ending of the OS when not ([`OpenOptions::default_eol`]).
//! * **Save.** Sakura writes straight into the target file. We write a temporary file and replace
//!   the target (ADR-0005), so a failure or a cancel leaves the old file as it was.
//! * **Characters the encoding cannot write.** Sakura checks before saving
//!   (`CCodeChecker::OnCheckSave`) and asks "yes (save anyway) / no / cancel"; saving anyway writes
//!   `?`. [`TextFile::check_save`] reports them and [`SaveOptions::allow_unmappable`] is "yes".
//! * **Line ending conversion.** "名前を付けて保存" can convert every line ending
//!   (`CDocVisitor::SetAllEol` called from `CSaveAgent::OnBeforeSave`); a plain save does not
//!   (`CDocFileOperation::FileSave` sets `EEolType::none`). Sakura changes the document and puts
//!   the change on the undo stack; we convert while writing and leave the document alone, and
//!   remember the choice in [`TextFile::eol_on_save`] so that a later plain save does not undo it
//!   (a line ending typed or pasted afterwards is converted too, which Sakura would not do).
//!   [`TextFile::has_mixed_line_endings`] is `_CheckSavingEolcode`, the check that makes Sakura ask whether
//!   to unify.

mod error;
mod save;
mod source;
#[cfg(test)]
mod tests;

use std::fs::{self, File};
use std::io::Read;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use error::{FileError, Result};
pub use save::{MAX_REPORTED, SaveOptions, SaveReport, UnmappableChar, UnmappableReport};

use crate::buffer::{Document, EncodingCodec, Eol};
use crate::encoding::{DetectOptions, Detection, Encoding, detect};
use source::FileBytes;

/// The size of one read while loading, and the unit of progress and cancellation.
const READ_CHUNK: usize = 1024 * 1024;

/// Called with `(done, total)` while a long operation runs. Return `ControlFlow::Break(())` to
/// cancel it. The operation then returns [`FileError::Cancelled`] and leaves nothing behind.
pub type Progress<'a> = &'a mut dyn FnMut(u64, u64) -> ControlFlow<()>;

/// A progress callback that never cancels.
pub fn no_progress(_done: u64, _total: u64) -> ControlFlow<()> {
    ControlFlow::Continue(())
}

/// How to open a file.
#[derive(Clone, Debug)]
pub struct OpenOptions {
    /// Read the file in this encoding instead of detecting it.
    pub encoding: Option<Encoding>,
    /// The settings of the detection (`SEncodingConfig`).
    pub detect: DetectOptions,
    /// The line ending for new lines when the file has none and has the default encoding
    /// (`m_eDefaultEoltype`; Sakura's default is CR LF, here the line ending of the OS).
    pub default_eol: Eol,
    /// Refuse files larger than this many bytes. `None` (the default) means no limit, as in
    /// 64-bit Sakura (ADR-0012).
    pub max_len: Option<u64>,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            encoding: None,
            detect: DetectOptions::default(),
            default_eol: Eol::platform_default(),
            max_len: None,
        }
    }
}

/// A document together with what saving it needs.
#[derive(Debug)]
pub struct TextFile {
    pub document: Document,
    /// Where the file is, or `None` for a new document.
    pub path: Option<PathBuf>,
    /// The encoding a plain save writes.
    pub encoding: Encoding,
    /// Whether a plain save writes the BOM.
    pub bom: bool,
    /// The line ending for new lines the user types.
    pub new_line: Eol,
    /// Convert every line ending to this one on a plain save. Set by a save that converted them
    /// (see the note on line ending conversion above); `None` keeps the endings as they are.
    pub eol_on_save: Option<Eol>,
    /// The encoding of the original bytes behind the document. Unedited parts of them are copied
    /// as they are when the target has the same encoding. It does not change when the file is
    /// saved in another encoding, because the document still refers to the bytes it was read from.
    source_encoding: Encoding,
    /// What the encoding detection said (`None` when the encoding was given).
    pub detection: Option<Detection>,
}

impl TextFile {
    /// A new, empty document that has no file yet.
    pub fn new_untitled(encoding: Encoding, bom: bool, new_line: Eol) -> Self {
        Self {
            document: Document::empty(),
            path: None,
            encoding,
            bom,
            new_line,
            eol_on_save: None,
            source_encoding: Encoding::Utf8,
            detection: None,
        }
    }

    /// The encoding of the bytes the document was read from.
    pub fn source_encoding(&self) -> Encoding {
        self.source_encoding
    }
}

/// Opens a file. Same as [`open_file_with`] without progress.
pub fn open_file(path: impl AsRef<Path>, options: &OpenOptions) -> Result<TextFile> {
    open_file_with(path, options, &mut no_progress)
}

/// Opens a file and reports progress while it is read. The size is checked first, so a file over
/// the limit is refused without reading it.
pub fn open_file_with(
    path: impl AsRef<Path>,
    options: &OpenOptions,
    progress: Progress<'_>,
) -> Result<TextFile> {
    let path = path.as_ref();
    let max = options.max_len.unwrap_or(u64::MAX).min(usize::MAX as u64);
    // Look before opening: opening a pipe can block for ever.
    if !fs::metadata(path)?.is_file() {
        return Err(not_regular());
    }
    let mut file = File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(not_regular());
    }
    let len = meta.len();
    if len > max {
        return Err(FileError::TooLarge { len, max });
    }
    let bytes = read_all(&mut file, len, max, progress)?;
    drop(file);

    let (encoding, bom, detection) = match options.encoding {
        Some(encoding) => (encoding, encoding.bom_len(&bytes) > 0, None),
        None => {
            let d = detect(&bytes, &options.detect);
            (d.encoding, d.bom, Some(d))
        }
    };
    let bom_len = if bom { encoding.bom().len() } else { 0 };
    // The first line ending, read from the bytes: `Document::line` would decode the whole first
    // line, which is the whole file when it has no line break.
    let first_eol = if encoding.codec().is_stateless() {
        encoding
            .codec()
            .find_eol(&bytes[bom_len..], 0)
            .map(|(_, eol)| eol)
    } else {
        None
    };
    let source = FileBytes::new(Arc::new(bytes), bom_len);
    let document = Document::new(source, Arc::new(EncodingCodec(encoding)))?;

    // `CDocEditor::OnAfterLoad`: the first line's ending, else the default.
    let fallback = if encoding == options.detect.default_encoding {
        options.default_eol
    } else {
        Eol::platform_default()
    };
    let first_eol = if encoding.codec().is_stateless() {
        first_eol
    } else {
        // A stateful encoding was converted to text at load time.
        document.line(0)?.and_then(|line| line.eol)
    };
    let new_line = first_eol.unwrap_or(fallback);

    Ok(TextFile {
        document,
        path: Some(path.to_path_buf()),
        encoding,
        bom,
        new_line,
        eol_on_save: None,
        source_encoding: encoding,
        detection,
    })
}

/// Reads `len` bytes in chunks. A file that grows while it is read is refused when it passes `max`.
fn read_all(file: &mut File, len: u64, max: u64, progress: Progress<'_>) -> Result<Vec<u8>> {
    // `len` is within `max`, so it fits `usize`. A file bigger than memory is an error, not an abort.
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len as usize)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::OutOfMemory, e))?;
    let mut chunk = vec![0u8; READ_CHUNK.min(len.max(1) as usize)];
    loop {
        let n = match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        bytes
            .try_reserve(n)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::OutOfMemory, e))?;
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() as u64 > max {
            return Err(FileError::TooLarge {
                len: bytes.len() as u64,
                max,
            });
        }
        if progress(bytes.len() as u64, len.max(bytes.len() as u64)).is_break() {
            return Err(FileError::Cancelled);
        }
    }
    Ok(bytes)
}

fn not_regular() -> FileError {
    FileError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "not a regular file",
    ))
}
