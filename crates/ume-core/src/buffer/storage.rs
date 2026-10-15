//! The two storages behind the pieces: the original bytes and the add buffer.

use std::borrow::Cow;
use std::fmt;
use std::io;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::codec::{Codec, Eol, Utf8Codec};
use super::error::{BufferError, Result};
use super::tree::{Agg, Piece, Store};

/// The original data of a document: the bytes of a file in its own encoding.
///
/// Reading a range may need I/O and may fail. A later ranged source (Phase 6) reads parts of a
/// file on demand; [`MemorySource`] holds everything in memory.
pub trait Source: Send + Sync + fmt::Debug {
    /// The length in bytes.
    fn len(&self) -> u64;

    /// Returns the bytes `start..end`. `end` must not be less than `start` or past `len()`.
    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A source held in memory. Cloning shares the bytes.
#[derive(Clone, Debug)]
pub struct MemorySource(Arc<[u8]>);

impl MemorySource {
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self(bytes.into())
    }
}

impl Source for MemorySource {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        let range = usize::try_from(start)
            .ok()
            .zip(usize::try_from(end).ok())
            .and_then(|(s, e)| self.0.get(s..e));
        range
            .map(Cow::Borrowed)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "range out of bounds"))
    }
}

/// One line ending of a storage, as seen from a piece.
#[derive(Clone, Copy, Debug)]
pub(crate) struct View {
    pub off: usize,
    pub len: usize,
    pub kind: Eol,
}

const KIND_BITS: u8 = 3;

fn kind_to_bits(kind: Eol) -> u8 {
    match kind {
        Eol::CrLf => 0,
        Eol::Lf => 1,
        Eol::Cr => 2,
        Eol::Nel => 3,
        Eol::Ls => 4,
        Eol::Ps => 5,
    }
}

fn bits_to_kind(bits: u8) -> Eol {
    match bits {
        0 => Eol::CrLf,
        1 => Eol::Lf,
        2 => Eol::Cr,
        3 => Eol::Nel,
        4 => Eol::Ls,
        _ => Eol::Ps,
    }
}

/// The line endings of one storage, sorted by position. 5 bytes per line ending.
///
/// A piece sees the line endings that overlap its byte range. A CRLF that crosses the edge of the
/// range is seen as a lone LF (when it crosses the start) or a lone CR (when it crosses the end),
/// so a piece never depends on what the neighbouring piece holds.
#[derive(Clone, Debug, Default)]
pub(crate) struct LineIndex {
    starts: Vec<u32>,
    /// Low 3 bits: the kind. High 5 bits: the length in bytes.
    meta: Vec<u8>,
}

impl LineIndex {
    fn push(&mut self, start: usize, len: usize, kind: Eol) {
        debug_assert!((1..32).contains(&len));
        self.starts.push(start as u32);
        self.meta
            .push(kind_to_bits(kind) | ((len as u8) << KIND_BITS));
    }

    pub fn shrink_to_fit(&mut self) {
        self.starts.shrink_to_fit();
        self.meta.shrink_to_fit();
    }

    pub fn heap_bytes(&self) -> usize {
        self.starts.capacity() * 4 + self.meta.capacity()
    }

    pub fn len(&self) -> usize {
        self.starts.len()
    }

    fn start(&self, j: usize) -> usize {
        self.starts[j] as usize
    }

    fn eol_len(&self, j: usize) -> usize {
        usize::from(self.meta[j] >> KIND_BITS)
    }

    fn kind(&self, j: usize) -> Eol {
        bits_to_kind(self.meta[j] & ((1 << KIND_BITS) - 1))
    }

    /// Scans `bytes` with `codec` in one pass. `base` is added to the offsets.
    pub fn scan(&mut self, codec: &dyn Codec, bytes: &[u8], base: usize) {
        let mut from = 0;
        while let Some((off, kind)) = codec.find_eol(bytes, from) {
            let fallback = if kind == Eol::CrLf { 2 } else { 1 };
            let len = codec.eol_bytes(kind).map_or(fallback, <[u8]>::len);
            // A misbehaving codec must not make this loop forever or index out of range.
            if off < from || len == 0 || len >= 32 || off + len > bytes.len() {
                break;
            }
            self.push(base + off, len, kind);
            from = off + len;
        }
    }

    /// The entries overlapping the byte range `s..e`, as an index range.
    fn overlapping(&self, s: usize, e: usize) -> (usize, usize) {
        if s >= e {
            return (0, 0);
        }
        let before = self.starts.partition_point(|&x| (x as usize) < s);
        let lo = if before > 0 && self.start(before - 1) + self.eol_len(before - 1) > s {
            before - 1
        } else {
            before
        };
        let hi = self.starts.partition_point(|&x| (x as usize) < e);
        (lo, hi.max(lo))
    }

    fn view(&self, j: usize, s: usize, e: usize) -> View {
        let start = self.start(j);
        let end = start + self.eol_len(j);
        if start < s {
            View {
                off: s,
                len: end.min(e) - s,
                kind: Eol::Lf,
            }
        } else if end > e {
            View {
                off: start,
                len: e - start,
                kind: Eol::Cr,
            }
        } else {
            View {
                off: start,
                len: end - start,
                kind: self.kind(j),
            }
        }
    }

    /// The aggregate of the byte range `s..e`.
    pub fn agg(&self, s: usize, e: usize) -> Agg {
        let (lo, hi) = self.overlapping(s, e);
        let mut agg = Agg {
            bytes: e - s,
            breaks: hi - lo,
            first_lf: 0,
            last_cr: false,
        };
        if hi > lo {
            let first = self.view(lo, s, e);
            if first.off == s && first.kind == Eol::Lf {
                agg.first_lf = first.len as u8;
            }
            let last = self.view(hi - 1, s, e);
            agg.last_cr = last.kind == Eol::Cr && last.off + last.len == e;
        }
        agg
    }

    /// The `n`-th line ending (0-based) seen from the piece `s..e`.
    pub fn nth_view(&self, s: usize, e: usize, n: usize) -> Option<View> {
        let (lo, hi) = self.overlapping(s, e);
        (lo + n < hi).then(|| self.view(lo + n, s, e))
    }
}

/// The original data, the codec of the file, and the line index built from them.
#[derive(Debug)]
pub(crate) struct Original {
    pub source: Box<dyn Source>,
    pub codec: Arc<dyn Codec>,
    pub index: LineIndex,
}

/// Text typed or pasted. UTF-8, append-only; clones of a document share it, and an old version only
/// refers to the part that existed when it was made.
#[derive(Debug, Default)]
pub(crate) struct AddBuf {
    pub data: Vec<u8>,
    pub index: LineIndex,
}

impl AddBuf {
    fn append(&mut self, text: &str) -> Result<(usize, usize)> {
        let start = self.data.len();
        let end = start + text.len();
        if end > u32::MAX as usize {
            return Err(BufferError::TooLarge);
        }
        let before = self.index.len();
        self.data.extend_from_slice(text.as_bytes());
        self.index.scan(&Utf8Codec, text.as_bytes(), start);
        // A CR at the end of the old text and an LF at the start of the new text are one CRLF.
        if before > 0
            && self.index.len() > before
            && self.index.start(before) == start
            && self.index.kind(before) == Eol::Lf
            && self.index.kind(before - 1) == Eol::Cr
            && self.index.start(before - 1) + 1 == start
        {
            let last = before - 1;
            self.index.meta[last] = kind_to_bits(Eol::CrLf) | (2 << KIND_BITS);
            self.index.starts.remove(before);
            self.index.meta.remove(before);
        }
        Ok((start, end))
    }
}

/// Everything the pieces of a document refer to. Shared (`Arc`) between clones of a document.
#[derive(Debug)]
pub(crate) struct Storages {
    pub original: Original,
    add: RwLock<AddBuf>,
}

impl Storages {
    pub fn new(original: Original) -> Self {
        Self {
            original,
            add: RwLock::new(AddBuf::default()),
        }
    }

    fn add_read(&self) -> RwLockReadGuard<'_, AddBuf> {
        // The add buffer is only appended to; a panic elsewhere cannot leave it half-updated in a
        // way that breaks reads, so a poisoned lock is still usable.
        self.add
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn add_write(&self) -> RwLockWriteGuard<'_, AddBuf> {
        self.add
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn with_index<R>(&self, store: Store, f: impl FnOnce(&LineIndex) -> R) -> R {
        match store {
            Store::Original => f(&self.original.index),
            Store::Add => f(&self.add_read().index),
        }
    }

    pub fn piece(&self, store: Store, start: usize, end: usize) -> Piece {
        let agg = self.with_index(store, |idx| idx.agg(start, end));
        Piece {
            store,
            start,
            end,
            agg,
        }
    }

    pub fn nth_view(&self, piece: &Piece, n: usize) -> Option<View> {
        self.with_index(piece.store, |idx| idx.nth_view(piece.start, piece.end, n))
    }

    /// Appends `text` to the add buffer and returns its byte range there.
    pub fn append_add(&self, text: &str) -> Result<(usize, usize)> {
        self.add_write().append(text)
    }

    /// Calls `f` with the bytes `start..end` of a storage.
    pub fn with_bytes<R>(
        &self,
        store: Store,
        start: usize,
        end: usize,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Result<R> {
        match store {
            Store::Original => Ok(f(&self.read_original(start, end)?)),
            Store::Add => {
                let add = self.add_read();
                let bytes = add.data.get(start..end).ok_or(BufferError::Corrupt)?;
                Ok(f(bytes))
            }
        }
    }

    pub fn read_original(&self, start: usize, end: usize) -> Result<Cow<'_, [u8]>> {
        let bytes = self.original.source.read(start as u64, end as u64)?;
        if bytes.len() != end - start {
            return Err(BufferError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the source returned fewer bytes than asked",
            )));
        }
        Ok(bytes)
    }

    /// Decodes the bytes `start..end` of a storage and appends the text to `out`.
    pub fn decode_into(
        &self,
        store: Store,
        start: usize,
        end: usize,
        out: &mut String,
    ) -> Result<()> {
        match store {
            Store::Original => {
                let bytes = self.read_original(start, end)?;
                self.original.codec.decode(&bytes, out);
                Ok(())
            }
            Store::Add => self.with_bytes(store, start, end, |b| {
                out.push_str(&String::from_utf8_lossy(b))
            }),
        }
    }

    /// The length of the decoded text of the bytes `start..end`. `scratch` avoids an allocation.
    pub fn decoded_len(
        &self,
        store: Store,
        start: usize,
        end: usize,
        scratch: &mut String,
    ) -> Result<usize> {
        match store {
            Store::Add => Ok(end - start),
            Store::Original => {
                scratch.clear();
                self.decode_into(store, start, end, scratch)?;
                Ok(scratch.len())
            }
        }
    }

    /// The heap bytes of the line indexes (for reports and tests).
    pub fn index_heap_bytes(&self) -> usize {
        self.original.index.heap_bytes() + self.add_read().index.heap_bytes()
    }

    pub fn add_len(&self) -> usize {
        self.add_read().data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_source_reads_ranges() {
        let s = MemorySource::new(b"hello".to_vec());
        assert_eq!(&*s.read(1, 3).unwrap(), b"el");
        assert!(s.read(3, 9).is_err());
        assert!(s.read(4, 2).is_err());
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn add_buffer_joins_cr_and_lf_across_appends() {
        let mut add = AddBuf::default();
        add.append("a\r").unwrap();
        add.append("\nb").unwrap();
        assert_eq!(add.index.len(), 1);
        assert_eq!(add.index.kind(0), Eol::CrLf);
        // The first append's piece sees a CR, the second append's piece sees an LF.
        assert!(add.index.agg(0, 2).last_cr);
        assert_eq!(add.index.agg(2, 4).first_lf, 1);
        // A piece covering both sees one break.
        assert_eq!(add.index.agg(0, 4).breaks, 1);
    }

    #[test]
    fn index_clips_a_crlf_at_piece_edges() {
        let mut idx = LineIndex::default();
        idx.scan(&Utf8Codec, b"ab\r\ncd", 0);
        let head = idx.agg(0, 3);
        assert_eq!((head.breaks, head.last_cr), (1, true));
        let tail = idx.agg(3, 6);
        assert_eq!((tail.breaks, tail.first_lf), (1, 1));
        let whole = idx.agg(0, 6);
        assert_eq!((whole.breaks, whole.first_lf, whole.last_cr), (1, 0, false));
    }
}
