//! The sequence of row counts, one per logical line: replace a range of lines, find the first
//! row of a line, and find the line of a row, all without touching the lines that do not change.
//!
//! The lines sit in blocks of two kinds. A `Ones` block is a run of lines of one row each and
//! holds only its length; a `Counts` block holds the counts of at most `2 * BLOCK` lines, at least
//! one of them not 1. So the memory follows the lines that have more than one row (or fewer),
//! not the number of lines: a document in which no line wraps is a single block of a few bytes,
//! whether it has a thousand lines or ten billion (ADR-0020, chapter 6, plan B). Runs of ones
//! shorter than `BLOCK` inside a `Counts` block cost 4 bytes per line, so the blocks hold at
//! most `BLOCK` ones for each exception, and the number of blocks is at most twice the number of
//! exceptions plus one.
//!
//! Two Fenwick trees over the blocks (lines and rows) find the block of a line or of a row in
//! O(log n); inside a `Counts` block the counts are added up (at most `2 * BLOCK` of them), inside
//! a `Ones` block the answer is arithmetic. An edit that stays inside one block moves two Fenwick
//! entries; one that reaches over blocks or changes the kind of a block re-cuts the blocks it
//! touches and their two neighbours (so that runs of ones join) and builds the trees again, which
//! is O(blocks) and happens when lines are pasted or deleted in bulk, or when a line is found to
//! wrap.

/// The usual number of lines in a block.
const BLOCK: usize = 256;

/// A Fenwick tree (binary indexed tree) of `u64` sums over the blocks.
#[derive(Clone, Debug, Default)]
struct Fenwick {
    tree: Vec<u64>,
}

impl Fenwick {
    fn build(values: impl ExactSizeIterator<Item = u64>) -> Self {
        let n = values.len();
        let mut tree = vec![0; n + 1];
        for (i, v) in values.enumerate() {
            let k = i + 1;
            tree[k] += v;
            let parent = k + k.isolate_lowest_one();
            if parent <= n {
                let carry = tree[k];
                tree[parent] += carry;
            }
        }
        Self { tree }
    }

    fn len(&self) -> usize {
        self.tree.len().saturating_sub(1)
    }

    /// Adds `delta` to the entry `i` (0-based).
    fn add(&mut self, i: usize, delta: i64) {
        let n = self.len();
        let mut k = i + 1;
        while k <= n {
            self.tree[k] = self.tree[k].wrapping_add_signed(delta);
            k += k.isolate_lowest_one();
        }
    }

    /// The sum of the first `i` entries.
    fn prefix(&self, i: usize) -> u64 {
        let mut k = i;
        let mut sum = 0;
        while k > 0 {
            sum += self.tree[k];
            k -= k.isolate_lowest_one();
        }
        sum
    }

    /// The entry that holds `target` when the entries are laid end to end: `(index, rest)` with
    /// `prefix(index) + rest == target` and `rest` less than the entry. The index is `len()` when
    /// `target` is the sum of all entries or more.
    fn find(&self, target: u64) -> (usize, u64) {
        let n = self.len();
        let mut pos = 0;
        let mut rest = target;
        let mut step = n.next_power_of_two();
        while step > 0 {
            let next = pos + step;
            if next <= n && self.tree[next] <= rest {
                pos = next;
                rest -= self.tree[next];
            }
            step >>= 1;
        }
        (pos, rest)
    }
}

/// A part of the sequence: a run of lines of one row each, or a few lines with their counts.
///
/// A `Ones` block holds only its length, so a document in which no line wraps is one block
/// whatever its number of lines. A `Counts` block always has at least one line of another row
/// count (see [`Builder`]), so the number of blocks follows the number of exceptions.
#[derive(Clone, Debug)]
enum Block {
    Ones(usize),
    Counts { counts: Vec<u32>, rows: u64 },
}

impl Block {
    fn counts(counts: Vec<u32>) -> Self {
        let rows = sum(&counts);
        Self::Counts { counts, rows }
    }

    fn len(&self) -> usize {
        match self {
            Self::Ones(n) => *n,
            Self::Counts { counts, .. } => counts.len(),
        }
    }

    fn rows(&self) -> u64 {
        match self {
            Self::Ones(n) => *n as u64,
            Self::Counts { rows, .. } => *rows,
        }
    }

    /// The rows of the first `off` lines.
    fn rows_before(&self, off: usize) -> u64 {
        match self {
            Self::Ones(_) => off as u64,
            Self::Counts { counts, .. } => sum(&counts[..off]),
        }
    }
}

/// Cuts a stream of counts into blocks: a run of `BLOCK` ones or more becomes a `Ones` block (and
/// joins the ones next to it); everything else is cut into `Counts` blocks of at most `BLOCK`
/// lines, and each of them holds an exception (a run of ones inside is shorter than `BLOCK`).
#[derive(Default)]
struct Builder {
    out: Vec<Block>,
    /// Counts that wait for a block. Ends with an exception.
    buf: Vec<u32>,
    /// Ones seen since the last exception.
    run: usize,
}

impl Builder {
    fn counts(&mut self, counts: &[u32]) {
        for &c in counts {
            if c == 1 {
                self.run += 1;
            } else {
                self.exception(c);
            }
        }
    }

    fn exception(&mut self, c: u32) {
        if self.run >= BLOCK {
            self.flush();
            self.out.push(Block::Ones(self.run));
        } else {
            self.buf.resize(self.buf.len() + self.run, 1);
        }
        self.run = 0;
        self.buf.push(c);
        while self.buf.len() >= BLOCK {
            let rest = self.buf.split_off(BLOCK);
            let full = std::mem::replace(&mut self.buf, rest);
            self.out.push(Block::counts(full));
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            self.out.push(Block::counts(std::mem::take(&mut self.buf)));
        }
    }

    /// Adds the lines `from..to` of `block`.
    fn feed(&mut self, block: &Block, from: usize, to: usize) {
        match block {
            Block::Ones(_) => self.run += to - from,
            Block::Counts { counts, .. } => self.counts(&counts[from..to]),
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush();
        if self.run > 0 {
            self.out.push(Block::Ones(self.run));
        }
        self.out
    }
}

/// The row count of every line.
#[derive(Clone, Debug, Default)]
pub(crate) struct RowSeq {
    blocks: Vec<Block>,
    fen_lines: Fenwick,
    fen_rows: Fenwick,
    lines: usize,
    rows: u64,
}

impl RowSeq {
    /// `lines` lines of one row each: one block.
    pub(crate) fn ones(lines: usize) -> Self {
        let blocks = if lines == 0 {
            Vec::new()
        } else {
            vec![Block::Ones(lines)]
        };
        Self::from_blocks(blocks)
    }

    #[cfg(test)]
    pub(crate) fn from_counts(counts: &[u32]) -> Self {
        let mut builder = Builder::default();
        builder.counts(counts);
        Self::from_blocks(builder.finish())
    }

    fn from_blocks(blocks: Vec<Block>) -> Self {
        let mut seq = Self {
            blocks,
            ..Self::default()
        };
        seq.rebuild();
        seq
    }

    /// Builds the trees and the totals from the blocks: O(blocks), the lines are not visited.
    fn rebuild(&mut self) {
        self.fen_lines = Fenwick::build(self.blocks.iter().map(|b| b.len() as u64));
        self.fen_rows = Fenwick::build(self.blocks.iter().map(Block::rows));
        self.lines = self.blocks.iter().map(Block::len).sum();
        self.rows = self.blocks.iter().map(Block::rows).sum();
    }

    pub(crate) fn len(&self) -> usize {
        self.lines
    }

    /// The sum of all counts.
    pub(crate) fn total_rows(&self) -> u64 {
        self.rows
    }

    /// The bytes this sequence holds on the heap (what its capacity reserves).
    #[cfg(test)]
    pub(crate) fn heap_bytes(&self) -> usize {
        let blocks = self.blocks.capacity() * std::mem::size_of::<Block>();
        let counts: usize = self
            .blocks
            .iter()
            .map(|b| match b {
                Block::Ones(_) => 0,
                Block::Counts { counts, .. } => counts.capacity() * 4,
            })
            .sum();
        blocks + counts + (self.fen_lines.tree.capacity() + self.fen_rows.tree.capacity()) * 8
    }

    /// The number of blocks (for tests).
    #[cfg(test)]
    pub(crate) fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// The block and the index in it of line `line` (`line <= len()`; the end is the end of the
    /// last block).
    fn locate_line(&self, line: usize) -> (usize, usize) {
        let (b, off) = self.fen_lines.find(line as u64);
        if b >= self.blocks.len() {
            let last = self.blocks.len().saturating_sub(1);
            (last, self.blocks.get(last).map_or(0, Block::len))
        } else {
            (b, off as usize)
        }
    }

    pub(crate) fn get(&self, line: usize) -> Option<u32> {
        if line >= self.lines {
            return None;
        }
        let (b, off) = self.locate_line(line);
        Some(match &self.blocks[b] {
            Block::Ones(_) => 1,
            Block::Counts { counts, .. } => counts[off],
        })
    }

    /// The number of the first row of `line`; `total_rows()` for `line >= len()`.
    pub(crate) fn first_row(&self, line: usize) -> u64 {
        if line >= self.lines {
            return self.rows;
        }
        let (b, off) = self.locate_line(line);
        self.fen_rows.prefix(b) + self.blocks[b].rows_before(off)
    }

    /// The line that holds `row` and the index of the row in it, or `None` past the last row.
    pub(crate) fn locate_row(&self, row: u64) -> Option<(usize, usize)> {
        if row >= self.rows {
            return None;
        }
        let (b, mut rest) = self.fen_rows.find(row);
        let first = self.fen_lines.prefix(b) as usize;
        match &self.blocks[b] {
            Block::Ones(_) => Some((first + rest as usize, 0)),
            Block::Counts { counts, .. } => {
                for (line, &c) in (first..).zip(counts) {
                    if rest < u64::from(c) {
                        return Some((line, rest as usize));
                    }
                    rest -= u64::from(c);
                }
                unreachable!("the Fenwick tree pointed at a block that does not hold the row")
            }
        }
    }

    /// Replaces the counts of lines `first..first + old_count` with `new`. `first + old_count`
    /// must not pass `len()`.
    ///
    /// Three ways, from the cheapest: inside one block (two Fenwick updates); the same number of
    /// lines over several blocks, as when a chunk of the first layout is stored (one Fenwick
    /// update per block); and the general case, which cuts the touched blocks and their two
    /// neighbours again and builds the trees from the block sums. The first two only run when
    /// every block stays valid (a `Ones` block gets ones only, a `Counts` block keeps an
    /// exception); the general case is what turns a block into the other kind or joins blocks.
    pub(crate) fn replace(&mut self, first: usize, old_count: usize, new: &[u32]) {
        assert!(first + old_count <= self.lines, "range past the lines");
        if old_count == 0 && new.is_empty() {
            return;
        }
        let (b0, off0) = self.locate_line(first);
        let (b1, off1) = self.locate_line(first + old_count);
        if !self.blocks.is_empty() && b0 == b1 && self.replace_in_block(b0, off0, old_count, new) {
            return;
        }
        if new.len() == old_count && self.overwrite(b0, off0, new) {
            return;
        }
        // The general case: cut the touched blocks and their neighbours again.
        let lo = b0.saturating_sub(1);
        let hi = (b1 + 2).min(self.blocks.len());
        let mut builder = Builder::default();
        for block in self.blocks.get(lo..b0).unwrap_or_default() {
            builder.feed(block, 0, block.len());
        }
        if let Some(block) = self.blocks.get(b0) {
            builder.feed(block, 0, off0);
        }
        builder.counts(new);
        if let Some(block) = self.blocks.get(b1) {
            builder.feed(block, off1, block.len());
        }
        for block in self.blocks.get(b1 + 1..hi).unwrap_or_default() {
            builder.feed(block, 0, block.len());
        }
        let pieces = builder.finish();
        self.blocks.splice(lo.min(hi)..hi, pieces);
        self.rebuild();
    }

    /// The edit inside block `b`, when the block stays valid. Returns whether it was done.
    fn replace_in_block(&mut self, b: usize, off: usize, old_count: usize, new: &[u32]) -> bool {
        let delta_lines = new.len() as i64 - old_count as i64;
        let delta_rows = match &mut self.blocks[b] {
            Block::Ones(len) => {
                let new_len = *len - old_count + new.len();
                if new_len == 0 || sum(new) != new.len() as u64 {
                    return false;
                }
                *len = new_len;
                delta_lines
            }
            Block::Counts { counts, rows } => {
                let new_len = counts.len() - old_count + new.len();
                let removed = sum(&counts[off..off + old_count]);
                let new_rows = *rows - removed + sum(new);
                if !(1..=2 * BLOCK).contains(&new_len) || new_rows == new_len as u64 {
                    return false;
                }
                counts.splice(off..off + old_count, new.iter().copied());
                let delta = new_rows as i64 - *rows as i64;
                *rows = new_rows;
                delta
            }
        };
        self.fen_lines.add(b, delta_lines);
        self.fen_rows.add(b, delta_rows);
        self.lines = self.lines.wrapping_add_signed(delta_lines as isize);
        self.rows = self.rows.wrapping_add_signed(delta_rows);
        true
    }

    /// Writes `new` over the same number of lines from `(b0, off0)` on, block by block, when
    /// every block stays valid. Returns whether it was done; nothing changes when not.
    fn overwrite(&mut self, b0: usize, off0: usize, new: &[u32]) -> bool {
        // First pass: would every block stay valid?
        let (mut b, mut off, mut done) = (b0, off0, 0);
        while done < new.len() {
            let Some(block) = self.blocks.get(b) else {
                return false;
            };
            let n = (block.len() - off).min(new.len() - done);
            let part = &new[done..done + n];
            let ok = match block {
                Block::Ones(_) => sum(part) == n as u64,
                Block::Counts { counts, rows } => {
                    *rows - sum(&counts[off..off + n]) + sum(part) != counts.len() as u64
                }
            };
            if !ok {
                return false;
            }
            done += n;
            b += 1;
            off = 0;
        }
        // Second pass: write.
        let (mut b, mut off, mut done) = (b0, off0, 0);
        while done < new.len() {
            let block = &mut self.blocks[b];
            let n = (block.len() - off).min(new.len() - done);
            let part = &new[done..done + n];
            if let Block::Counts { counts, rows } = block {
                let delta_rows = sum(part) as i64 - sum(&counts[off..off + n]) as i64;
                counts[off..off + n].copy_from_slice(part);
                *rows = rows.wrapping_add_signed(delta_rows);
                self.fen_rows.add(b, delta_rows);
                self.rows = self.rows.wrapping_add_signed(delta_rows);
            }
            done += n;
            b += 1;
            off = 0;
        }
        true
    }

    /// The counts, one by one (for tests).
    #[cfg(test)]
    pub(crate) fn to_vec(&self) -> Vec<u32> {
        self.blocks
            .iter()
            .flat_map(|b| match b {
                Block::Ones(n) => vec![1; *n],
                Block::Counts { counts, .. } => counts.clone(),
            })
            .collect()
    }
}

fn sum(counts: &[u32]) -> u64 {
    counts.iter().map(|&c| u64::from(c)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(seq: &RowSeq, expect: &[u32]) {
        assert_eq!(seq.to_vec(), expect);
        assert_eq!(seq.len(), expect.len());
        assert_eq!(seq.total_rows(), expect.iter().map(|&c| u64::from(c)).sum());
        let mut row = 0u64;
        for (line, &c) in expect.iter().enumerate() {
            assert_eq!(seq.get(line), Some(c));
            assert_eq!(seq.first_row(line), row);
            for k in 0..c as usize {
                assert_eq!(seq.locate_row(row + k as u64), Some((line, k)));
            }
            row += u64::from(c);
        }
        assert_eq!(seq.get(expect.len()), None);
        assert_eq!(seq.first_row(expect.len()), row);
        assert_eq!(seq.locate_row(row), None);
    }

    struct Rng(u64);
    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }
    }

    #[test]
    fn empty_and_small() {
        let mut seq = RowSeq::default();
        check(&seq, &[]);
        seq.replace(0, 0, &[2, 1, 3]);
        check(&seq, &[2, 1, 3]);
        seq.replace(1, 1, &[]);
        check(&seq, &[2, 3]);
        seq.replace(0, 2, &[]);
        check(&seq, &[]);
        seq.replace(0, 0, &[4]);
        check(&seq, &[4]);
    }

    #[test]
    fn random_replacements_match_a_vec() {
        for seed in 1..=6u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let mut model: Vec<u32> = (0..rng.below(2000))
                .map(|_| 1 + rng.below(4) as u32)
                .collect();
            let mut seq = RowSeq::from_counts(&model);
            check(&seq, &model);
            for step in 0..300 {
                let first = rng.below(model.len() + 1);
                let max_old = (model.len() - first).min(if step % 17 == 0 { 3000 } else { 40 });
                let old = rng.below(max_old + 1);
                let n = if step % 23 == 0 {
                    rng.below(1500)
                } else {
                    rng.below(40)
                };
                let new: Vec<u32> = (0..n).map(|_| 1 + rng.below(5) as u32).collect();
                model.splice(first..first + old, new.iter().copied());
                seq.replace(first, old, &new);
                if step % 10 == 0 || model.len() < 50 {
                    check(&seq, &model);
                }
            }
            check(&seq, &model);
        }
    }

    #[test]
    fn same_length_replacement_over_many_blocks() {
        let mut model: Vec<u32> = (0..3000).map(|i| 1 + (i % 3) as u32).collect();
        let mut seq = RowSeq::from_counts(&model);
        let new = vec![5; 1000];
        model.splice(100..1100, new.iter().copied());
        seq.replace(100, 1000, &new);
        check(&seq, &model);
        let new = vec![2; 2900];
        model.splice(100..3000, new.iter().copied());
        seq.replace(100, 2900, &new);
        check(&seq, &model);
    }

    #[test]
    fn ones_is_one_row_per_line() {
        let seq = RowSeq::ones(1000);
        check(&seq, &vec![1; 1000]);
    }

    /// The number of blocks stays within twice the exceptions plus one.
    fn check_blocks(seq: &RowSeq, model: &[u32]) {
        let exceptions = model.iter().filter(|&&c| c != 1).count();
        assert!(
            seq.block_count() <= 2 * exceptions + 1,
            "{} blocks for {exceptions} exceptions",
            seq.block_count()
        );
    }

    /// A model with long runs of ones and a few lines of other counts.
    fn sparse_model(rng: &mut Rng, len: usize, exceptions: usize) -> Vec<u32> {
        let mut model = vec![1; len];
        for _ in 0..exceptions {
            let at = rng.below(len.max(1));
            if at < model.len() {
                model[at] = 2 + rng.below(4) as u32;
            }
        }
        model
    }

    #[test]
    fn random_replacements_over_long_runs_of_ones() {
        for seed in 1..=4u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut model = sparse_model(&mut rng, 12_000, 12);
            let mut seq = RowSeq::from_counts(&model);
            check(&seq, &model);
            check_blocks(&seq, &model);
            for step in 0..400 {
                let first = rng.below(model.len() + 1);
                let max_old = (model.len() - first).min(match step % 11 {
                    0 => 6000,
                    1..=4 => 300,
                    _ => 5,
                });
                let old = rng.below(max_old + 1);
                // Mostly ones; sometimes a lone exception; sometimes a long run.
                let n = match step % 7 {
                    0 => rng.below(2000),
                    1 => 1,
                    _ => rng.below(10),
                };
                let new: Vec<u32> = (0..n)
                    .map(|_| {
                        if rng.below(40) == 0 {
                            2 + rng.below(3) as u32
                        } else {
                            1
                        }
                    })
                    .collect();
                model.splice(first..first + old, new.iter().copied());
                seq.replace(first, old, &new);
                if step % 5 == 0 || model.len() < 50 {
                    check(&seq, &model);
                }
                check_blocks(&seq, &model);
            }
            check(&seq, &model);
            // Back to one row each everywhere: one block again.
            let n = model.len();
            let ones = vec![1; n];
            seq.replace(0, n, &ones);
            check(&seq, &ones);
            assert_eq!(seq.block_count(), 1.min(n));
        }
    }

    #[test]
    fn same_length_overwrite_keeps_blocks_valid() {
        let mut model = vec![1u32; 5000];
        model[2000] = 3;
        let mut seq = RowSeq::from_counts(&model);
        let before = seq.block_count();
        // The same ones over the runs: nothing changes.
        seq.replace(100, 1500, &[1; 1500]);
        assert_eq!(seq.block_count(), before);
        check(&seq, &model);
        // An exception inside a run of ones splits it.
        seq.replace(100, 3, &[1, 4, 1]);
        model[101] = 4;
        check(&seq, &model);
        // Taking the only exception of a block away turns it into ones and joins the neighbours.
        seq.replace(2000, 1, &[1]);
        model[2000] = 1;
        check(&seq, &model);
        check_blocks(&seq, &model);
    }

    #[test]
    fn ones_hold_no_memory_per_line() {
        // 10^8 lines, none of them wraps: the sequence is one block.
        let mut seq = RowSeq::ones(100_000_000);
        assert!(seq.heap_bytes() < 1024, "{} bytes", seq.heap_bytes());
        assert_eq!(seq.block_count(), 1);
        assert_eq!(seq.total_rows(), 100_000_000);
        assert_eq!(seq.first_row(99_999_999), 99_999_999);
        assert_eq!(seq.locate_row(54_321), Some((54_321, 0)));
        // A line in the middle wraps into 3 rows, and then it is edited and wraps no more.
        seq.replace(50_000_000, 1, &[3]);
        assert_eq!(seq.block_count(), 3);
        assert!(seq.heap_bytes() < 4096, "{} bytes", seq.heap_bytes());
        assert_eq!(seq.total_rows(), 100_000_002);
        assert_eq!(seq.first_row(50_000_001), 50_000_003);
        assert_eq!(seq.locate_row(50_000_002), Some((50_000_000, 2)));
        assert_eq!(seq.locate_row(50_000_003), Some((50_000_001, 0)));
        seq.replace(50_000_000, 1, &[1]);
        assert_eq!(seq.block_count(), 1);
        assert_eq!(seq.total_rows(), 100_000_000);
        // Lines inserted and removed far from any exception.
        seq.replace(7, 0, &[1; 100]);
        seq.replace(0, 1_000_000, &[]);
        assert_eq!(seq.len(), 99_000_100);
        assert_eq!(seq.block_count(), 1);
    }
}
