//! The block line index (ADR-0020, chapter 2, plan B) and the scan that makes it.
//!
//! A file that is read by ranges cannot keep every line ending in memory: ten billion of them
//! would take 50 GB. The block index cuts the file into blocks of about 64 KiB and keeps, per
//! block, only where it starts and how many line endings come before it (16 bytes per block, about
//! 2.5 MB for 10 GB). To find a line ending inside a block, the block is read and its line endings
//! are found again; the table of the most recently used blocks is kept (a small cache next to the
//! byte cache of the source), so that reading line after line does not scan a block for each line.
//!
//! The index is one of two implementations of [`EolIndex`](super::storage::EolIndex); the piece
//! tree, the layout and the search cannot tell it from the dense [`LineIndex`](super::storage).
//!
//! # Block edges
//!
//! The scan goes through the file from the start and cuts it only where
//!
//! * a character ends, as [`Codec::char_len`] walks the characters from the previous edge. This is
//!   what makes Shift_JIS, EUC-JP, UTF-16 and UTF-32 safe: from the middle of a file the first
//!   character of a range is not always known (a Shift_JIS trail byte 0x40..0x7E looks like
//!   ASCII), but every edge is a boundary found by walking from the start, so a block is always
//!   read from a known character boundary;
//! * no line ending is cut: a CRLF (or a 4-byte UTF-16 CRLF) is never split, an edge that would
//!   fall inside one moves to its end.
//!
//! The edges are also the "marks of the character boundary" of ADR-0020 chapter 3 for a very long
//! line: a line over many blocks has a known boundary every block (64 KiB, finer than the 1 MiB of
//! the ADR, at no extra memory), and [`BlockIndex::char_boundary_at_or_before`] finds the boundary
//! of any position by walking one block at most.
//!
//! Encodings that depend on earlier bytes (ISO-2022-JP, UTF-7) have no such edges. They are not
//! scanned ([`ScanError::Stateful`]); the caller reads them whole as before (ADR-0020, chapter 3,
//! plan A).

mod long;
mod scan;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};

use long::{CharsCache, LongAnchor};
pub use scan::{
    BlockScanner, DEFAULT_LONG_LINE, ScanError, ScanFailure, ScanJob, ScanOptions, ScanProgress,
    ScanState, scan_blocks,
};

use super::codec::{self, Codec, Eol};
use super::storage::{Entry, EolIndex, Source};

/// The default size of a block, 64 KiB (ADR-0020, chapter 2).
pub const DEFAULT_INDEX_BLOCK_SIZE: usize = 64 * 1024;

/// How many blocks' line ending tables are kept.
const TABLE_CACHE_BLOCKS: usize = 32;

/// The length in bytes of each kind of line ending in a codec, looked up once. A codec that
/// cannot write a kind (and so cannot tell its length) gets the length of the ASCII ending, as
/// `LineIndex::scan` does.
#[derive(Clone, Copy, Debug)]
struct EolLens([usize; 6]);

impl EolLens {
    fn new(codec: &dyn Codec) -> Self {
        let len = |kind: Eol| {
            let fallback = if kind == Eol::CrLf { 2 } else { 1 };
            codec.eol_bytes(kind).map_or(fallback, <[u8]>::len)
        };
        let mut lens = [0; 6];
        for kind in [Eol::CrLf, Eol::Lf, Eol::Cr, Eol::Nel, Eol::Ls, Eol::Ps] {
            lens[usize::from(kind_bits(kind))] = len(kind);
        }
        Self(lens)
    }

    fn get(&self, kind: Eol) -> usize {
        self.0[usize::from(kind_bits(kind))]
    }
}

/// Calls `f(offset, length, kind)` for every line ending of `bytes` that starts at or after `from`
/// and before `until`, in order, with the same rules as `LineIndex::scan`. Returns the start of
/// the first line ending at or after `until`, if there is one.
fn walk_eols(
    codec: &dyn Codec,
    lens: &EolLens,
    bytes: &[u8],
    mut from: usize,
    until: usize,
    ext_eol: bool,
    mut f: impl FnMut(usize, usize, Eol),
) -> Option<usize> {
    loop {
        let (off, kind) = codec::find_eol(codec, bytes, from, ext_eol)?;
        let len = lens.get(kind);
        // A misbehaving codec must not make this loop forever or index out of range.
        if off < from || len == 0 || len >= 32 || off + len > bytes.len() {
            return None;
        }
        if off >= until {
            return Some(off);
        }
        f(off, len, kind);
        from = off + len;
    }
}

/// The line endings of one block: offsets from the start of the block, and the length and kind
/// (5 bytes per line ending, like the dense index).
#[derive(Debug, Default)]
struct Table {
    offs: Vec<u32>,
    meta: Vec<u8>,
}

impl Table {
    fn push(&mut self, off: usize, len: usize, kind: Eol) {
        self.offs.push(off as u32);
        self.meta.push(((len as u8) << 3) | kind_bits(kind));
    }

    fn len(&self) -> usize {
        self.offs.len()
    }

    /// The number of line endings that start before `off`.
    fn count_before(&self, off: usize) -> usize {
        u32::try_from(off).map_or(self.offs.len(), |off| {
            self.offs.partition_point(|&x| x < off)
        })
    }

    fn entry(&self, j: usize, block_start: u64) -> Entry {
        Entry {
            start: (block_start + u64::from(self.offs[j])) as usize,
            len: usize::from(self.meta[j] >> 3),
            kind: bits_kind(self.meta[j] & 7),
        }
    }
}

fn kind_bits(kind: Eol) -> u8 {
    match kind {
        Eol::CrLf => 0,
        Eol::Lf => 1,
        Eol::Cr => 2,
        Eol::Nel => 3,
        Eol::Ls => 4,
        Eol::Ps => 5,
    }
}

fn bits_kind(bits: u8) -> Eol {
    match bits {
        0 => Eol::CrLf,
        1 => Eol::Lf,
        2 => Eol::Cr,
        3 => Eol::Nel,
        4 => Eol::Ls,
        _ => Eol::Ps,
    }
}

/// The tables of the blocks used last (LRU). Shared by the clones of an index: a block's table
/// never changes.
#[derive(Debug, Default)]
struct TableCache {
    tables: HashMap<usize, (Arc<Table>, u64)>,
    tick: u64,
}

impl TableCache {
    fn get(&mut self, block: usize) -> Option<Arc<Table>> {
        self.tick += 1;
        let tick = self.tick;
        let (table, used) = self.tables.get_mut(&block)?;
        *used = tick;
        Some(table.clone())
    }

    fn insert(&mut self, block: usize, table: Arc<Table>) {
        self.tick += 1;
        while self.tables.len() >= TABLE_CACHE_BLOCKS && !self.tables.contains_key(&block) {
            let Some(oldest) = self
                .tables
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(block, _)| *block)
            else {
                break;
            };
            self.tables.remove(&oldest);
        }
        self.tables.insert(block, (table, self.tick));
    }

    fn bytes(&self) -> usize {
        self.tables
            .values()
            .map(|(t, _)| t.offs.capacity() * 4 + t.meta.capacity())
            .sum()
    }
}

/// The line endings of a file read by ranges, kept as a count per block. See the module
/// documentation.
///
/// Cloning copies the two arrays of the blocks (16 bytes per block) and shares the source and the
/// table cache.
#[derive(Clone)]
pub struct BlockIndex {
    source: Arc<dyn Source>,
    codec: Arc<dyn Codec>,
    lens: EolLens,
    ext_eol: bool,
    block_size: usize,
    /// The start of each block. Block `i` is `starts[i]..starts[i + 1]`, the last one ends at
    /// `covered`.
    starts: Vec<u64>,
    /// The number of line endings that start before each block.
    breaks_before: Vec<u64>,
    /// The end of the scanned part: the file is known up to here.
    covered: u64,
    /// The number of line endings in `0..covered`.
    breaks: u64,
    /// The blocks that are in a long line, sorted by block (ADR-0020, 9.10).
    long_anchors: Vec<LongAnchor>,
    chars: Arc<Mutex<CharsCache>>,
    /// The size from which a line is long (see [`ScanOptions::long_line`]); 0 when there are none.
    long_line: usize,
    /// A byte below 0x80 that stands alone decodes to itself in this codec (not in UTF-16, where
    /// an odd byte is an invalid one).
    ascii_bytes: bool,
    tables: Arc<Mutex<TableCache>>,
}

impl fmt::Debug for BlockIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockIndex")
            .field("block_size", &self.block_size)
            .field("blocks", &self.starts.len())
            .field("covered", &self.covered)
            .field("breaks", &self.breaks)
            .field("complete", &self.is_complete())
            .finish_non_exhaustive()
    }
}

impl BlockIndex {
    /// An index that covers nothing yet. The scan fills it.
    fn new(
        source: Arc<dyn Source>,
        codec: Arc<dyn Codec>,
        ext_eol: bool,
        block_size: usize,
        long_line: usize,
    ) -> Self {
        let ascii_bytes = {
            let mut text = String::new();
            codec.decode(b"a", &mut text);
            text == "a"
        };
        Self {
            source,
            lens: EolLens::new(&*codec),
            codec,
            ext_eol,
            block_size,
            starts: Vec::new(),
            breaks_before: Vec::new(),
            covered: 0,
            breaks: 0,
            long_anchors: Vec::new(),
            chars: Arc::default(),
            ascii_bytes,
            long_line,
            tables: Arc::default(),
        }
    }

    /// The source the index was made from.
    pub fn source(&self) -> &Arc<dyn Source> {
        &self.source
    }

    /// The codec the index was made with.
    pub fn codec(&self) -> &Arc<dyn Codec> {
        &self.codec
    }

    /// Whether NEL, LS and PS ended lines when the index was made.
    pub fn ext_eol(&self) -> bool {
        self.ext_eol
    }

    /// The size blocks were cut at (a block is a little longer when a character or a line ending
    /// crosses the edge, and the last one may be shorter).
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// The number of blocks.
    pub fn block_count(&self) -> usize {
        self.starts.len()
    }

    /// The bytes `0..covered_len()` are scanned. This is the length of the file once the index is
    /// complete.
    pub fn covered_len(&self) -> u64 {
        self.covered
    }

    /// The length of the file.
    pub fn source_len(&self) -> u64 {
        self.source.len()
    }

    /// Whether the whole file is scanned.
    pub fn is_complete(&self) -> bool {
        self.covered >= self.source.len()
    }

    /// The number of line endings in the scanned part. Exact (a CRLF is one).
    pub fn break_count(&self) -> u64 {
        self.breaks
    }

    /// The bytes the two block arrays hold (16 per block).
    pub fn heap_bytes(&self) -> usize {
        self.starts.capacity() * 8 + self.breaks_before.capacity() * 8
    }

    /// The bytes of the line ending tables that are cached now.
    pub fn cached_table_bytes(&self) -> usize {
        self.tables
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .bytes()
    }

    /// The block that holds byte `pos`, if `pos` is in the scanned part.
    pub fn block_of(&self, pos: u64) -> Option<usize> {
        (pos < self.covered).then(|| self.starts.partition_point(|&s| s <= pos) - 1)
    }

    /// The byte range of block `block`.
    pub fn block_range(&self, block: usize) -> Option<(u64, u64)> {
        let start = *self.starts.get(block)?;
        let end = self.starts.get(block + 1).copied().unwrap_or(self.covered);
        Some((start, end))
    }

    /// The number of line endings before block `block` (`break_count()` for one past the last).
    pub fn breaks_before_block(&self, block: usize) -> Option<u64> {
        match block.cmp(&self.starts.len()) {
            std::cmp::Ordering::Less => Some(self.breaks_before[block]),
            std::cmp::Ordering::Equal => Some(self.breaks),
            std::cmp::Ordering::Greater => None,
        }
    }

    /// The largest character boundary at or before `pos`, found by walking the characters from
    /// the start of the block that holds `pos`. This is how a position inside a very long line is
    /// moved to a place a character starts (ADR-0020, chapter 3). A `pos` past the scanned part
    /// gives the end of it.
    pub fn char_boundary_at_or_before(&self, pos: u64) -> io::Result<u64> {
        let Some(block) = self.block_of(pos) else {
            return Ok(self.covered);
        };
        let (start, end) = self.block_range(block).unwrap_or((0, 0));
        let bytes = self.source.read(start, end)?;
        if bytes.len() as u64 != end - start {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let target = (pos - start) as usize;
        let mut at = 0;
        loop {
            let n = self.codec.char_len(&bytes, at).clamp(1, bytes.len() - at);
            if at + n > target {
                return Ok(start + at as u64);
            }
            at += n;
        }
    }

    /// The table of block `block`, from the cache or by reading the block.
    fn table(&self, block: usize) -> io::Result<Arc<Table>> {
        if let Some(table) = self
            .tables
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(block)
        {
            return Ok(table);
        }
        // Read outside the lock, so that a slow disk does not block the other readers.
        let (start, end) = self
            .block_range(block)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "block out of range"))?;
        let bytes = self.source.read(start, end)?;
        if bytes.len() as u64 != end - start {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let mut table = Table::default();
        walk_eols(
            &*self.codec,
            &self.lens,
            &bytes,
            0,
            usize::MAX,
            self.ext_eol,
            |o, l, k| {
                table.push(o, l, k);
            },
        );
        let expected = self.breaks_before_block(block + 1).unwrap_or(0)
            - self.breaks_before_block(block).unwrap_or(0);
        if table.len() as u64 != expected {
            // The bytes are not the ones that were scanned.
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "a block does not hold the line endings it had when it was scanned",
            ));
        }
        let table = Arc::new(table);
        self.tables
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(block, table.clone());
        Ok(table)
    }

    /// Adds the blocks of a scanned window. `blocks` are `(start, breaks in the block)`; they
    /// begin at `covered` and end at `new_covered`.
    fn extend(&mut self, blocks: &[(u64, u64)], new_covered: u64) {
        for &(start, breaks) in blocks {
            debug_assert!(start >= self.covered);
            self.starts.push(start);
            self.breaks_before.push(self.breaks);
            self.breaks += breaks;
        }
        self.covered = new_covered;
    }

    /// Reserves room for `n` more blocks, failing instead of aborting when memory runs out.
    fn try_reserve(&mut self, n: usize) -> Result<(), std::collections::TryReserveError> {
        self.starts.try_reserve(n)?;
        self.breaks_before.try_reserve(n)
    }
}

impl EolIndex for BlockIndex {
    fn len(&self) -> usize {
        usize::try_from(self.breaks).unwrap_or(usize::MAX)
    }

    fn count_before(&self, pos: usize) -> io::Result<usize> {
        let pos = pos as u64;
        let Some(block) = self.block_of(pos) else {
            return Ok(self.len());
        };
        let before = self.breaks_before[block];
        let start = self.starts[block];
        if pos == start {
            return Ok(before as usize);
        }
        let table = self.table(block)?;
        Ok(before as usize + table.count_before((pos - start) as usize))
    }

    fn entry(&self, j: usize) -> io::Result<Entry> {
        let j64 = j as u64;
        if j64 >= self.breaks {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "line ending out of range",
            ));
        }
        // The last block that has `j` or fewer line endings before it holds entry `j`.
        let block = self.breaks_before.partition_point(|&b| b <= j64) - 1;
        let table = self.table(block)?;
        Ok(table.entry(j - self.breaks_before[block] as usize, self.starts[block]))
    }

    fn heap_bytes(&self) -> usize {
        BlockIndex::heap_bytes(self)
    }

    fn long_line_bytes(&self) -> Option<usize> {
        (self.long_line > 0).then_some(self.long_line)
    }

    fn decoded_len_long(&self, start: usize, end: usize) -> io::Result<Option<usize>> {
        Ok(BlockIndex::decoded_len_long(self, start as u64, end as u64)?.map(|n| n as usize))
    }

    fn raw_at_decoded_long(
        &self,
        start: usize,
        end: usize,
        cols: usize,
    ) -> io::Result<Option<usize>> {
        Ok(
            BlockIndex::raw_at_decoded_long(self, start as u64, end as u64, cols as u64)?
                .map(|n| n as usize),
        )
    }
}
