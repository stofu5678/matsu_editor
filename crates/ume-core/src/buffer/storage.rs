//! The two storages behind the pieces: the original bytes and the add buffer.

use std::borrow::Cow;
use std::collections::TryReserveError;
use std::fmt;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::codec::{self, Codec, Eol};
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
/// Positions are 64-bit, but only the low 32 bits are stored per entry. The high 32 bits change
/// rarely (once per 4 GiB), so `epochs` records where they change. A plain 64-bit array would
/// cost 9 bytes per line ending instead of 5 (measured: 16.5 MB -> 29.7 MB for 100 MB of text).
///
/// A piece sees the line endings that overlap its byte range. A CRLF that crosses the edge of the
/// range is seen as a lone LF (when it crosses the start) or a lone CR (when it crosses the end),
/// so a piece never depends on what the neighbouring piece holds.
#[derive(Clone, Debug, Default)]
pub(crate) struct LineIndex {
    /// The low 32 bits of the start of each line ending.
    starts: Vec<u32>,
    /// `(first entry, high 32 bits)` for each run of entries whose start has non-zero high bits,
    /// sorted. Entries before the first item have high bits 0. Empty below 4 GiB.
    epochs: Vec<(usize, u32)>,
    /// Low 3 bits: the kind. High 5 bits: the length in bytes.
    meta: Vec<u8>,
}

impl LineIndex {
    /// Adds an entry. Fails, leaving the index as it was, when memory runs out: a huge file must
    /// not abort the process (ADR-0012).
    fn push(
        &mut self,
        start: usize,
        len: usize,
        kind: Eol,
    ) -> std::result::Result<(), TryReserveError> {
        debug_assert!((1..32).contains(&len));
        let high = (start as u64 >> 32) as u32;
        self.starts.try_reserve(1)?;
        self.meta.try_reserve(1)?;
        if high != self.high_of_last() {
            self.epochs.try_reserve(1)?;
            self.epochs.push((self.starts.len(), high));
        }
        self.starts.push(start as u32);
        self.meta
            .push(kind_to_bits(kind) | ((len as u8) << KIND_BITS));
        Ok(())
    }

    /// Drops the entries from `n` on.
    fn truncate(&mut self, n: usize) {
        self.starts.truncate(n);
        self.meta.truncate(n);
        while self.epochs.last().is_some_and(|&(first, _)| first >= n) {
            self.epochs.pop();
        }
    }

    fn high_of_last(&self) -> u32 {
        self.epochs.last().map_or(0, |&(_, high)| high)
    }

    pub fn shrink_to_fit(&mut self) {
        self.starts.shrink_to_fit();
        self.epochs.shrink_to_fit();
        self.meta.shrink_to_fit();
    }

    pub fn heap_bytes(&self) -> usize {
        self.starts.capacity() * 4
            + self.meta.capacity()
            + self.epochs.capacity() * std::mem::size_of::<(usize, u32)>()
    }

    pub fn len(&self) -> usize {
        self.starts.len()
    }

    /// Removes entry `j`, which must be the one the add buffer just appended.
    fn remove_entry(&mut self, j: usize) {
        self.starts.remove(j);
        self.meta.remove(j);
        // Epoch starts at or after `j` move down by one; one that began exactly at `j` now begins
        // at the entry that took its place (or is dropped when none did).
        let n = self.starts.len();
        for e in &mut self.epochs {
            if e.0 > j {
                e.0 -= 1;
            }
        }
        while self.epochs.last().is_some_and(|&(first, _)| first >= n) {
            self.epochs.pop();
        }
    }

    fn start(&self, j: usize) -> usize {
        let high = if self.epochs.is_empty() {
            0
        } else {
            let k = self.epochs.partition_point(|&(first, _)| first <= j);
            k.checked_sub(1).map_or(0, |k| self.epochs[k].1)
        };
        ((u64::from(high) << 32) | u64::from(self.starts[j])) as usize
    }

    /// The number of entries whose start is less than `pos`.
    fn count_before(&self, pos: usize) -> usize {
        if self.epochs.is_empty() {
            // Fast path below 4 GiB: compare the stored 32-bit values.
            let Ok(pos) = u32::try_from(pos) else {
                return self.starts.len();
            };
            return self.starts.partition_point(|&x| x < pos);
        }
        let (mut lo, mut hi) = (0, self.starts.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.start(mid) < pos {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    fn eol_len(&self, j: usize) -> usize {
        usize::from(self.meta[j] >> KIND_BITS)
    }

    fn kind(&self, j: usize) -> Eol {
        bits_to_kind(self.meta[j] & ((1 << KIND_BITS) - 1))
    }

    /// Scans `bytes` with `codec` in one pass. `base` is added to the offsets. NEL, LS and PS
    /// end a line only when `ext_eol` is set. When memory runs out, the entries of this call are
    /// dropped again and the error is returned.
    pub fn scan(
        &mut self,
        codec: &dyn Codec,
        bytes: &[u8],
        base: usize,
        ext_eol: bool,
    ) -> std::result::Result<(), TryReserveError> {
        let before = self.len();
        let result = self.scan_inner(codec, bytes, base, ext_eol);
        if result.is_err() {
            self.truncate(before);
        }
        result
    }

    fn scan_inner(
        &mut self,
        codec: &dyn Codec,
        bytes: &[u8],
        base: usize,
        ext_eol: bool,
    ) -> std::result::Result<(), TryReserveError> {
        let mut from = 0;
        while let Some((off, kind)) = codec::find_eol(codec, bytes, from, ext_eol) {
            let fallback = if kind == Eol::CrLf { 2 } else { 1 };
            let len = codec.eol_bytes(kind).map_or(fallback, <[u8]>::len);
            // A misbehaving codec must not make this loop forever or index out of range.
            if off < from || len == 0 || len >= 32 || off + len > bytes.len() {
                break;
            }
            self.push(base + off, len, kind)?;
            from = off + len;
        }
        Ok(())
    }

    /// The entries overlapping the byte range `s..e`, as an index range.
    fn overlapping(&self, s: usize, e: usize) -> (usize, usize) {
        if s >= e {
            return (0, 0);
        }
        let before = self.count_before(s);
        let lo = if before > 0 && self.start(before - 1) + self.eol_len(before - 1) > s {
            before - 1
        } else {
            before
        };
        let hi = self.count_before(e);
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
    /// `(first byte, setting)` for each run of text that was added under one setting of
    /// `bEnableExtEol`. The index above was made with it, and a saver that scans the text again
    /// must use the same one.
    pub epochs: Vec<(usize, bool)>,
}

impl AddBuf {
    fn append(&mut self, text: &str, ext_eol: bool) -> Result<(usize, usize)> {
        let start = self.data.len();
        // Only a length that does not fit the address space can fail here; memory runs out first.
        let end = start.checked_add(text.len()).ok_or(BufferError::TooLarge)?;
        let before = self.index.len();
        self.data.try_reserve(text.len())?;
        self.epochs.try_reserve(1)?;
        self.index
            .scan(codec::utf8(), text.as_bytes(), start, ext_eol)?;
        self.data.extend_from_slice(text.as_bytes());
        if self.epochs.last().is_none_or(|&(_, on)| on != ext_eol) {
            self.epochs.push((start, ext_eol));
        }
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
            self.index.remove_entry(before);
        }
        Ok((start, end))
    }
}

/// Everything the pieces of a document refer to. Shared (`Arc`) between clones of a document.
#[derive(Debug)]
pub(crate) struct Storages {
    pub original: Original,
    add: RwLock<AddBuf>,
    /// Whether NEL, LS and PS end a line in text added from now on (`m_bEnableExtEol`). Lines
    /// that exist keep the endings they were made with, as in Sakura.
    ext_eol: AtomicBool,
    /// The setting when the original was read: the line endings of the original are those.
    load_ext_eol: bool,
}

impl Storages {
    pub fn new(original: Original, ext_eol: bool) -> Self {
        Self {
            original,
            add: RwLock::new(AddBuf::default()),
            ext_eol: AtomicBool::new(ext_eol),
            load_ext_eol: ext_eol,
        }
    }

    pub fn ext_eol(&self) -> bool {
        self.ext_eol.load(Ordering::Relaxed)
    }

    /// The setting that the added text at byte `offset` was scanned under.
    pub fn add_ext_eol_at(&self, offset: usize) -> bool {
        let add = self.add_read();
        let i = add.epochs.partition_point(|&(first, _)| first <= offset);
        i.checked_sub(1)
            .map_or_else(|| self.ext_eol(), |i| add.epochs[i].1)
    }

    pub fn load_ext_eol(&self) -> bool {
        self.load_ext_eol
    }

    pub fn set_ext_eol(&self, on: bool) {
        self.ext_eol.store(on, Ordering::Relaxed);
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
        self.add_write().append(text, self.ext_eol())
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
        add.append("a\r", true).unwrap();
        add.append("\nb", true).unwrap();
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
        idx.scan(codec::utf8(), b"ab\r\ncd", 0, true).unwrap();
        let head = idx.agg(0, 3);
        assert_eq!((head.breaks, head.last_cr), (1, true));
        let tail = idx.agg(3, 6);
        assert_eq!((tail.breaks, tail.first_lf), (1, 1));
        let whole = idx.agg(0, 6);
        assert_eq!((whole.breaks, whole.first_lf, whole.last_cr), (1, 0, false));
    }

    #[cfg(target_pointer_width = "64")]
    const GIB: u64 = 1 << 30;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_line_index_addresses_more_than_4_gib() {
        let mut idx = LineIndex::default();
        let base = 5 * GIB as usize;
        let far = 9 * GIB as usize;
        idx.scan(codec::utf8(), b"ab\r\ncd\nef", base, true)
            .unwrap();
        idx.scan(codec::utf8(), b"\n", far, true).unwrap();
        assert_eq!(idx.len(), 3);
        assert_eq!(idx.agg(base, base + 10).breaks, 2);
        // A CRLF cut by a piece edge is seen as a lone CR or a lone LF.
        let head = idx.agg(base, base + 3);
        assert_eq!((head.breaks, head.last_cr), (1, true));
        let tail = idx.agg(base + 3, base + 10);
        assert_eq!((tail.breaks, tail.first_lf), (2, 1));
        assert_eq!(idx.agg(0, base).breaks, 0);
        assert_eq!(idx.agg(0, 10 * GIB as usize).breaks, 3);
        assert_eq!(idx.agg(base + 10, far).breaks, 0);
        assert_eq!(idx.agg(far, far + 1).breaks, 1);
        // Below 4 GiB the high words cost nothing.
        let mut small = LineIndex::default();
        small.scan(codec::utf8(), b"a\nb\n", 0, true).unwrap();
        small.shrink_to_fit();
        assert_eq!(small.heap_bytes(), small.len() * 5);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn removing_an_index_entry_keeps_the_high_words_right() {
        let mut idx = LineIndex::default();
        let at = 4 * GIB as usize - 1;
        idx.scan(codec::utf8(), b"\r", at, true).unwrap();
        idx.scan(codec::utf8(), b"\n", at + 1, true).unwrap();
        assert_eq!(idx.len(), 2);
        idx.remove_entry(1);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.agg(at - 4, at + 4).breaks, 1);
        // An entry that began a high word, followed by one that did not.
        let mut idx = LineIndex::default();
        idx.scan(codec::utf8(), b"\n\n\n", 4 * GIB as usize, true)
            .unwrap();
        idx.remove_entry(0);
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.agg(4 * GIB as usize, 4 * GIB as usize + 3).breaks, 2);
    }
}
