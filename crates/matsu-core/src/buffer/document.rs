//! The document: a persistent piece tree over the original bytes and the add buffer.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use super::block::BlockIndex;
use super::codec::{Codec, EncodingCodec, Eol};
use super::error::{BufferError, Result};
use super::position::Pos;
use super::storage::{EolIndex, LineIndex, MemorySource, Original, Source, Storages};
use super::tree::{self, Store, Tree};
use crate::encoding::Encoding;

/// One line: its text without the line ending, and the line ending, if it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub eol: Option<Eol>,
}

/// The result of an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Where the edited range started (clamped, and the smaller of the two ends).
    pub start: Pos,
    /// Where the inserted text ends. For a deletion this is `start`.
    pub end: Pos,
}

/// A piece of the document for the saver: the original bytes as they are, or text added by
/// editing (UTF-8, to be encoded).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chunk<'a> {
    Original(Cow<'a, [u8]>),
    Added(String),
}

/// A line break found in the document.
#[derive(Clone, Copy, Debug)]
struct Break {
    start: usize,
    len: usize,
    kind: Eol,
}

impl Break {
    fn end(&self) -> usize {
        self.start + self.len
    }
}

/// A text document.
///
/// Cloning is cheap: it copies one pointer and shares everything else. An edit makes a new
/// version that shares the unchanged parts with the old one, so clones can be kept for undo or
/// read by another thread while this one goes on editing.
///
/// Text is never returned as one string; read it by lines or by range. Raw offsets (bytes of
/// the pieces, which count the original data in its own encoding) are internal; callers use
/// [`Pos`].
#[derive(Clone, Debug)]
pub struct Document {
    root: Tree,
    st: Arc<Storages>,
    /// Set on a document that shows only the scanned start of a file being scanned
    /// ([`Document::from_block_index`]): edits are refused until the scan is done and the
    /// complete document replaces this one (ADR-0020, chapter 2).
    read_only: bool,
    /// The document was made over a file read by ranges ([`Document::from_block_index`]).
    ranged: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self::empty()
    }
}

impl Document {
    /// An empty document.
    pub fn empty() -> Self {
        Self::from_utf8(Vec::new())
            .unwrap_or_else(|_| unreachable!("an empty source is never too large"))
    }

    /// A document over UTF-8 bytes (invalid bytes are shown as stand-ins and saved unchanged).
    pub fn from_utf8(bytes: impl Into<Arc<[u8]>>) -> Result<Self> {
        Self::from_utf8_with(bytes, false)
    }

    /// Like [`Document::from_utf8`], with the choice of [`Document::with_ext_eol`].
    pub fn from_utf8_with(bytes: impl Into<Arc<[u8]>>, ext_eol: bool) -> Result<Self> {
        Self::with_ext_eol(
            MemorySource::new(bytes),
            Arc::new(EncodingCodec(Encoding::Utf8)),
            ext_eol,
        )
    }

    /// A document over `source`, read through `codec`. There is no size limit, as in 64-bit
    /// Sakura (ADR-0012); memory is the bound. On a target whose `usize` is 32 bits, a source over
    /// 4 GiB is refused ([`BufferError::SourceTooLarge`]).
    ///
    /// NEL (U+0085), LS (U+2028) and PS (U+2029) are ordinary characters here, as in Sakura with
    /// its default setting; see [`Document::with_ext_eol`].
    pub fn new(source: impl Source + 'static, codec: Arc<dyn Codec>) -> Result<Self> {
        Self::open(source, codec, u64::MAX, false)
    }

    /// Like [`Document::new`], with the choice of whether NEL, LS and PS end a line
    /// (`m_bEnableExtEol`, 「改行コードNEL,PS,LSを有効にする」; `WCODE::IsLineDelimiter`). Off, they
    /// are characters of the line. The choice is made for the lines found now; see
    /// [`Document::set_ext_eol`] for the lines made later.
    pub fn with_ext_eol(
        source: impl Source + 'static,
        codec: Arc<dyn Codec>,
        ext_eol: bool,
    ) -> Result<Self> {
        Self::open(source, codec, u64::MAX, ext_eol)
    }

    /// Whether NEL, LS and PS end a line in text added from now on.
    pub fn ext_eol(&self) -> bool {
        self.st.ext_eol()
    }

    /// Changes whether NEL, LS and PS end a line in text added by later edits. The lines that exist
    /// keep their endings and are not split or joined again: Sakura reads the setting where it
    /// sets a line (`CDocLine::SetDocLineString`), so a change reaches the lines made after it and
    /// the files opened after it. It is shared by the clones of the document (undo keeps them).
    pub fn set_ext_eol(&self, ext_eol: bool) {
        self.st.set_ext_eol(ext_eol);
    }

    /// Like [`Document::new`], but refuses a source longer than `max` bytes. For callers and
    /// tests that want a limit; the address space is a limit too (see [`Document::new`]).
    pub fn with_limit(
        source: impl Source + 'static,
        codec: Arc<dyn Codec>,
        max: u64,
    ) -> Result<Self> {
        Self::open(source, codec, max, false)
    }

    fn open(
        source: impl Source + 'static,
        codec: Arc<dyn Codec>,
        max: u64,
        ext_eol: bool,
    ) -> Result<Self> {
        let len = source.len();
        let max = max.min(usize::MAX as u64);
        if len > max {
            return Err(BufferError::SourceTooLarge { len, max });
        }
        if !codec.is_stateless() {
            // The bytes cannot be decoded from the middle, so convert once and keep the text as
            // added text (it will be encoded again when saved).
            let mut text = String::new();
            codec.decode(&source.read(0, len)?, &mut text);
            let mut doc = Self::build(
                MemorySource::new(Vec::new()),
                Arc::new(EncodingCodec(Encoding::Utf8)),
                LineIndex::default(),
                ext_eol,
            );
            let (s, e) = doc.st.append_add(&text)?;
            if e > s {
                let piece = doc.st.piece(Store::Add, s, e)?;
                doc.root = Some(tree::join(None, piece, None));
            }
            // The original is empty now: no piece refers to the file's bytes.
            return Ok(doc);
        }
        let bytes = source.read(0, len)?;
        let mut index = LineIndex::default();
        index.scan(&*codec, &bytes, 0, ext_eol)?;
        index.shrink_to_fit();
        drop(bytes);
        let mut doc = Self::build(source, codec, index, ext_eol);
        if len > 0 {
            let piece = doc.st.piece(Store::Original, 0, len as usize)?;
            doc.root = Some(tree::join(None, piece, None));
        }
        Ok(doc)
    }

    /// A document over a file that was scanned into a block index (ADR-0020, chapter 2). The
    /// source and the codec are the ones the scan used ([`BlockIndex::source`]). Nothing is read
    /// from the source here beyond the ends of the scanned part.
    ///
    /// The document shows the scanned start of the file as one piece. When the index is complete
    /// that is the whole file and the document can be edited. While a scan is still running the
    /// index only covers the start; the document then is read-only
    /// ([`Document::is_read_only`]) and a new one is made when the scan has finished.
    pub fn from_block_index(index: BlockIndex) -> Result<Self> {
        let (source, codec) = (index.source().clone(), index.codec().clone());
        let ext_eol = index.ext_eol();
        if index.covered_len() > source.len() {
            return Err(BufferError::Corrupt);
        }
        let covered = usize::try_from(index.covered_len()).map_err(|_| BufferError::TooLarge)?;
        let read_only = !index.is_complete();
        let mut doc = Self::build(source, codec, index, ext_eol);
        doc.read_only = read_only;
        doc.ranged = true;
        if covered > 0 {
            let piece = doc.st.piece(Store::Original, 0, covered)?;
            doc.root = Some(tree::join(None, piece, None));
        }
        Ok(doc)
    }

    /// Whether the document is over a file that is read by ranges ([`Document::from_block_index`]).
    /// Its lines are cut at about [`DEFAULT_LONG_LINE`](super::DEFAULT_LONG_LINE) bytes, and a
    /// caller that reads many lines can skip the long ones ([`Document::line_raw_len`]).
    pub fn is_ranged(&self) -> bool {
        self.ranged
    }

    /// The length in bytes, as the file has them, of the text of line `i` (without its line
    /// ending), or `None` if there is no such line. It reads the index only, never the text.
    pub fn line_raw_len(&self, i: usize) -> Result<Option<usize>> {
        if i >= self.try_line_count()? {
            return Ok(None);
        }
        Ok(Some(self.line_text_end(i)? - self.line_start(i)?))
    }

    /// Whether edits are refused: the document shows the scanned start of a file that is still
    /// being scanned ([`Document::from_block_index`]).
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// A document over `source` with a line index the caller made, without reading the source.
    /// For tests with sources too big to read (offsets above 4 GiB).
    #[cfg(test)]
    pub(super) fn with_prebuilt_index(
        source: impl Source + 'static,
        codec: Arc<dyn Codec>,
        index: LineIndex,
    ) -> Self {
        let len = source.len();
        let mut doc = Self::build(source, codec, index, true);
        if len > 0 {
            let piece = doc
                .st
                .piece(Store::Original, 0, len as usize)
                .expect("a prebuilt in-memory index does not fail");
            doc.root = Some(tree::join(None, piece, None));
        }
        doc
    }

    fn build(
        source: impl Source + 'static,
        codec: Arc<dyn Codec>,
        index: impl EolIndex + 'static,
        ext_eol: bool,
    ) -> Self {
        let original = Original {
            source: Box::new(source),
            codec,
            index: Box::new(index),
        };
        Self {
            root: None,
            st: Arc::new(Storages::new(original, ext_eol)),
            read_only: false,
            ranged: false,
        }
    }

    // ---- raw offsets ----

    fn total(&self) -> usize {
        tree::agg(&self.root).bytes
    }

    fn break_count(&self) -> usize {
        tree::agg(&self.root).breaks
    }

    /// The `k`-th line break, with a CRLF over two pieces joined.
    fn brk(&self, k: usize) -> Result<Option<Break>> {
        let Some(raw) = tree::nth_break(&self.root, k, &self.st)? else {
            return Ok(None);
        };
        let mut b = Break {
            start: raw.start,
            len: raw.len,
            kind: raw.kind,
        };
        if raw.open_cr
            && let Some((next, _)) = tree::piece_at(&self.root, b.end())
            && next.agg.first_lf > 0
        {
            b.kind = Eol::CrLf;
            b.len += usize::from(next.agg.first_lf);
        }
        Ok(Some(b))
    }

    /// Where line `i` starts (`i` may be the number of breaks: the text after the last break).
    fn line_start(&self, i: usize) -> Result<usize> {
        if i == 0 {
            Ok(0)
        } else {
            Ok(self.brk(i - 1)?.map_or(self.total(), |b| b.end()))
        }
    }

    /// Where the text of line `i` ends (before its line ending).
    fn line_text_end(&self, i: usize) -> Result<usize> {
        Ok(self.brk(i)?.map_or(self.total(), |b| b.start))
    }

    /// The number of lines. The empty text after the last line ending is not a line, and an empty
    /// document has no lines (Sakura: `sakura_core/doc/logic/CDocLineMgr.cpp`).
    ///
    /// Finding out whether text follows the last line ending may read the source. When that read
    /// fails (a file read by ranges that was changed outside), this returns the number of line
    /// endings, which misses the last line when it has no ending; a caller that must tell the
    /// failure uses [`Document::try_line_count`].
    pub fn line_count(&self) -> usize {
        self.try_line_count().unwrap_or_else(|_| self.break_count())
    }

    /// Like [`Document::line_count`], but reports a failure to read the source.
    pub fn try_line_count(&self) -> Result<usize> {
        let breaks = self.break_count();
        Ok(if self.has_tail()? { breaks + 1 } else { breaks })
    }

    /// The number of rows of the text: its line breaks plus one. Unlike [`line_count`](Self::line_count)
    /// it counts the empty row after a final line break (and is 1 for an empty document). It
    /// reads the index only, never the text.
    pub fn row_count(&self) -> usize {
        self.break_count() + 1
    }

    /// Whether there is text after the last line break.
    fn has_tail(&self) -> Result<bool> {
        let breaks = self.break_count();
        let end = if breaks == 0 {
            0
        } else {
            self.brk(breaks - 1)?.map_or(0, |b| b.end())
        };
        Ok(self.total() > end)
    }

    /// The end of the document: `(line_count, 0)` when it is empty or ends with a line ending,
    /// and the end of the last line otherwise.
    pub fn end_pos(&self) -> Result<Pos> {
        self.raw_to_pos(self.total())
    }

    fn raw_to_pos(&self, raw: usize) -> Result<Pos> {
        let breaks = self.break_count();
        // The last line start that is not after `raw`.
        let (mut lo, mut hi) = (0, breaks);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.line_start(mid)? <= raw {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let start = self.line_start(lo)?;
        // A raw offset inside a CRLF is at the end of the text of its line.
        let end = raw.min(self.line_text_end(lo)?);
        let col = self.decoded_len(start, end)?;
        Ok(Pos { line: lo, col })
    }

    fn pos_to_raw(&self, pos: Pos) -> Result<usize> {
        if pos.line >= self.try_line_count()? {
            return Ok(self.total());
        }
        let start = self.line_start(pos.line)?;
        if pos.col == 0 {
            return Ok(start);
        }
        let end = self.line_text_end(pos.line)?;
        self.raw_for_col(start, end, pos.col)
    }

    /// Walks the text `start..end` (document offsets) for `col` decoded bytes and returns the
    /// document offset there, rounded down to a character boundary.
    fn raw_for_col(&self, start: usize, end: usize, col: usize) -> Result<usize> {
        let mut remaining = col;
        let mut found = None;
        let mut scratch = String::new();
        let st = &self.st;
        tree::visit(&self.root, start, end, &mut |piece,
                                                  s,
                                                  e,
                                                  raw|
         -> Result<()> {
            if found.is_some() {
                return Ok(());
            }
            match piece.store {
                Store::Add => {
                    if remaining < e - s {
                        found = Some(st.with_bytes(Store::Add, s, e, |bytes| {
                            let mut c = remaining;
                            while c > 0 && (bytes[c] & 0xC0) == 0x80 {
                                c -= 1;
                            }
                            raw + c
                        })?);
                    } else {
                        remaining -= e - s;
                    }
                }
                Store::Original => {
                    let bytes = st.read_original(s, e)?;
                    let codec = &st.original.codec;
                    let (mut i, mut seen) = (0, 0);
                    while i < bytes.len() {
                        let n = codec.char_len(&bytes, i).clamp(1, bytes.len() - i);
                        scratch.clear();
                        codec.decode(&bytes[i..i + n], &mut scratch);
                        if seen + scratch.len() > remaining {
                            found = Some(raw + i);
                            return Ok(());
                        }
                        seen += scratch.len();
                        i += n;
                    }
                    remaining -= seen;
                }
            }
            Ok(())
        })?;
        Ok(found.unwrap_or(end))
    }

    /// The length of the decoded text of the document bytes `start..end`.
    fn decoded_len(&self, start: usize, end: usize) -> Result<usize> {
        let mut total = 0;
        let mut scratch = String::new();
        tree::visit(&self.root, start, end, &mut |p, s, e, _| -> Result<()> {
            total += self.st.decoded_len(p.store, s, e, &mut scratch)?;
            Ok(())
        })?;
        Ok(total)
    }

    fn decode_range(&self, start: usize, end: usize) -> Result<String> {
        let mut out = String::new();
        tree::visit(&self.root, start, end, &mut |p, s, e, _| -> Result<()> {
            self.st.decode_into(p.store, s, e, &mut out)
        })?;
        Ok(out)
    }

    // ---- reading ----

    /// Line `i` (0-based), or `None` if there is no such line.
    pub fn line(&self, i: usize) -> Result<Option<Line>> {
        if i >= self.try_line_count()? {
            return Ok(None);
        }
        let start = self.line_start(i)?;
        let brk = self.brk(i)?;
        let end = brk.map_or(self.total(), |b| b.start);
        Ok(Some(Line {
            text: self.decode_range(start, end)?,
            eol: brk.map(|b| b.kind),
        }))
    }

    /// The length in bytes of the text of line `i` (the largest `col` there), or `None`.
    pub fn line_len(&self, i: usize) -> Result<Option<usize>> {
        if i >= self.try_line_count()? {
            return Ok(None);
        }
        self.decoded_len(self.line_start(i)?, self.line_text_end(i)?)
            .map(Some)
    }

    /// The lines in `range` (clamped to the document), one at a time.
    pub fn lines(&self, range: Range<usize>) -> Lines<'_> {
        let end = range.end.min(self.line_count());
        Lines {
            doc: self,
            next: range.start,
            end,
        }
    }

    /// The text between two positions, with the line endings as they are in the document.
    /// The positions may be given in either order and are clamped.
    pub fn text_between(&self, a: Pos, b: Pos) -> Result<String> {
        let (a, b) = (self.pos_to_raw(a)?, self.pos_to_raw(b)?);
        self.decode_range(a.min(b), a.max(b))
    }

    /// Clamps a position into the document: a line past the end becomes the end of the document, a
    /// column past the end of the line becomes the end of the text of the line, and a column inside
    /// a character moves back to the start of the character.
    pub fn clamp_pos(&self, pos: Pos) -> Result<Pos> {
        self.raw_to_pos(self.pos_to_raw(pos)?)
    }

    /// The pieces of the document in order, for the saver: the original bytes as they are, and the
    /// added text as UTF-8. Unedited parts can be copied to the new file unchanged. A piece is
    /// returned in several chunks of at most [`CHUNK_LEN`] bytes (cut at character boundaries), so
    /// a piece of any size is never read into one buffer.
    pub fn chunks(&self) -> Chunks<'_> {
        self.chunks_of(chunk_len())
    }

    /// Like [`Document::chunks`] with chunks of at most `max_len` bytes (a chunk always has at
    /// least one character). Small sizes are for tests.
    pub fn chunks_of(&self, max_len: usize) -> Chunks<'_> {
        Chunks {
            doc: self,
            pieces: tree::pieces(&self.root).into_iter(),
            rest: None,
            max_len: max_len.max(1),
        }
    }

    /// The size of the document in bytes as it is held: the original data in its own encoding plus the
    /// added text in UTF-8. A measure of how big the document is, not the size of a saved file.
    pub fn raw_len(&self) -> usize {
        self.total()
    }

    /// The number of pieces (a measure of how fragmented the document is).
    pub fn piece_count(&self) -> usize {
        tree::pieces(&self.root).len()
    }

    /// The heap bytes of the line indexes, for reports.
    pub fn index_heap_bytes(&self) -> usize {
        self.st.index_heap_bytes()
    }

    /// The bytes of added text so far, including text that was edited away again. Clones share
    /// the add buffer, which only grows.
    pub fn added_bytes(&self) -> usize {
        self.st.add_len()
    }

    // ---- editing ----

    /// Inserts `text` at `pos`. Line endings in `text` (CR, LF, CRLF, and U+0085, U+2028, U+2029
    /// when [`Document::ext_eol`] is set) become line breaks as written.
    pub fn insert(&mut self, pos: Pos, text: &str) -> Result<Edit> {
        self.replace(pos, pos, text)
    }

    /// Deletes the text between two positions (in either order).
    pub fn delete(&mut self, from: Pos, to: Pos) -> Result<Edit> {
        self.replace(from, to, "")
    }

    /// Replaces the text between two positions with `text`.
    pub fn replace(&mut self, from: Pos, to: Pos, text: &str) -> Result<Edit> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        let (a, b) = (self.pos_to_raw(from)?, self.pos_to_raw(to)?);
        let (a, b) = (a.min(b), a.max(b));
        if a == b && text.is_empty() {
            let start = self.raw_to_pos(a)?;
            return Ok(Edit { start, end: start });
        }
        // Appending first means a failure leaves the document as it was.
        let added = if text.is_empty() {
            None
        } else {
            Some(self.st.append_add(text)?)
        };
        let (left, rest) = tree::split(&self.root, a, &self.st)?;
        let (_, right) = tree::split(&rest, b - a, &self.st)?;
        let root = match added {
            None => tree::concat(left, right),
            Some((s, e)) => Some(match tree::last_piece(&left) {
                // Typing continues the piece that ends where this text starts.
                // (not across a change of the setting: a piece is scanned under one setting)
                Some(last)
                    if last.store == Store::Add
                        && last.end == s
                        && self.st.add_ext_eol_at(last.start) == self.st.ext_eol() =>
                {
                    let merged = self.st.piece(Store::Add, last.start, e)?;
                    let rest = tree::pop_last(&left).and_then(|(rest, _)| rest);
                    tree::join(rest, merged, right)
                }
                _ => tree::join(left, self.st.piece(Store::Add, s, e)?, right),
            }),
        };
        // Build the new version and read the positions from it before replacing this one, so
        // that a read error leaves the document as it was.
        let next = Self {
            root,
            st: Arc::clone(&self.st),
            read_only: false,
            ranged: self.ranged,
        };
        let start = next.raw_to_pos(a)?;
        let end = if text.is_empty() {
            start
        } else {
            next.raw_to_pos(a + text.len())?
        };
        *self = next;
        Ok(Edit { start, end })
    }

    #[cfg(test)]
    pub(crate) fn check_invariants(&self) {
        tree::check_invariants(&self.root, &self.st);
    }
}

/// An iterator over a range of lines.
#[derive(Debug)]
pub struct Lines<'a> {
    doc: &'a Document,
    next: usize,
    end: usize,
}

impl Iterator for Lines<'_> {
    type Item = Result<Line>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }
        let i = self.next;
        self.next += 1;
        self.doc.line(i).transpose()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end.saturating_sub(self.next);
        (n, Some(n))
    }
}

/// The largest chunk [`Document::chunks`] returns, in bytes (the same as the saver's read size).
pub const CHUNK_LEN: usize = 1024 * 1024;

/// How far past the cut a chunk is read, so that the character at the cut is seen whole (the
/// longest character is 4 bytes, and a UTF-16 high surrogate looks 2 bytes further).
const CUT_MARGIN: usize = 8;

#[cfg(test)]
thread_local! {
    static TEST_CHUNK_LEN: std::cell::Cell<usize> = const { std::cell::Cell::new(CHUNK_LEN) };
}

/// The chunk size `Document::chunks` uses now.
fn chunk_len() -> usize {
    #[cfg(test)]
    return TEST_CHUNK_LEN.with(std::cell::Cell::get);
    #[cfg(not(test))]
    CHUNK_LEN
}

/// Runs `f` with `Document::chunks` returning chunks of at most `len` bytes on this thread.
#[cfg(test)]
pub(crate) fn with_chunk_len<R>(len: usize, f: impl FnOnce() -> R) -> R {
    struct Reset(usize);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_CHUNK_LEN.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(TEST_CHUNK_LEN.with(|c| c.replace(len)));
    f()
}

/// An iterator over the pieces of a document. See [`Document::chunks`].
#[derive(Debug)]
pub struct Chunks<'a> {
    doc: &'a Document,
    pieces: std::vec::IntoIter<tree::Piece>,
    /// What is left of the piece being cut into chunks.
    rest: Option<tree::Piece>,
    max_len: usize,
}

impl<'a> Chunks<'a> {
    /// The next chunk and whether NEL, LS and PS ended lines in it when its line endings were
    /// found (the setting at the time: the original's at load, added text's at the edit). A saver
    /// that finds the endings again must use it, not the setting now.
    pub(crate) fn next_with_ext_eol(&mut self) -> Option<Result<(Chunk<'a>, bool)>> {
        let p = match self.rest.take() {
            Some(p) => p,
            None => self.pieces.next()?,
        };
        let doc: &'a Document = self.doc;
        let st = &doc.st;
        let max = self.max_len;
        let whole = p.end - p.start <= max;
        // Read a few bytes past the cut so the character there is seen whole.
        let end = if whole {
            p.end
        } else {
            p.end
                .min(p.start.saturating_add(max).saturating_add(CUT_MARGIN))
        };
        // Where this chunk ends, as an offset from the start of the piece.
        let cut = std::cell::Cell::new(0usize);
        let result = match p.store {
            Store::Original => st.read_original(p.start, end).map(|bytes| {
                let n = if whole {
                    bytes.len()
                } else {
                    // A piece starts at a character boundary, so walking the characters from
                    // its start finds the boundary at or after `max` bytes.
                    let codec = &st.original.codec;
                    let mut i = 0;
                    while i < max {
                        i += codec.char_len(&bytes, i).clamp(1, bytes.len() - i);
                    }
                    i
                };
                cut.set(n);
                let bytes = match bytes {
                    Cow::Borrowed(b) => Cow::Borrowed(&b[..n]),
                    Cow::Owned(mut v) => {
                        v.truncate(n);
                        Cow::Owned(v)
                    }
                };
                (Chunk::Original(bytes), st.load_ext_eol())
            }),
            Store::Add => {
                // Read before `with_bytes`, which holds the lock of the add buffer.
                let ext_eol = st.add_ext_eol_at(p.start);
                st.with_bytes(Store::Add, p.start, end, |b| {
                    let mut n = if whole { b.len() } else { max };
                    // The add buffer is UTF-8: do not cut inside a character.
                    while n < b.len() && (b[n] & 0xC0) == 0x80 {
                        n += 1;
                    }
                    cut.set(n);
                    (
                        Chunk::Added(String::from_utf8_lossy(&b[..n]).into_owned()),
                        ext_eol,
                    )
                })
            }
        };
        if result.is_ok() && p.start + cut.get() < p.end {
            self.rest = Some(tree::Piece {
                start: p.start + cut.get(),
                ..p
            });
        }
        Some(result)
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Result<Chunk<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_with_ext_eol()?.map(|(chunk, _)| chunk))
    }
}
