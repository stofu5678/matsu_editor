//! Places in a very long line (ADR-0020, 9.10).
//!
//! The scan records, for each block that is in a line of `long_line` bytes or more, where the line
//! starts and how long its decoded text is before the block ([`LongAnchor`]). With that, the
//! decoded length of any part of the line, and the place where a given length of text has passed,
//! are found by looking at one block: the block is read and the position of each of its characters
//! in the decoded text is kept ([`BlockChars`], the blocks used last are cached). Nothing is
//! decoded from the start of the line.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use super::BlockIndex;

/// How many blocks' character tables are kept.
const CHARS_CACHE_BLOCKS: usize = 16;

/// A block in a very long line, and what is known of the decoded text of the line before it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LongAnchor {
    pub block: usize,
    /// Where the line starts.
    pub line_start: u64,
    /// The length of the decoded text of the line from its start to the start of the block.
    pub dec_before: u64,
}

/// The characters of one block: where each starts in the block and in the decoded text of the
/// block.
#[derive(Debug)]
pub(crate) struct BlockChars {
    /// `(offset in the block, length of the decoded text before it)` of each character.
    chars: Vec<(u32, u32)>,
    /// The length of the bytes and of the decoded text of the block.
    len: u32,
    decoded: u32,
}

impl BlockChars {
    /// The length of the decoded text before the character that holds the byte `offset`.
    fn decoded_before(&self, offset: u64) -> u64 {
        if offset >= u64::from(self.len) {
            return u64::from(self.decoded);
        }
        let i = self
            .chars
            .partition_point(|&(raw, _)| u64::from(raw) <= offset);
        i.checked_sub(1).map_or(0, |i| u64::from(self.chars[i].1))
    }

    /// The offset of the character that holds the byte `dec` of the decoded text of the block, or
    /// `None` when the block has not so much text.
    fn offset_of_decoded(&self, dec: u64) -> Option<u64> {
        if dec >= u64::from(self.decoded) {
            return None;
        }
        let i = self.chars.partition_point(|&(_, d)| u64::from(d) <= dec);
        i.checked_sub(1).map(|i| u64::from(self.chars[i].0))
    }
}

/// The tables of the blocks used last (LRU). Shared by the clones of an index.
#[derive(Debug, Default)]
pub(crate) struct CharsCache {
    blocks: HashMap<usize, (Arc<BlockChars>, u64)>,
    tick: u64,
}

impl CharsCache {
    fn get(&mut self, block: usize) -> Option<Arc<BlockChars>> {
        self.tick += 1;
        let tick = self.tick;
        let (chars, used) = self.blocks.get_mut(&block)?;
        *used = tick;
        Some(chars.clone())
    }

    fn insert(&mut self, block: usize, chars: Arc<BlockChars>) {
        self.tick += 1;
        while self.blocks.len() >= CHARS_CACHE_BLOCKS && !self.blocks.contains_key(&block) {
            let Some(oldest) = self
                .blocks
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(block, _)| *block)
            else {
                break;
            };
            self.blocks.remove(&oldest);
        }
        self.blocks.insert(block, (chars, self.tick));
    }
}

impl BlockIndex {
    fn anchor_of(&self, block: usize) -> Option<&LongAnchor> {
        self.long_anchors
            .binary_search_by_key(&block, |a| a.block)
            .ok()
            .map(|i| &self.long_anchors[i])
    }

    /// The decoded length of the bytes `from..to`, read from the source.
    fn direct_dec(&self, from: u64, to: u64) -> io::Result<u64> {
        if to <= from {
            return Ok(0);
        }
        let bytes = self.source.read(from, to)?;
        if bytes.len() as u64 != to - from {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let mut text = String::with_capacity(bytes.len());
        self.codec.decode(&bytes, &mut text);
        Ok(text.len() as u64)
    }

    /// The characters of block `block`, from the cache or by reading the block.
    fn block_chars(&self, block: usize) -> io::Result<Arc<BlockChars>> {
        if let Some(hit) = self
            .chars
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(block)
        {
            return Ok(hit);
        }
        let (start, end) = self
            .block_range(block)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "block out of range"))?;
        let bytes = self.source.read(start, end)?;
        if bytes.len() as u64 != end - start {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        let mut chars = Vec::new();
        chars.try_reserve(bytes.len()).map_err(io::Error::other)?;
        let (mut at, mut decoded) = (0usize, 0u32);
        let mut scratch = String::new();
        while at < bytes.len() {
            chars.push((at as u32, decoded));
            let n = self.codec.char_len(&bytes, at).clamp(1, bytes.len() - at);
            decoded += if n == 1 && bytes[at] < 0x80 && self.ascii_bytes {
                1
            } else {
                scratch.clear();
                self.codec.decode(&bytes[at..at + n], &mut scratch);
                scratch.len() as u32
            };
            at += n;
        }
        let chars = Arc::new(BlockChars {
            chars,
            len: bytes.len() as u32,
            decoded,
        });
        self.chars
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(block, chars.clone());
        Ok(chars)
    }

    /// Where the line that holds `x` starts and how long its decoded text is up to `x`, when `x`
    /// is in a block that has an anchor. The end of a line that ends at the edge of a block is
    /// in the block before the edge.
    fn dec_at(&self, x: u64) -> io::Result<Option<(u64, u64)>> {
        if self.long_anchors.is_empty() || x > self.covered || self.starts.is_empty() {
            return Ok(None);
        }
        let b = self
            .block_of(x)
            .unwrap_or_else(|| self.starts.len().saturating_sub(1));
        let (b, start) = match self.anchor_of(b) {
            Some(_) => (b, self.starts[b]),
            None if x == self.starts[b] && b > 0 && self.anchor_of(b - 1).is_some() => {
                (b - 1, self.starts[b - 1])
            }
            None => return Ok(None),
        };
        let a = self.anchor_of(b).expect("checked above");
        let before = self.block_chars(b)?.decoded_before(x - start);
        Ok(Some((a.line_start, a.dec_before + before)))
    }

    /// Where the line of `start..end` starts, and the decoded length of the line up to `start`.
    fn dec_of_start(&self, start: u64, end: u64) -> io::Result<Option<(u64, u64)>> {
        if let Some(found) = self.dec_at(start)? {
            return Ok(Some(found));
        }
        // Before the first anchor of the line (less than a block of it): the anchor at the end
        // knows where the line starts.
        let Some((line_start, _)) = self.dec_at(end)? else {
            return Ok(None);
        };
        if start < line_start || start - line_start > 2 * self.block_size as u64 {
            return Ok(None);
        }
        Ok(Some((line_start, self.direct_dec(line_start, start)?)))
    }

    /// The decoded length of the bytes `start..end` of one line, without reading them all, if the
    /// line is a long one (ADR-0020, 9.10). `None` for a short range or a short line.
    pub(crate) fn decoded_len_long(&self, start: u64, end: u64) -> io::Result<Option<u64>> {
        if self.long_anchors.is_empty() || end < start || end - start < 2 * self.block_size as u64 {
            return Ok(None);
        }
        let Some((line_start, d_end)) = self.dec_at(end)? else {
            return Ok(None);
        };
        let Some((ls, d_start)) = self.dec_of_start(start, end)? else {
            return Ok(None);
        };
        Ok((ls == line_start && d_end >= d_start).then(|| d_end - d_start))
    }

    /// The offset in `start..end` (bytes of one line) where `cols` bytes of decoded text from
    /// `start` have been passed, rounded down to a character, if the line is a long one. `None`
    /// when the place is not in the part that has anchors (the caller reads from `start`).
    pub(crate) fn raw_at_decoded_long(
        &self,
        start: u64,
        end: u64,
        cols: u64,
    ) -> io::Result<Option<u64>> {
        if self.long_anchors.is_empty() || end < start || end - start < 2 * self.block_size as u64 {
            return Ok(None);
        }
        let Some((line_start, _)) = self.dec_at(end)? else {
            return Ok(None);
        };
        let Some((ls, d_start)) = self.dec_of_start(start, end)? else {
            return Ok(None);
        };
        if ls != line_start {
            return Ok(None);
        }
        let target = d_start + cols;
        // The anchors of this line from the block of `start` on: the last whose text before it is
        // not past the target holds the place.
        let first_block = self.block_of(start).unwrap_or(0);
        let first = self.long_anchors.partition_point(|a| a.block < first_block);
        let slice = &self.long_anchors[first..];
        let upto = slice.partition_point(|a| a.line_start == ls && a.dec_before <= target);
        let Some(anchor) = upto.checked_sub(1).map(|i| slice[i]) else {
            return Ok(None);
        };
        let (block_start, block_end) = self.block_range(anchor.block).unwrap_or((0, 0));
        let chars = self.block_chars(anchor.block)?;
        let at = match chars.offset_of_decoded(target - anchor.dec_before) {
            Some(offset) => block_start + offset,
            // not so much text in the block: the line ends in it
            None => block_end,
        };
        Ok(Some(at.min(end).max(start)))
    }
}
