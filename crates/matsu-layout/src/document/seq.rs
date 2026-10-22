//! The sequence of row counts, one per logical line: replace a range of lines, find the first
//! row of a line, and find the line of a row, all without touching the lines that do not change.
//!
//! The counts sit in blocks of at most `2 * BLOCK` lines. Two Fenwick trees over the blocks
//! (lines and rows) find the block of a line or of a row in O(log n); inside the block the
//! counts are added up (at most `2 * BLOCK` of them). An edit that stays inside one block moves
//! two Fenwick entries; one that reaches over blocks re-cuts the blocks it touches and builds the
//! trees again, which is O(blocks) and happens when lines are pasted or deleted in bulk.
//!
//! The counts take 4 bytes per line (the 1M-line document of the performance report uses 4 MB).

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

/// Up to `2 * BLOCK` counts and their sum.
#[derive(Clone, Debug)]
struct Block {
    counts: Vec<u32>,
    rows: u64,
}

impl Block {
    fn new(counts: Vec<u32>) -> Self {
        let rows = sum(&counts);
        Self { counts, rows }
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
    /// `lines` lines of one row each.
    pub(crate) fn ones(lines: usize) -> Self {
        let mut blocks = Vec::new();
        let mut left = lines;
        while left > 0 {
            let n = left.min(BLOCK);
            blocks.push(Block::new(vec![1; n]));
            left -= n;
        }
        Self::from_blocks(blocks)
    }

    #[cfg(test)]
    pub(crate) fn from_counts(counts: &[u32]) -> Self {
        Self::from_blocks(
            counts
                .chunks(BLOCK)
                .map(|c| Block::new(c.to_vec()))
                .collect(),
        )
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
        self.fen_lines = Fenwick::build(self.blocks.iter().map(|b| b.counts.len() as u64));
        self.fen_rows = Fenwick::build(self.blocks.iter().map(|b| b.rows));
        self.lines = self.blocks.iter().map(|b| b.counts.len()).sum();
        self.rows = self.blocks.iter().map(|b| b.rows).sum();
    }

    pub(crate) fn len(&self) -> usize {
        self.lines
    }

    /// The sum of all counts.
    pub(crate) fn total_rows(&self) -> u64 {
        self.rows
    }

    /// The block and the index in it of line `line` (`line <= len()`; the end is the end of the
    /// last block).
    fn locate_line(&self, line: usize) -> (usize, usize) {
        let (b, off) = self.fen_lines.find(line as u64);
        if b >= self.blocks.len() {
            let last = self.blocks.len().saturating_sub(1);
            (last, self.blocks.get(last).map_or(0, |b| b.counts.len()))
        } else {
            (b, off as usize)
        }
    }

    pub(crate) fn get(&self, line: usize) -> Option<u32> {
        if line >= self.lines {
            return None;
        }
        let (b, off) = self.locate_line(line);
        Some(self.blocks[b].counts[off])
    }

    /// The number of the first row of `line`; `total_rows()` for `line >= len()`.
    pub(crate) fn first_row(&self, line: usize) -> u64 {
        if line >= self.lines {
            return self.rows;
        }
        let (b, off) = self.locate_line(line);
        self.fen_rows.prefix(b) + sum(&self.blocks[b].counts[..off])
    }

    /// The line that holds `row` and the index of the row in it, or `None` past the last row.
    pub(crate) fn locate_row(&self, row: u64) -> Option<(usize, usize)> {
        if row >= self.rows {
            return None;
        }
        let (b, mut rest) = self.fen_rows.find(row);
        let first = self.fen_lines.prefix(b) as usize;
        for (line, &c) in (first..).zip(&self.blocks[b].counts) {
            if rest < u64::from(c) {
                return Some((line, rest as usize));
            }
            rest -= u64::from(c);
        }
        unreachable!("the Fenwick tree pointed at a block that does not hold the row")
    }

    /// Replaces the counts of lines `first..first + old_count` with `new`. `first + old_count`
    /// must not pass `len()`.
    ///
    /// Three ways, from the cheapest: inside one block (two Fenwick updates); the same number of
    /// lines over several blocks, as when a chunk of the first layout is stored (one Fenwick
    /// update per block); and the general case, which cuts the blocks again and builds the trees
    /// from the block sums.
    pub(crate) fn replace(&mut self, first: usize, old_count: usize, new: &[u32]) {
        assert!(first + old_count <= self.lines, "range past the lines");
        if old_count == 0 && new.is_empty() {
            return;
        }
        let (b0, off0) = self.locate_line(first);
        let (b1, off1) = self.locate_line(first + old_count);
        if !self.blocks.is_empty() && b0 == b1 {
            let block = &mut self.blocks[b0];
            let new_len = block.counts.len() - old_count + new.len();
            if (1..=2 * BLOCK).contains(&new_len) {
                let removed = sum(&block.counts[off0..off0 + old_count]);
                block
                    .counts
                    .splice(off0..off0 + old_count, new.iter().copied());
                let delta_rows = sum(new) as i64 - removed as i64;
                block.rows = block.rows.wrapping_add_signed(delta_rows);
                self.fen_lines.add(b0, new.len() as i64 - old_count as i64);
                self.fen_rows.add(b0, delta_rows);
                self.lines = self.lines - old_count + new.len();
                self.rows = self.rows.wrapping_add_signed(delta_rows);
                return;
            }
        }
        if new.len() == old_count {
            // Overwrite in place, block by block.
            let (mut b, mut off, mut done) = (b0, off0, 0);
            while done < new.len() {
                let block = &mut self.blocks[b];
                let n = (block.counts.len() - off).min(new.len() - done);
                let part = &new[done..done + n];
                let delta_rows = sum(part) as i64 - sum(&block.counts[off..off + n]) as i64;
                block.counts[off..off + n].copy_from_slice(part);
                block.rows = block.rows.wrapping_add_signed(delta_rows);
                self.fen_rows.add(b, delta_rows);
                self.rows = self.rows.wrapping_add_signed(delta_rows);
                done += n;
                b += 1;
                off = 0;
            }
            return;
        }
        // The general case: cut the touched blocks again.
        let mut joined: Vec<u32> = Vec::new();
        if let Some(block) = self.blocks.get(b0) {
            joined.extend_from_slice(&block.counts[..off0]);
        }
        joined.extend_from_slice(new);
        if let Some(block) = self.blocks.get(b1) {
            joined.extend_from_slice(&block.counts[off1..]);
        }
        let end = (b1 + 1).min(self.blocks.len());
        let pieces = joined.chunks(BLOCK).map(|c| Block::new(c.to_vec()));
        self.blocks.splice(b0.min(end)..end, pieces);
        self.rebuild();
    }

    /// The counts, one by one (for tests).
    #[cfg(test)]
    pub(crate) fn to_vec(&self) -> Vec<u32> {
        self.blocks
            .iter()
            .flat_map(|b| b.counts.iter().copied())
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
}
