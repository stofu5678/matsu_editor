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
//! * By default only the permissions of the old file are kept. The owner and group can be kept
//!   with [`Preserve::owner_and_group`] (Unix); the other things in [`Preserve`]'s list of future
//!   choices (extended attributes, ACLs, alternate data streams, hard links) are not kept: the new
//!   file replaces the old one (a hard link to the old file keeps the old content). A symbolic
//!   link is followed, so the link stays and its target is replaced.
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
use super::{Progress, READ_CHUNK, TextFile, no_progress};
use crate::buffer::{Chunk, Document, Pos, col_to_utf16, utf16_len};
use crate::encoding::{Encoding, Eol};

/// How many unmappable characters a report lists (the total is always counted).
pub const MAX_REPORTED: usize = 100;

/// What a save keeps of the file it replaces.
///
/// A save writes a temporary file and renames it over the target (ADR-0005), so the file that ends
/// up on disk is a new file. Everything the old file had that is not copied is lost. The
/// permissions (Unix mode, Windows read-only flag) are always copied. Sakura writes in place and so
/// keeps everything; the choices below say what we can add, one by one, and what each costs. Adding
/// one means a new field here, one function that applies it and one call in `write_file`.
///
/// `Preserve::default()` is the default of [`SaveOptions`]: the permissions only. The struct is
/// `non_exhaustive`: start from `Preserve::default()` and use the `with_*` methods.
///
/// ```
/// use matsu_core::file::Preserve;
/// assert!(!Preserve::default().owner_and_group);
/// assert!(Preserve::default().with_owner_and_group(true).owner_and_group);
/// ```
///
/// # Implemented
///
/// * `owner_and_group` (Unix): `chown` the new file to the owner and group of the old one. Only
///   root can give a file to another owner, and a user can only choose a group they belong to, so
///   this is best effort: what the OS refuses is skipped, and the save still succeeds.
///   It does nothing on Windows.
///
/// # Future choices
///
/// * Extended attributes (Linux `xattr`, macOS; includes Finder tags and quarantine flags):
///   needs `listxattr`/`setxattr`, not in the standard library, so a dependency or FFI. Not
///   copying them loses tags and, on Linux, SELinux labels and file capabilities.
/// * ACLs (POSIX ACLs, macOS ACLs, Windows security descriptors): needs OS calls beyond the standard
///   library. On Windows the new file gets the ACL inherited from the folder, which is usually
///   what is wanted, so copying is only needed for files with a hand-made ACL.
/// * Alternate data streams (NTFS): they hold, for example, the "downloaded from the internet"
///   mark. Copying needs `FindFirstStreamW` and a copy per stream; the cost grows with their size.
/// * Writing in place (Sakura's way): keeps the inode, hard links, owner, xattrs, ACLs and streams
///   at once, but a crash or a full disk during the write leaves a half-written file. It would be a
///   different save strategy rather than another flag here, and is the one choice that gives up
///   the safety that ADR-0005 chose.
///
/// # Settings
///
/// A `matsu.ini` key comes with the settings work, in a MATSU-only section (ADR-0010) so that Sakura
/// does not see it: `[MATSU] SavePreserveOwnerGroup=0|1`, and one more key per choice, named
/// `SavePreserve<Choice>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Preserve {
    /// Give the new file the owner and group of the old one (Unix only; best effort).
    pub owner_and_group: bool,
}

impl Preserve {
    /// Sets [`Preserve::owner_and_group`].
    #[must_use]
    pub fn with_owner_and_group(mut self, on: bool) -> Self {
        self.owner_and_group = on;
        self
    }
}

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
    /// What to keep of the file that is replaced, besides its permissions.
    pub preserve: Preserve,
}

impl SaveOptions {
    /// A plain save ("上書き保存"): the encoding and BOM of the file. Line endings are kept as they
    /// are, unless an earlier save converted them ([`TextFile::eol_on_save`]).
    pub fn keep(file: &TextFile) -> Self {
        Self {
            encoding: file.encoding,
            bom: file.bom,
            eol: file.eol_on_save,
            allow_unmappable: false,
            preserve: Preserve::default(),
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
            preserve: Preserve::default(),
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
    /// Counts one more character. `make` builds the entry and only runs while the list is not
    /// full, so a line with very many problems is not scanned again for each of them.
    fn push_with(&mut self, make: impl FnOnce() -> UnmappableChar) {
        self.total += 1;
        if self.first.len() < MAX_REPORTED {
            self.first.push(make());
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
            // Sakura converts the lines of the document itself, so a later plain save keeps the
            // converted endings. We only convert while writing, so remember to do it again.
            self.new_line = eol;
        }
        // The next plain save writes the endings as this one did.
        self.eol_on_save = options.eol;
        Ok(report)
    }
}

impl TextFile {
    /// Whether the document has line endings of more than one kind (`_CheckSavingEolcode` in
    /// `sakura_core/env/CCodeChecker.cpp`). Sakura asks "unify the line endings?" when this is true
    /// and the save does not convert them. It compares every line ending with the first line's.
    pub fn has_mixed_line_endings(&self) -> Result<bool> {
        let mut first = None;
        let mut mixed = false;
        walk_events(
            &self.document,
            self.source_encoding(),
            0,
            &mut no_progress,
            &mut |event| {
                if let Event::Eol(eol) = event
                    && !mixed
                {
                    match first {
                        None => first = Some(eol),
                        Some(f) => mixed = f != eol,
                    }
                }
                Ok(())
            },
        )?;
        Ok(mixed)
    }
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
        let mut done = 0u64;
        'scan: for chunk in doc.chunks() {
            if let Chunk::Added(text) = chunk? {
                for part in slices(&text) {
                    scratch.clear();
                    if codec.encode(part, &mut scratch).is_err() {
                        clean = false;
                        break 'scan;
                    }
                    done += part.len() as u64;
                    // The total is not known before the scan, so it is reported as the amount so far.
                    if progress(done, done).is_break() {
                        return Err(FileError::Cancelled);
                    }
                }
            }
        }
        if clean {
            return Ok(UnmappableReport::default());
        }
    }

    // Decode in bounded slices to find where the problems are. A line is never held whole, so a
    // file without line breaks does not need memory for its whole text.
    let total = total_len(doc)?;
    let mut report = UnmappableReport::default();
    let (mut line, mut col, mut col16) = (0usize, 0usize, 0usize);
    walk_events(doc, source, total, progress, &mut |event| {
        scratch.clear();
        match event {
            Event::Text(text) => {
                if let Err(e) = codec.encode(text, &mut scratch) {
                    for offset in e.offsets {
                        if let Some(ch) = text.get(offset..).and_then(|s| s.chars().next()) {
                            report.push_with(|| UnmappableChar {
                                pos: Pos::new(line, col + offset),
                                col_utf16: col16 + col_to_utf16(text, offset),
                                ch,
                            });
                        }
                    }
                }
                col += text.len();
                col16 += utf16_len(text);
            }
            Event::Eol(kind) => {
                let eol = options.eol.unwrap_or(kind);
                if codec.encode(eol.as_str(), &mut scratch).is_err() {
                    report.push_with(|| UnmappableChar {
                        pos: Pos::new(line, col),
                        col_utf16: col16,
                        ch: eol.as_str().chars().next().unwrap_or('\n'),
                    });
                }
                line += 1;
                col = 0;
                col16 = 0;
            }
        }
        Ok(())
    })?;
    Ok(report)
}

/// A piece of the text of the document: text without line endings, or one line ending.
enum Event<'a> {
    Text(&'a str),
    Eol(Eol),
}

/// Finds line endings in text that arrives in slices (a CR at the end of a slice may be joined
/// with an LF at the start of the next). NEL, LS and PS are line endings only when `ext_eol` is
/// set (the setting of the document, `m_bEnableExtEol`); the added text of a document is UTF-8
/// whatever the file's encoding is, and the codecs that cannot decode to them never produce them.
#[derive(Default)]
struct EolScanner {
    pending_cr: bool,
    ext_eol: bool,
}

impl EolScanner {
    fn scan<'a>(
        &mut self,
        mut text: &'a str,
        f: &mut dyn FnMut(Event<'a>) -> Result<()>,
    ) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if self.pending_cr {
            self.pending_cr = false;
            if let Some(rest) = text.strip_prefix('\n') {
                f(Event::Eol(Eol::CrLf))?;
                text = rest;
            } else {
                f(Event::Eol(Eol::Cr))?;
            }
        }
        let bytes = text.as_bytes();
        let (mut start, mut i) = (0, 0);
        while i < bytes.len() {
            let (kind, len) = match bytes[i] {
                b'\n' => (Eol::Lf, 1),
                b'\r' => match bytes.get(i + 1) {
                    Some(b'\n') => (Eol::CrLf, 2),
                    Some(_) => (Eol::Cr, 1),
                    None => {
                        // The last unit of the slice: the next slice decides.
                        if i > start {
                            f(Event::Text(&text[start..i]))?;
                        }
                        self.pending_cr = true;
                        return Ok(());
                    }
                },
                0xC2 if self.ext_eol && bytes.get(i + 1) == Some(&0x85) => (Eol::Nel, 2),
                0xE2 if self.ext_eol && bytes.get(i + 1) == Some(&0x80) => match bytes.get(i + 2) {
                    Some(0xA8) => (Eol::Ls, 3),
                    Some(0xA9) => (Eol::Ps, 3),
                    _ => {
                        i += 1;
                        continue;
                    }
                },
                _ => {
                    i += 1;
                    continue;
                }
            };
            if i > start {
                f(Event::Text(&text[start..i]))?;
            }
            f(Event::Eol(kind))?;
            i += len;
            start = i;
        }
        if start < text.len() {
            f(Event::Text(&text[start..]))?;
        }
        Ok(())
    }

    fn finish(&mut self, f: &mut dyn FnMut(Event<'_>) -> Result<()>) -> Result<()> {
        if std::mem::take(&mut self.pending_cr) {
            f(Event::Eol(Eol::Cr))?;
        }
        Ok(())
    }
}

/// The number of bytes in the pieces of the document (the total for progress reports).
fn total_len(doc: &Document) -> Result<u64> {
    let mut total = 0u64;
    for chunk in doc.chunks() {
        total += match chunk? {
            Chunk::Original(b) => b.len() as u64,
            Chunk::Added(s) => s.len() as u64,
        };
    }
    Ok(total)
}

/// Decodes the document in slices of about `READ_CHUNK` bytes (cut at the character boundaries of
/// the original's encoding, so that decoding a slice gives what decoding the whole would) and
/// hands them to `f` as text and line endings. Reports progress after each slice and stops when
/// asked to, also in the middle of a very long line.
fn walk_events(
    doc: &Document,
    source: Encoding,
    total: u64,
    progress: Progress<'_>,
    f: &mut dyn FnMut(Event<'_>) -> Result<()>,
) -> Result<()> {
    let codec = source.codec();
    let mut scanner = EolScanner::default();
    let mut buf = String::new();
    let mut done = 0u64;
    let mut chunks = doc.chunks();
    while let Some(next) = chunks.next_with_ext_eol() {
        // Each chunk is scanned under the setting its line endings were found with (Sakura writes
        // every line with the ending recorded when it was made, `CDocLine::m_cEol`).
        let (chunk, ext_eol) = next?;
        scanner.ext_eol = ext_eol;
        match chunk {
            Chunk::Original(bytes) => {
                let mut i = 0;
                while i < bytes.len() {
                    let mut j = i;
                    while j < bytes.len() && (j == i || j - i < READ_CHUNK) {
                        j += codec.char_len(&bytes, j).clamp(1, bytes.len() - j);
                    }
                    buf.clear();
                    codec.decode(&bytes[i..j], &mut buf);
                    scanner.scan(&buf, f)?;
                    done += (j - i) as u64;
                    if progress(done, total).is_break() {
                        return Err(FileError::Cancelled);
                    }
                    i = j;
                }
            }
            Chunk::Added(text) => {
                for part in slices(&text) {
                    scanner.scan(part, f)?;
                    done += part.len() as u64;
                    if progress(done, total).is_break() {
                        return Err(FileError::Cancelled);
                    }
                }
            }
        }
    }
    scanner.finish(f)
}

/// Cuts `text` into slices of about `READ_CHUNK` bytes at char boundaries.
fn slices(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let mut n = rest.len().min(READ_CHUNK);
        while !rest.is_char_boundary(n) {
            n -= 1;
        }
        let (head, tail) = rest.split_at(n);
        rest = tail;
        Some(head)
    })
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
///
/// On Unix `mode` (the mode of the file being replaced) is applied at creation, so that the new
/// content is never readable by more users than the old content was, not even for a moment. The
/// umask can only remove bits from it; the exact mode is set before the rename.
fn create_temp(dir: &Path, mode: Option<&fs::Permissions>) -> io::Result<(File, TempFile)> {
    let mut last = None;
    for _ in 0..100 {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!(".matsu-save-{}-{n}.tmp", std::process::id()));
        let mut options = File::options();
        options.write(true).create_new(true);
        #[cfg(unix)]
        if let Some(mode) = mode {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            options.mode(mode.mode());
        }
        #[cfg(not(unix))]
        let _ = mode;
        match options.open(&path) {
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
        let total = total_len(doc)?;
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
                    // One piece can be huge (a big paste, or a whole file of a stateful
                    // encoding), so it is taken in slices to keep the buffers small.
                    for part in slices(&text) {
                        pending.push_str(part);
                        done += part.len() as u64;
                        if pending.len() >= READ_CHUNK {
                            flush(&mut pending, out)?;
                            if progress(done, total).is_break() {
                                return Err(FileError::Cancelled);
                            }
                        }
                    }
                }
            }
        }
        flush(&mut pending, out)?;
        return Ok(substituted);
    }

    let total = total_len(doc)?;
    walk_events(doc, source, total, progress, &mut |event| {
        match event {
            Event::Text(text) => pending.push_str(text),
            Event::Eol(kind) => pending.push_str(options.eol.unwrap_or(kind).as_str()),
        }
        if pending.len() >= READ_CHUNK {
            flush(&mut pending, out)?;
        }
        Ok(())
    })?;
    flush(&mut pending, out)?;
    Ok(substituted)
}

/// Follows symbolic links (also dangling ones) so that the link itself is kept and the file it
/// points to is replaced. A path that does not exist is used as it is.
fn resolve_target(path: &Path) -> io::Result<PathBuf> {
    let mut current = path.to_path_buf();
    for _ in 0..40 {
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let link = fs::read_link(&current)?;
                current = match current.parent() {
                    Some(parent) => parent.join(link),
                    None => link,
                };
            }
            _ => return Ok(current),
        }
    }
    Err(io::Error::other("too many levels of symbolic links"))
}

fn write_file(
    doc: &Document,
    source: Encoding,
    target: &Path,
    options: &SaveOptions,
    progress: Progress<'_>,
) -> Result<SaveReport> {
    let real = resolve_target(target)?;
    let existing = fs::metadata(&real).ok();
    if let Some(meta) = &existing {
        // A folder, a pipe or a device must not be replaced by a regular file.
        if !meta.is_file() {
            return Err(FileError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the target is not a regular file",
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
    let permissions = existing.as_ref().map(fs::Metadata::permissions);
    let (file, mut temp) = create_temp(dir, permissions.as_ref())?;
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
    // The last chance to cancel, before the target is touched. (A short document may have
    // reported no progress at all.)
    if progress(written, written).is_break() {
        return Err(FileError::Cancelled);
    }
    let file = out
        .inner
        .into_inner()
        .map_err(io::IntoInnerError::into_error)?;
    super::sync_file(&file)?;
    drop(file);
    if let Some(meta) = &existing {
        // Before the permissions: `chown` may clear the setuid and setgid bits.
        if options.preserve.owner_and_group {
            copy_owner_and_group(&temp.path, meta);
        }
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

/// Writes `bytes` to `path` the way a save does (ADR-0005): to a new file next to the target,
/// which then replaces it, so a failed write leaves the old file as it was. A symbolic link is
/// followed, and the permissions (and with `preserve` the owner and group) of the replaced file
/// are kept. For bytes that are not the text of a document (the output of Base64 and uudecode).
pub fn write_bytes(path: &Path, bytes: &[u8], preserve: Preserve) -> Result<()> {
    let real = resolve_target(path)?;
    let existing = fs::metadata(&real).ok();
    if let Some(meta) = &existing {
        if !meta.is_file() {
            return Err(FileError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the target is not a regular file",
            )));
        }
        if meta.permissions().readonly() {
            return Err(FileError::ReadOnly);
        }
    }
    let dir = real
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let permissions = existing.as_ref().map(fs::Metadata::permissions);
    let (mut file, mut temp) = create_temp(dir, permissions.as_ref())?;
    file.write_all(bytes)?;
    super::sync_file(&file)?;
    drop(file);
    if let Some(meta) = &existing {
        if preserve.owner_and_group {
            copy_owner_and_group(&temp.path, meta);
        }
        fs::set_permissions(&temp.path, meta.permissions())?;
    }
    fs::rename(&temp.path, &real)?;
    temp.keep = true;
    sync_dir(dir);
    Ok(())
}

/// Gives `new` the owner and group of the old file. What the OS refuses is skipped: only root may
/// change the owner, and a user may only pick a group they are in. If both cannot be set together,
/// the group alone is tried, so a plain user still keeps the group.
#[cfg(unix)]
fn copy_owner_and_group(new: &Path, old: &fs::Metadata) {
    use std::os::unix::fs::{MetadataExt, chown};
    let (uid, gid) = (old.uid(), old.gid());
    if chown(new, Some(uid), Some(gid)).is_err() {
        let _ = chown(new, None, Some(gid));
    }
}

#[cfg(not(unix))]
fn copy_owner_and_group(_new: &Path, _old: &fs::Metadata) {}

/// Asks the OS to record the rename. Best effort: the content is already safe, and not every OS
/// lets a directory be opened.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = super::sync_file(&d);
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// For the tests: creates a temporary file and leaves it to the caller (the folder is removed).
#[cfg(all(test, unix))]
pub(super) fn create_temp_for_test(dir: &Path, mode: Option<&fs::Permissions>) -> (File, PathBuf) {
    let (file, mut temp) = create_temp(dir, mode).unwrap();
    temp.keep = true;
    (file, temp.path.clone())
}
