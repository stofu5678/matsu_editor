//! Saving: check, write a temporary file, replace the target.
//!
//! Saving is where user data can be lost (ADR-0014). The order is chosen so that nothing is
//! touched until everything that can be checked has been checked:
//!
//! 1. Refuse a read-only target and a directory.
//! 2. Unless the caller allows it, find the characters the encoding cannot write and refuse
//!    ([`FileError::Unmappable`]) before any file is created.
//! 3. Write the whole content to a new temporary file in the folder of the target (the same file
//!    system, so that the last step is a rename), flush it and ask the OS to put it on disk.
//! 4. Copy the permissions of the old file, then rename the temporary file over the target. The
//!    rename is atomic where the OS says so (POSIX `rename`; `MoveFileExW` with replace on Windows,
//!    which `std::fs::rename` uses). If any step fails the temporary file is removed and the old
//!    file is as it was.
//!
//! # What this does not do
//!
//! * The owner and group of the old file, its extended attributes, ACLs, alternate data streams
//!   and hard links are not kept: the new file replaces the old one (a hard link to the old file
//!   keeps the old content). A symbolic link is followed, so the link stays and its target is replaced.
//! * It does not lock the file or detect that another program changed it after it was opened.
//! * It does not make a backup (Sakura's backup options are another feature).
//! * The check and the replace are not atomic with each other: a file that becomes read-only
//!   between them is replaced anyway if the folder allows it.
//!
//! # Bytes that are copied and bytes that are written
//!
//! If the target encoding is the encoding the document was read in and the line endings are not
//! converted, the unedited parts of the original file are copied byte for byte ([`Chunk::Original`])
//! and only the text added by editing is encoded. Invalid bytes in the original therefore survive.
//!
//! Otherwise (another encoding, or line endings converted) every line is decoded and encoded
//! again. The 256 stand-ins for invalid bytes (`INVALID_BYTE_BASE`) are then written as their raw
//! byte, which is what Sakura does too (`CCodeBase::TextToBin`): the bytes survive, but they are
//! now bytes of the new encoding and usually invalid there as well. They are not reported as
//! unmappable.
//!
//! [`Chunk::Original`]: crate::buffer::Chunk::Original

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::error::{FileError, Result};
use super::{Progress, READ_CHUNK, TextFile};
use crate::buffer::{Chunk, Document, Pos, col_to_utf16, utf16_len};
use crate::encoding::{Encoding, Eol};

/// How many unmappable characters a report lists (the total is always counted).
pub const MAX_REPORTED: usize = 100;

/// How lines are counted between two progress calls.
const LINES_PER_PROGRESS: usize = 4096;

/// How to save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveOptions {
    /// The encoding to write.
    pub encoding: Encoding,
    /// Write the BOM. Ignored for encodings that have none (`Encoding::bom` is empty).
    pub bom: bool,
    /// Convert every line ending to this one while writing. `None` keeps them as they are
    /// (a plain save in Sakura). The last line, which has no ending, stays without one.
    pub eol: Option<Eol>,
    /// Save even if characters cannot be written in `encoding`: they become `?` (Sakura's "yes").
    pub allow_unmappable: bool,
}

impl SaveOptions {
    /// A plain save ("上書き保存"): the encoding and BOM of the file, line endings as they are.
    pub fn keep(file: &TextFile) -> Self {
        Self {
            encoding: file.encoding,
            bom: file.bom,
            eol: None,
            allow_unmappable: false,
        }
    }

    /// Save in another encoding. The BOM is on or off as Sakura's `m_bIsBomDefOn` says for it
    /// (`CDocFileOperation::FileSaveAs`).
    pub fn for_encoding(encoding: Encoding) -> Self {
        Self {
            encoding,
            bom: encoding.bom_default_on(),
            eol: None,
            allow_unmappable: false,
        }
    }
}

/// A character that the encoding cannot write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnmappableChar {
    /// Where it is. `col` is a byte column of the decoded text of the line. A line ending that
    /// cannot be written is at the end of the text of its line.
    pub pos: Pos,
    /// The column in UTF-16 units, which is what Sakura shows (`CCodeChecker::OnCheckSave`).
    pub col_utf16: usize,
    pub ch: char,
}

/// The characters that cannot be written, found before anything is written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnmappableReport {
    /// How many characters there are in all.
    pub total: usize,
    /// The first [`MAX_REPORTED`] of them, in document order.
    pub first: Vec<UnmappableChar>,
}

impl UnmappableReport {
    fn push(&mut self, c: UnmappableChar) {
        self.total += 1;
        if self.first.len() < MAX_REPORTED {
            self.first.push(c);
        }
    }
}

/// What a save did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveReport {
    pub bytes_written: u64,
    /// How many characters were written as `?` because the encoding cannot write them (only with
    /// `allow_unmappable`; Sakura's `SAVED_LOSESOME`).
    pub substituted: usize,
}

impl TextFile {
    /// Finds the characters that `options` cannot write. Nothing is written. Call it to ask the
    /// user before [`TextFile::save`] (which does the same check itself and refuses).
    pub fn check_save(
        &self,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> Result<UnmappableReport> {
        check(&self.document, self.source_encoding(), options, progress)
    }

    /// Saves to the path of the file. Fails with [`FileError::NoPath`] for a new document.
    pub fn save(&mut self, options: &SaveOptions, progress: Progress<'_>) -> Result<SaveReport> {
        let path = self.path.clone().ok_or(FileError::NoPath)?;
        self.save_as(path, options, progress)
    }

    /// Saves to `path`. When it succeeds the document takes the path, the encoding, the BOM and
    /// the line ending of `options` for later saves (`CSaveAgent::OnSave`). When it fails, nothing
    /// of the document changes.
    pub fn save_as(
        &mut self,
        path: impl Into<PathBuf>,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> Result<SaveReport> {
        let path = path.into();
        let report = write_file(
            &self.document,
            self.source_encoding(),
            &path,
            options,
            progress,
        )?;
        self.path = Some(path);
        self.encoding = options.encoding;
        self.bom = options.bom;
        if let Some(eol) = options.eol {
            self.new_line = eol;
        }
        Ok(report)
    }
}

/// Whether the document has line endings of more than one kind (`_CheckSavingEolcode` in
/// `sakura_core/env/CCodeChecker.cpp`). Sakura asks "unify the line endings?" when this is true and
/// the save does not convert them. It compares every line ending with the first line's.
pub fn has_mixed_line_endings(doc: &Document) -> Result<bool> {
    let mut first = None;
    for (i, line) in doc.lines(0..usize::MAX).enumerate() {
        let eol = line?.eol;
        if i == 0 {
            first = eol;
        } else if eol.is_some() && eol != first {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether the unedited original bytes can be copied as they are.
fn copies_original(source: Encoding, options: &SaveOptions) -> bool {
    options.encoding == source && options.eol.is_none()
}

fn check(
    doc: &Document,
    source: Encoding,
    options: &SaveOptions,
    progress: Progress<'_>,
) -> Result<UnmappableReport> {
    let codec = options.encoding.codec();
    let mut scratch = Vec::new();
    if copies_original(source, options) {
        // Only added text is encoded, and the original is in the target encoding. If none of
        // the added text is a problem there is nothing to report and no need to decode the file.
        let mut clean = true;
        for chunk in doc.chunks() {
            if let Chunk::Added(text) = chunk? {
                scratch.clear();
                if codec.encode(&text, &mut scratch).is_err() {
                    clean = false;
                    break;
                }
            }
        }
        if clean {
            return Ok(UnmappableReport::default());
        }
    }

    // Decode line by line to find where the problems are.
    let mut report = UnmappableReport::default();
    let line_count = doc.line_count();
    for (i, line) in doc.lines(0..usize::MAX).enumerate() {
        let line = line?;
        scratch.clear();
        if let Err(e) = codec.encode(&line.text, &mut scratch) {
            for offset in e.offsets {
                if let Some(ch) = line.text.get(offset..).and_then(|s| s.chars().next()) {
                    report.push(UnmappableChar {
                        pos: Pos::new(i, offset),
                        col_utf16: col_to_utf16(&line.text, offset),
                        ch,
                    });
                }
            }
        }
        if let Some(eol) = line.eol.map(|e| options.eol.unwrap_or(e)) {
            scratch.clear();
            if codec.encode(eol.as_str(), &mut scratch).is_err() {
                report.push(UnmappableChar {
                    pos: Pos::new(i, line.text.len()),
                    col_utf16: utf16_len(&line.text),
                    ch: eol.as_str().chars().next().unwrap_or('\n'),
                });
            }
        }
        if i % LINES_PER_PROGRESS == 0 && progress(i as u64, line_count as u64).is_break() {
            return Err(FileError::Cancelled);
        }
    }
    Ok(report)
}

/// Counts the bytes that pass through.
struct Counting<W> {
    inner: W,
    written: u64,
}

impl<W: Write> Write for Counting<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A temporary file that is removed when dropped, unless it was renamed away.
struct TempFile {
    path: PathBuf,
    keep: bool,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Creates a new file in `dir` that did not exist (`create_new`, so another file is never
/// overwritten by accident).
fn create_temp(dir: &Path) -> io::Result<(File, TempFile)> {
    let mut last = None;
    for _ in 0..100 {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!(".ume-save-{}-{n}.tmp", std::process::id()));
        match File::options().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, TempFile { path, keep: false })),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("cannot create a temporary file")))
}

/// Writes the content of the document into `out`. Returns how many characters became `?`.
fn write_body(
    doc: &Document,
    source: Encoding,
    options: &SaveOptions,
    out: &mut dyn Write,
    progress: Progress<'_>,
) -> Result<usize> {
    let codec = options.encoding.codec();
    let mut substituted = 0;
    let mut pending = String::new();
    let mut scratch = Vec::new();
    let mut flush = |pending: &mut String, out: &mut dyn Write| -> Result<()> {
        if pending.is_empty() {
            return Ok(());
        }
        scratch.clear();
        if let Err(e) = codec.encode(pending, &mut scratch) {
            substituted += e.offsets.len();
        }
        out.write_all(&scratch)?;
        pending.clear();
        Ok(())
    };

    if copies_original(source, options) {
        let mut total = 0u64;
        for chunk in doc.chunks() {
            total += match chunk? {
                Chunk::Original(b) => b.len() as u64,
                Chunk::Added(s) => s.len() as u64,
            };
        }
        let mut done = 0u64;
        for chunk in doc.chunks() {
            match chunk? {
                Chunk::Original(bytes) => {
                    // Text added just before is written first (it comes before in the document).
                    flush(&mut pending, out)?;
                    for part in bytes.chunks(READ_CHUNK) {
                        out.write_all(part)?;
                        done += part.len() as u64;
                        if progress(done, total).is_break() {
                            return Err(FileError::Cancelled);
                        }
                    }
                }
                Chunk::Added(text) => {
                    done += text.len() as u64;
                    pending.push_str(&text);
                    if pending.len() >= READ_CHUNK {
                        flush(&mut pending, out)?;
                        if progress(done, total).is_break() {
                            return Err(FileError::Cancelled);
                        }
                    }
                }
            }
        }
        flush(&mut pending, out)?;
        return Ok(substituted);
    }

    let line_count = doc.line_count();
    for (i, line) in doc.lines(0..usize::MAX).enumerate() {
        let line = line?;
        pending.push_str(&line.text);
        if let Some(eol) = line.eol.map(|e| options.eol.unwrap_or(e)) {
            pending.push_str(eol.as_str());
        }
        if pending.len() >= READ_CHUNK {
            flush(&mut pending, out)?;
        }
        if i % LINES_PER_PROGRESS == 0 && progress(i as u64, line_count as u64).is_break() {
            return Err(FileError::Cancelled);
        }
    }
    flush(&mut pending, out)?;
    Ok(substituted)
}

/// Follows symbolic links so that the link itself is kept and its target is replaced. A path
/// that does not exist is used as it is.
fn resolve_target(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn write_file(
    doc: &Document,
    source: Encoding,
    target: &Path,
    options: &SaveOptions,
    progress: Progress<'_>,
) -> Result<SaveReport> {
    let real = resolve_target(target);
    let existing = fs::metadata(&real).ok();
    if let Some(meta) = &existing {
        if meta.is_dir() {
            return Err(FileError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the target is a directory",
            )));
        }
        if meta.permissions().readonly() {
            return Err(FileError::ReadOnly);
        }
    }

    if !options.allow_unmappable {
        let report = check(doc, source, options, &mut *progress)?;
        if report.total > 0 {
            return Err(FileError::Unmappable(report));
        }
    }

    let dir = real
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let (file, mut temp) = create_temp(dir)?;
    let mut out = Counting {
        inner: BufWriter::with_capacity(READ_CHUNK, file),
        written: 0,
    };
    if options.bom {
        out.write_all(options.encoding.bom())?;
    }
    let substituted = write_body(doc, source, options, &mut out, progress)?;
    out.flush()?;
    let written = out.written;
    let file = out
        .inner
        .into_inner()
        .map_err(io::IntoInnerError::into_error)?;
    file.sync_all()?;
    drop(file);
    if let Some(meta) = &existing {
        fs::set_permissions(&temp.path, meta.permissions())?;
    }
    fs::rename(&temp.path, &real)?;
    temp.keep = true;
    sync_dir(dir);
    Ok(SaveReport {
        bytes_written: written,
        substituted,
    })
}

/// Asks the OS to record the rename. Best effort: the content is already safe, and not every OS
/// lets a directory be opened.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}
