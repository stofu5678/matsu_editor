//! A window on a part of a file the scan has not reached (ADR-0020, chapter 2: 「走査していない
//! 場所への移動」).
//!
//! The scan goes through the file from the start, so a place far down is not in the block index
//! yet. To show it anyway, a small read-only document is made over a stretch of the file around
//! the place: [`ScanJob::open_window`]. Only that stretch is read (about [`WINDOW_BYTES`]),
//! never the file between. Its line numbers are counted from the start of the stretch; the
//! caller adds [`ScanWindow::lines_before`], which is exact once the scan has passed the start of
//! the window and an estimate before.

use std::borrow::Cow;
use std::io;
use std::sync::Arc;

use super::scan::{ScanError, ScanJob};
use super::{BlockIndex, EolLens, walk_eols};
use crate::buffer::codec::Codec;
use crate::buffer::storage::EolIndex;
use crate::buffer::{Document, Source};

/// How much of the file a window covers by default (1 MiB). A window of a file of one-byte
/// lines holds a million lines, which the dense line index of a document still takes easily.
pub const WINDOW_BYTES: usize = 1024 * 1024;

/// How far past the wanted start the line ending that begins the window is looked for. A line
/// longer than this gives no window: the place is shown when the scan has reached it.
const LINE_SEARCH: u64 = 256 * 1024;

/// A few bytes past an edge, so that a line ending or a character at the edge is seen whole.
const LOOKAHEAD: u64 = 16;

/// A stretch of the file, `start..end` of the bytes of the source, as a document.
#[derive(Debug)]
pub struct ScanWindow {
    /// The document: read-only, over the stretch. Its byte offsets
    /// ([`Document::pos_to_offset`]) are the bytes of the file minus `start`.
    pub document: Document,
    /// Where the stretch starts in the file: the start of a line.
    pub start: u64,
    /// Where it ends: the end of a line ending, or the end of the file.
    pub end: u64,
    /// How many lines come before the first line of the window in the file.
    pub lines_before: u64,
    /// Whether `lines_before` is exact: the scan has passed `start`. If not it is estimated from
    /// the share of line endings in the part scanned.
    pub exact: bool,
}

/// `start..end` of a source.
#[derive(Debug)]
struct SubSource {
    inner: Arc<dyn Source>,
    start: u64,
    end: u64,
}

impl Source for SubSource {
    fn len(&self) -> u64 {
        self.end - self.start
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        if start > end || end > self.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "range out of bounds",
            ));
        }
        self.inner.read(self.start + start, self.start + end)
    }
}

fn read_all(source: &dyn Source, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
    let bytes = source.read(start, end)?;
    if bytes.len() as u64 != end - start {
        return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
    }
    Ok(bytes)
}

/// The stretch around `target`, from line start to line end, about `size` bytes long, or `None`
/// when a line is in the way (longer than the search or the stretch).
fn find_stretch(
    source: &dyn Source,
    codec: &dyn Codec,
    ext_eol: bool,
    target: u64,
    size: usize,
) -> io::Result<Option<(u64, u64)>> {
    let len = source.len();
    let size = size.max(64) as u64;
    let lens = EolLens::new(codec);
    // 4-byte aligned, so that a UTF-16 or UTF-32 unit is not cut.
    let wanted = target.min(len).saturating_sub(size / 2) & !3;
    let start = if wanted == 0 {
        0
    } else {
        let upto = wanted.saturating_add(LINE_SEARCH + LOOKAHEAD).min(len);
        let bytes = read_all(source, wanted, upto)?;
        // An ending that begins in the search (a CRLF may end past it, within the lookahead).
        let found = super::super::find_eol(codec, &bytes, 0, ext_eol)
            .filter(|&(off, _)| (off as u64) < LINE_SEARCH);
        let Some((off, kind)) = found else {
            return Ok(None);
        };
        wanted + (off + lens.get(kind)) as u64
    };
    if start >= len {
        return Ok(None);
    }
    let wanted_end = start.saturating_add(size).min(len);
    if wanted_end == len {
        return Ok((start <= target.min(len)).then_some((start, len)));
    }
    let upto = (wanted_end + LOOKAHEAD).min(len);
    let bytes = read_all(source, start, upto)?;
    let mut last = None;
    walk_eols(
        codec,
        &lens,
        &bytes,
        0,
        (wanted_end - start) as usize,
        ext_eol,
        |off, n, _| last = Some(off + n),
    );
    // A long line that crosses the target and the end of the window leaves the last line ending
    // before the target: there is no window around it.
    Ok(last
        .map(|e| (start, start + e as u64))
        // `end` is not the end of the file here: the target must be before it (a target at the
        // very start of a long line is not in the window that ends where the line begins).
        .filter(|&(start, end)| (start..end).contains(&target.min(len))))
}

impl BlockIndex {
    /// The number of line endings before byte `pos`, if `pos` is in the scanned part or at the
    /// end of it.
    pub fn breaks_before(&self, pos: u64) -> io::Result<Option<u64>> {
        if pos > self.covered_len() {
            return Ok(None);
        }
        Ok(Some(self.count_before(pos as usize)? as u64))
    }

    /// The byte where line `line` (0-based) starts, if the scan has passed the line ending before
    /// it.
    pub fn line_start_byte(&self, line: u64) -> io::Result<Option<u64>> {
        if line == 0 {
            return Ok(Some(0));
        }
        if line > self.break_count() {
            return Ok(None);
        }
        let entry = self.entry((line - 1) as usize)?;
        Ok(Some((entry.start + entry.len) as u64))
    }
}

impl ScanJob {
    /// A window on the file around byte `target` ([`WINDOW_BYTES`] long), or `None` when the
    /// lines there are too long to cut a window out (the caller waits for the scan then).
    ///
    /// # Errors
    ///
    /// Reading the file failed, or the document could not be made.
    pub fn open_window(&self, target: u64, size: usize) -> Result<Option<ScanWindow>, ScanError> {
        let index = self.index_snapshot();
        let (source, codec, ext_eol) = (index.source(), index.codec(), index.ext_eol());
        let Some((start, end)) = find_stretch(&**source, &**codec, ext_eol, target, size)? else {
            return Ok(None);
        };
        let sub = SubSource {
            inner: source.clone(),
            start,
            end,
        };
        let document = Document::with_ext_eol(sub, codec.clone(), ext_eol)?.into_read_only();
        let (lines_before, exact) = match index.breaks_before(start)? {
            Some(n) => (n, true),
            None => {
                // The share of line endings in the scanned part, applied to the bytes before the
                // window.
                let covered = u128::from(index.covered_len().max(1));
                let n = u128::from(index.break_count()) * u128::from(start) / covered;
                (n as u64, false)
            }
        };
        Ok(Some(ScanWindow {
            document,
            start,
            end,
            lines_before,
            exact,
        }))
    }

    /// How many line endings come before byte `pos`, if the scan has passed it.
    pub fn exact_lines_before(&self, pos: u64) -> Option<u64> {
        self.index_snapshot().breaks_before(pos).ok().flatten()
    }

    /// The byte where line `line` (0-based) starts, if the scan has passed the line ending before
    /// it.
    pub fn line_start_byte(&self, line: u64) -> Option<u64> {
        self.index_snapshot().line_start_byte(line).ok().flatten()
    }
}
