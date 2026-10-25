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
mod ranged;
mod save;
mod source;
mod sync;
#[cfg(test)]
mod tests;

use std::fs::{self, File};
use std::io::Read;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use error::{FileError, Result};
pub use ranged::{
    CacheStats, ChangedOutside, DEFAULT_BLOCK_SIZE, DEFAULT_CACHE_BLOCKS, RangedFile,
    RangedOptions, is_changed_outside,
};
pub use save::{
    MAX_REPORTED, Preserve, SaveOptions, SaveReport, UnmappableChar, UnmappableReport, write_bytes,
};
pub use sync::sync_file;

use crate::buffer::{Document, EncodingCodec, Eol, ScanError, ScanJob, ScanOptions, Source};
use crate::encoding::{
    DETECT_SAMPLE_LEN, DetectOptions, Detection, Encoding, detect, mime_header_decode_jis,
};
use source::{FileBytes, OffsetSource};

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
    /// Decode the MIME encoded words (`=?ISO-2022-JP?B?...?=`) of an ISO-2022-JP file before
    /// reading it (`bAutoMIMEdecode`, 「MIMEエンコードされたヘッダを デコードする」). Other encodings are
    /// not touched: only `CJis` takes the option (`CJis::JISToUnicode`, `CCodeFactory`).
    pub mime_decode: bool,
    /// Treat NEL (U+0085), LS (U+2028) and PS (U+2029) as line endings (`m_bEnableExtEol`, 「改行コード
    /// NEL,PS,LSを有効にする」). Off by default, as in Sakura: they are then ordinary characters.
    pub ext_eol: bool,
    /// Open a file of this many bytes or more by ranges (ADR-0020): the file is not read whole;
    /// a background scan finds its line endings ([`TextFile::take_scan`]) and the document is
    /// read-only until it is done. `None` (the default) reads every file whole. Encodings that
    /// depend on earlier bytes (ISO-2022-JP, UTF-7) are always read whole.
    pub huge_file_len: Option<u64>,
    /// The sizes and the pace of the scan of a file opened by ranges.
    /// [`ext_eol`](Self::ext_eol) overrides the one in it.
    pub huge_scan: ScanOptions,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            encoding: None,
            detect: DetectOptions::default(),
            default_eol: Eol::platform_default(),
            max_len: None,
            mime_decode: false,
            ext_eol: false,
            huge_file_len: None,
            huge_scan: ScanOptions::default(),
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
    /// The scan of a file opened by ranges, while it runs ([`TextFile::take_scan`]).
    scan: Option<ScanJob>,
    /// The document reads its file by ranges (ADR-0020), whether or not the scan still runs.
    ranged: bool,
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
            scan: None,
            ranged: false,
        }
    }

    /// A document brought back from a copy of unsaved text (ADR-0022): its text is UTF-8, whatever `encoding` a save
    /// writes, so nothing of it is copied as it is to a file of another encoding.
    pub fn restored(
        document: Document,
        path: Option<PathBuf>,
        encoding: Encoding,
        bom: bool,
        new_line: Eol,
    ) -> Self {
        Self {
            document,
            path,
            encoding,
            bom,
            new_line,
            eol_on_save: None,
            source_encoding: Encoding::Utf8,
            detection: None,
            scan: None,
            ranged: false,
        }
    }

    /// The background scan of a file opened by ranges, if it is still running when the file is
    /// opened. The document of the file is read-only until the scan is done: the owner keeps the
    /// job, shows its progress, and replaces the document with [`ScanJob::snapshot`] now and then
    /// and with [`ScanJob::finish`] at the end.
    pub fn take_scan(&mut self) -> Option<ScanJob> {
        self.scan.take()
    }

    /// Whether the document reads the file by ranges (ADR-0020): the file stays open behind the
    /// document, and is not read whole.
    pub fn is_ranged(&self) -> bool {
        self.ranged
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
    if let Some(from) = options.huge_file_len
        && len >= from
        && len > 0
    {
        drop(file);
        if let Some(opened) = open_ranged(path, options, progress)? {
            return Ok(opened);
        }
        file = File::open(path)?;
    }
    let mut bytes = read_all(&mut file, len, max, progress)?;
    drop(file);

    let (encoding, bom, detection) = match options.encoding {
        Some(encoding) => (encoding, encoding.bom_len(&bytes) > 0, None),
        None => {
            let d = detect(&bytes, &options.detect);
            (d.encoding, d.bom, Some(d))
        }
    };
    // `CFileLoad::FileOpen` hands the option to the code base of the encoding, and only ISO-2022-JP
    // uses it. The detection above looked at the bytes as they are in the file, as Sakura's does.
    if options.mime_decode && encoding == Encoding::Jis {
        bytes = mime_header_decode_jis(&bytes);
    }
    let bom_len = if bom { encoding.bom().len() } else { 0 };
    // The first line ending, read from the bytes: `Document::line` would decode the whole first
    // line, which is the whole file when it has no line break.
    let first_eol = if encoding.codec().is_stateless() {
        crate::buffer::find_eol(encoding.codec(), &bytes[bom_len..], 0, options.ext_eol)
            .map(|(_, eol)| eol)
    } else {
        None
    };
    let source = FileBytes::new(Arc::new(bytes), bom_len);
    let document =
        Document::with_ext_eol(source, Arc::new(EncodingCodec(encoding)), options.ext_eol)?;

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
        scan: None,
        ranged: false,
    })
}

/// How long [`open_ranged`] waits for the scan to cover the first block: the first screen is
/// shown from it.
const FIRST_BLOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

fn scan_to_file_error(error: ScanError) -> FileError {
    match error {
        ScanError::Cancelled => FileError::Cancelled,
        ScanError::Io(e) => FileError::Io(e),
        ScanError::Buffer(e) => FileError::from(e),
        ScanError::Stateful => FileError::Io(std::io::Error::other(error.to_string())),
    }
}

/// Opens `path` by ranges (ADR-0020, chapter 7): the file is read only at its start (for the
/// encoding and the first line ending), a [`ScanJob`] finds the line endings in the background,
/// and the document shows what is scanned and is read-only until it is done. `None` when the
/// file has to be read whole: an encoding that depends on earlier bytes, or nothing after the BOM.
fn open_ranged(
    path: &Path,
    options: &OpenOptions,
    progress: Progress<'_>,
) -> Result<Option<TextFile>> {
    let ranged = RangedFile::open(path)?;
    let len = ranged.len();
    let head = ranged
        .read(0, len.min(DETECT_SAMPLE_LEN as u64))?
        .into_owned();
    let (encoding, bom, detection) = match options.encoding {
        Some(encoding) => (encoding, encoding.bom_len(&head) > 0, None),
        None => {
            let d = detect(&head, &options.detect);
            (d.encoding, d.bom, Some(d))
        }
    };
    if !encoding.codec().is_stateless() {
        return Ok(None);
    }
    let bom_len = if bom { encoding.bom().len() } else { 0 };
    let source = Arc::new(OffsetSource::new(ranged, bom_len as u64));
    if source.len() == 0 {
        return Ok(None);
    }
    // The first line ending, as the whole read finds it. A first line longer than the probe has
    // none here and takes the default.
    let probe = source.read(0, source.len().min(PROBE_LEN))?;
    let first_eol =
        crate::buffer::find_eol(encoding.codec(), &probe, 0, options.ext_eol).map(|(_, eol)| eol);
    drop(probe);
    // The cancel of the caller is honored here, before a thread is started.
    if progress(len, len).is_break() {
        return Err(FileError::Cancelled);
    }
    let scan_options = ScanOptions {
        ext_eol: options.ext_eol,
        ..options.huge_scan
    };
    let job = ScanJob::spawn(source, Arc::new(EncodingCodec(encoding)), scan_options)
        .map_err(scan_to_file_error)?;
    let waited = std::time::Instant::now();
    while job.progress().scanned == 0 && !job.is_finished() && waited.elapsed() < FIRST_BLOCK_WAIT {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // A scan that is done already (a small file) gives the complete document at once.
    let (document, scan) = if job.is_finished() {
        (job.finish().map_err(scan_to_file_error)?, None)
    } else {
        let document = job.snapshot().map_err(scan_to_file_error)?;
        // The scan may have ended since `is_finished` was asked: a complete document can be
        // edited, so the job must not be kept to put the same text over an edit later.
        let scan = document.is_read_only().then_some(job);
        (document, scan)
    };
    let fallback = if encoding == options.detect.default_encoding {
        options.default_eol
    } else {
        Eol::platform_default()
    };
    Ok(Some(TextFile {
        document,
        path: Some(path.to_path_buf()),
        encoding,
        bom,
        new_line: first_eol.unwrap_or(fallback),
        eol_on_save: None,
        source_encoding: encoding,
        detection,
        scan,
        ranged: true,
    }))
}

/// How much of the start is looked at for the first line ending.
const PROBE_LEN: u64 = 1024 * 1024;

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
