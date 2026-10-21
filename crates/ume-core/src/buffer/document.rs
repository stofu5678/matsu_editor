//! The document: a persistent piece tree over the original bytes and the add buffer.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use super::codec::{Codec, EncodingCodec, Eol};
use super::error::{BufferError, Result};
use super::position::Pos;
use super::storage::{LineIndex, MemorySource, Original, Source, Storages};
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

    /// Whether NEL, LS and PS ended a line in the original bytes, as they were read. The saver
    /// needs it to find the same endings after the setting changed.
    pub(crate) fn loaded_ext_eol(&self) -> bool {
        self.st.load_ext_eol()
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
            doc.root = (e > s).then(|| {
                let piece = doc.st.piece(Store::Add, s, e);
                tree::join(None, piece, None)
            });
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
            let piece = doc.st.piece(Store::Original, 0, len as usize);
            doc.root = Some(tree::join(None, piece, None));
        }
        Ok(doc)
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
            let piece = doc.st.piece(Store::Original, 0, len as usize);
            doc.root = Some(tree::join(None, piece, None));
        }
        doc
    }

    fn build(
        source: impl Source + 'static,
        codec: Arc<dyn Codec>,
        index: LineIndex,
        ext_eol: bool,
    ) -> Self {
        let original = Original {
            source: Box::new(source),
            codec,
            index,
        };
        Self {
            root: None,
            st: Arc::new(Storages::new(original, ext_eol)),
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
    fn brk(&self, k: usize) -> Option<Break> {
        let raw = tree::nth_break(&self.root, k, &self.st)?;
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
        Some(b)
    }

    /// Where line `i` starts (`i` may be the number of breaks: the text after the last break).
    fn line_start(&self, i: usize) -> usize {
        if i == 0 {
            0
        } else {
            self.brk(i - 1).map_or(self.total(), |b| b.end())
        }
    }

    /// Where the text of line `i` ends (before its line ending).
    fn line_text_end(&self, i: usize) -> usize {
        self.brk(i).map_or(self.total(), |b| b.start)
    }

    /// The number of lines. The empty text after the last line ending is not a line, and an empty
    /// document has no lines (Sakura: `sakura_core/doc/logic/CDocLineMgr.cpp`).
    pub fn line_count(&self) -> usize {
        let breaks = self.break_count();
        if self.has_tail() { breaks + 1 } else { breaks }
    }

    /// The number of rows of the text: its line breaks plus one. Unlike [`line_count`](Self::line_count)
    /// it counts the empty row after a final line break (and is 1 for an empty document). It
    /// reads the index only, never the text.
    pub fn row_count(&self) -> usize {
        self.break_count() + 1
    }

    /// Whether there is text after the last line break.
    fn has_tail(&self) -> bool {
        let breaks = self.break_count();
        let end = if breaks == 0 {
            0
        } else {
            self.brk(breaks - 1).map_or(0, |b| b.end())
        };
        self.total() > end
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
            if self.line_start(mid) <= raw {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let start = self.line_start(lo);
        // A raw offset inside a CRLF is at the end of the text of its line.
        let end = raw.min(self.line_text_end(lo));
        let col = self.decoded_len(start, end)?;
        Ok(Pos { line: lo, col })
    }

    fn pos_to_raw(&self, pos: Pos) -> Result<usize> {
        if pos.line >= self.line_count() {
            return Ok(self.total());
        }
        let start = self.line_start(pos.line);
        if pos.col == 0 {
            return Ok(start);
        }
        let end = self.line_text_end(pos.line);
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
        if i >= self.line_count() {
            return Ok(None);
        }
        let start = self.line_start(i);
        let brk = self.brk(i);
        let end = brk.map_or(self.total(), |b| b.start);
        Ok(Some(Line {
            text: self.decode_range(start, end)?,
            eol: brk.map(|b| b.kind),
        }))
    }

    /// The length in bytes of the text of line `i` (the largest `col` there), or `None`.
    pub fn line_len(&self, i: usize) -> Result<Option<usize>> {
        if i >= self.line_count() {
            return Ok(None);
        }
        self.decoded_len(self.line_start(i), self.line_text_end(i))
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
    /// added text as UTF-8. Unedited parts can be copied to the new file unchanged.
    pub fn chunks(&self) -> Chunks<'_> {
        Chunks {
            doc: self,
            pieces: tree::pieces(&self.root).into_iter(),
        }
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
        let (left, rest) = tree::split(&self.root, a, &self.st);
        let (_, right) = tree::split(&rest, b - a, &self.st);
        let root = match added {
            None => tree::concat(left, right),
            Some((s, e)) => Some(match tree::last_piece(&left) {
                // Typing continues the piece that ends where this text starts.
                Some(last) if last.store == Store::Add && last.end == s => {
                    let merged = self.st.piece(Store::Add, last.start, e);
                    let rest = tree::pop_last(&left).and_then(|(rest, _)| rest);
                    tree::join(rest, merged, right)
                }
                _ => tree::join(left, self.st.piece(Store::Add, s, e), right),
            }),
        };
        // Build the new version and read the positions from it before replacing this one, so
        // that a read error leaves the document as it was.
        let next = Self {
            root,
            st: Arc::clone(&self.st),
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

/// An iterator over the pieces of a document. See [`Document::chunks`].
#[derive(Debug)]
pub struct Chunks<'a> {
    doc: &'a Document,
    pieces: std::vec::IntoIter<tree::Piece>,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Result<Chunk<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let p = self.pieces.next()?;
        let doc: &'a Document = self.doc;
        let st = &doc.st;
        Some(match p.store {
            Store::Original => st.read_original(p.start, p.end).map(Chunk::Original),
            Store::Add => st.with_bytes(Store::Add, p.start, p.end, |b| {
                Chunk::Added(String::from_utf8_lossy(b).into_owned())
            }),
        })
    }
}
