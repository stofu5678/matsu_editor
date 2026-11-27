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
//! The blocks sit in groups of `GROUP` blocks, and two Fenwick trees over the groups (lines and
//! rows) find the group of a line or of a row in O(log n); then the blocks of the group are
//! walked (at most `GROUP`), and inside a `Counts` block the counts are added up (at most
//! `2 * BLOCK` of them), inside a `Ones` block the answer is arithmetic. An edit that stays
//! inside one block moves two Fenwick entries. One that reaches over blocks or changes the kind
//! of a block (a line is found to wrap, or a distant line is laid out before the lines in front
//! of it) takes the groups that hold the touched blocks and the groups next to them apart, re-cuts
//! the touched blocks and their two neighbours (so that runs of ones join), and puts the groups
//! back: O(`GROUP` + log n) wherever it is, and a build of the trees over the groups (O(blocks /
//! `GROUP`)) only when the number of groups changes in the middle. At the end the trees grow by
//! appending.

/// The usual number of lines in a block.
const BLOCK: usize = 256;

/// The blocks in a group.
const GROUP: usize = 32;

// The steps the trees took, for the test that checks the first layout is not quadratic.
#[cfg(test)]
thread_local! {
    static TREE_WORK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn work(n: usize) {
    #[cfg(test)]
    TREE_WORK.with(|w| w.set(w.get() + n as u64));
    #[cfg(not(test))]
    let _ = n;
}

/// A Fenwick tree (binary indexed tree) of `u64` sums over the blocks.
#[derive(Clone, Debug, Default)]
struct Fenwick {
    tree: Vec<u64>,
}

impl Fenwick {
    fn build(values: impl ExactSizeIterator<Item = u64>) -> Self {
        let n = values.len();
        work(n);
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

    /// Cuts the tree back to its first `n` entries (the entries `1..=n` of the tree only hold
    /// sums over entries up to themselves, so they stay right).
    fn truncate(&mut self, n: usize) {
        if self.tree.len() > n + 1 {
            self.tree.truncate(n + 1);
        }
    }

    /// Appends an entry, O(log n).
    fn push(&mut self, value: u64) {
        if self.tree.is_empty() {
            self.tree.push(0);
        }
        let k = self.tree.len();
        // The entry `k` holds the sum over `(k - lowbit(k), k]`.
        let below = self.prefix(k - 1) - self.prefix(k - k.isolate_lowest_one());
        self.tree.push(value + below);
    }

    /// Adds `delta` to the entry `i` (0-based).
    fn add(&mut self, i: usize, delta: i64) {
        let n = self.len();
        let mut k = i + 1;
        while k <= n {
            work(1);
            self.tree[k] = self.tree[k].wrapping_add_signed(delta);
            k += k.isolate_lowest_one();
        }
    }

    /// The sum of the first `i` entries.
    fn prefix(&self, i: usize) -> u64 {
        let mut k = i;
        let mut sum = 0;
        while k > 0 {
            work(1);
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
            work(1);
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

/// Up to `2 * GROUP` blocks and the sums of their lines and rows. The Fenwick trees run over the
/// groups, so a block can be inserted or split anywhere by changing one group, and the trees
/// change in O(log n) (or are built again over the groups, O(blocks / GROUP), when the number of
/// groups changes).
#[derive(Clone, Debug, Default)]
struct Group {
    blocks: Vec<Block>,
    lines: usize,
    rows: u64,
}

impl Group {
    fn new(blocks: Vec<Block>) -> Self {
        let lines = blocks.iter().map(Block::len).sum();
        let rows = blocks.iter().map(Block::rows).sum();
        Self {
            blocks,
            lines,
            rows,
        }
    }
}

/// The row count of every line.
#[derive(Clone, Debug, Default)]
pub(crate) struct RowSeq {
    groups: Vec<Group>,
    fen_lines: Fenwick,
    fen_rows: Fenwick,
    lines: usize,
    rows: u64,
}

/// A place in the sequence: the group, the block in it, and the line in the block.
type Place = (usize, usize, usize);

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
            groups: regroup(blocks),
            ..Self::default()
        };
        seq.rebuild();
        seq
    }

    /// Builds the trees and the totals from the groups: O(groups), the blocks are not visited.
    fn rebuild(&mut self) {
        self.fen_lines = Fenwick::build(self.groups.iter().map(|g| g.lines as u64));
        self.fen_rows = Fenwick::build(self.groups.iter().map(|g| g.rows));
        self.lines = self.groups.iter().map(|g| g.lines).sum();
        self.rows = self.groups.iter().map(|g| g.rows).sum();
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
        let mut bytes = self.groups.capacity() * std::mem::size_of::<Group>()
            + (self.fen_lines.tree.capacity() + self.fen_rows.tree.capacity()) * 8;
        for group in &self.groups {
            bytes += group.blocks.capacity() * std::mem::size_of::<Block>();
            for block in &group.blocks {
                if let Block::Counts { counts, .. } = block {
                    bytes += counts.capacity() * 4;
                }
            }
        }
        bytes
    }

    /// The number of blocks (for tests).
    #[cfg(test)]
    pub(crate) fn block_count(&self) -> usize {
        self.groups.iter().map(|g| g.blocks.len()).sum()
    }

    /// The place of line `line` (`line <= len()`; the end is the end of the last block).
    fn locate_line(&self, line: usize) -> Place {
        let (g, rest) = self.fen_lines.find(line as u64);
        if g >= self.groups.len() {
            let Some(group) = self.groups.last() else {
                return (0, 0, 0);
            };
            let b = group.blocks.len() - 1;
            return (self.groups.len() - 1, b, group.blocks[b].len());
        }
        let mut rest = rest as usize;
        for (b, block) in self.groups[g].blocks.iter().enumerate() {
            work(1);
            if rest < block.len() {
                return (g, b, rest);
            }
            rest -= block.len();
        }
        unreachable!("the Fenwick tree pointed at a group that does not hold the line")
    }

    pub(crate) fn get(&self, line: usize) -> Option<u32> {
        if line >= self.lines {
            return None;
        }
        let (g, b, off) = self.locate_line(line);
        Some(match &self.groups[g].blocks[b] {
            Block::Ones(_) => 1,
            Block::Counts { counts, .. } => counts[off],
        })
    }

    /// The number of the first row of `line`; `total_rows()` for `line >= len()`.
    pub(crate) fn first_row(&self, line: usize) -> u64 {
        if line >= self.lines {
            return self.rows;
        }
        let (g, b, off) = self.locate_line(line);
        let before: u64 = self.groups[g].blocks[..b].iter().map(Block::rows).sum();
        self.fen_rows.prefix(g) + before + self.groups[g].blocks[b].rows_before(off)
    }

    /// The line that holds `row` and the index of the row in it, or `None` past the last row.
    pub(crate) fn locate_row(&self, row: u64) -> Option<(usize, usize)> {
        if row >= self.rows {
            return None;
        }
        let (g, mut rest) = self.fen_rows.find(row);
        let mut first = self.fen_lines.prefix(g) as usize;
        for block in &self.groups[g].blocks {
            work(1);
            if rest >= block.rows() {
                rest -= block.rows();
                first += block.len();
                continue;
            }
            return match block {
                Block::Ones(_) => Some((first + rest as usize, 0)),
                Block::Counts { counts, .. } => {
                    for (line, &c) in (first..).zip(counts) {
                        if rest < u64::from(c) {
                            return Some((line, rest as usize));
                        }
                        rest -= u64::from(c);
                    }
                    unreachable!("the block does not hold the row it was chosen for")
                }
            };
        }
        unreachable!("the Fenwick tree pointed at a group that does not hold the row")
    }

    /// Replaces the counts of lines `first..first + old_count` with `new`. `first + old_count`
    /// must not pass `len()`.
    ///
    /// Three ways, from the cheapest: inside one block (two Fenwick updates); the same number of
    /// lines over several blocks, as when a chunk of the first layout is stored (one Fenwick
    /// update per block); and the general case, which cuts the touched blocks and their two
    /// neighbours again, in the groups that hold them and the groups next to those. The first two
    /// only run when every block stays valid (a `Ones` block gets ones only, a `Counts` block
    /// keeps an exception); the general case is what turns a block into the other kind, splits a
    /// block, or joins blocks. It costs O(blocks in a few groups + log n), wherever the lines are,
    /// and a build of the trees over the groups when the number of groups changes (not at the
    /// end, where the trees grow by appending).
    pub(crate) fn replace(&mut self, first: usize, old_count: usize, new: &[u32]) {
        assert!(first + old_count <= self.lines, "range past the lines");
        if old_count == 0 && new.is_empty() {
            return;
        }
        let at0 = self.locate_line(first);
        let at1 = self.locate_line(first + old_count);
        if !self.groups.is_empty()
            && at0.0 == at1.0
            && at0.1 == at1.1
            && self.replace_in_block(at0, old_count, new)
        {
            return;
        }
        if new.len() == old_count && self.overwrite(at0, new) {
            return;
        }
        self.recut(at0, at1, new);
    }

    /// The general case of [`replace`](Self::replace).
    fn recut(&mut self, (g0, b0, off0): Place, (g1, b1, off1): Place, new: &[u32]) {
        let glo = g0.saturating_sub(1);
        let ghi = (g1 + 2).min(self.groups.len());
        let old_groups = self.groups.len();
        let drained: Vec<Group> = self.groups.drain(glo..ghi).collect();
        let old_sums: Vec<(usize, u64)> = drained.iter().map(|g| (g.lines, g.rows)).collect();
        let mut flat: Vec<Block> = Vec::new();
        let (mut bb0, mut bb1) = (0, 0);
        for (k, group) in drained.into_iter().enumerate() {
            work(group.blocks.len());
            if glo + k < g0 {
                bb0 += group.blocks.len();
            }
            if glo + k < g1 {
                bb1 += group.blocks.len();
            }
            flat.extend(group.blocks);
        }
        bb0 += b0;
        bb1 += b1;
        // Cut the block of the start, the block of the end and their neighbours again.
        let lo = bb0.saturating_sub(1);
        let hi = (bb1 + 2).min(flat.len());
        let mut builder = Builder::default();
        for block in flat.get(lo..bb0).unwrap_or_default() {
            builder.feed(block, 0, block.len());
        }
        if let Some(block) = flat.get(bb0) {
            builder.feed(block, 0, off0);
        }
        builder.counts(new);
        if let Some(block) = flat.get(bb1) {
            builder.feed(block, off1, block.len());
        }
        for block in flat.get(bb1 + 1..hi).unwrap_or_default() {
            builder.feed(block, 0, block.len());
        }
        let pieces = builder.finish();
        flat.splice(lo.min(hi)..hi, pieces);
        let fresh = regroup(flat);
        let new_sums: Vec<(usize, u64)> = fresh.iter().map(|g| (g.lines, g.rows)).collect();
        self.groups.splice(glo..glo, fresh);
        let (old_lines, old_rows) = old_sums
            .iter()
            .fold((0usize, 0u64), |(l, r), &(gl, gr)| (l + gl, r + gr));
        let (new_lines, new_rows) = new_sums
            .iter()
            .fold((0usize, 0u64), |(l, r), &(gl, gr)| (l + gl, r + gr));
        if new_sums.len() == old_sums.len() {
            // The same groups: move the entries of the trees.
            for (k, (o, n)) in old_sums.iter().zip(&new_sums).enumerate() {
                self.fen_lines.add(glo + k, n.0 as i64 - o.0 as i64);
                self.fen_rows.add(glo + k, n.1 as i64 - o.1 as i64);
            }
        } else if ghi == old_groups {
            // The change reaches the last group, as when the first layout stores its chunks one
            // after the other in front of the one run of ones that is left: cut the trees back
            // and append the new groups, O(log n) each.
            self.fen_lines.truncate(glo);
            self.fen_rows.truncate(glo);
            for &(l, r) in &new_sums {
                self.fen_lines.push(l as u64);
                self.fen_rows.push(r);
            }
        } else {
            self.fen_lines = Fenwick::build(self.groups.iter().map(|g| g.lines as u64));
            self.fen_rows = Fenwick::build(self.groups.iter().map(|g| g.rows));
        }
        self.lines = self.lines - old_lines + new_lines;
        self.rows = self.rows - old_rows + new_rows;
    }

    /// The edit inside one block, when the block stays valid. Returns whether it was done.
    fn replace_in_block(&mut self, (g, b, off): Place, old_count: usize, new: &[u32]) -> bool {
        let delta_lines = new.len() as i64 - old_count as i64;
        let delta_rows = match &mut self.groups[g].blocks[b] {
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
        let group = &mut self.groups[g];
        group.lines = group.lines.wrapping_add_signed(delta_lines as isize);
        group.rows = group.rows.wrapping_add_signed(delta_rows);
        self.fen_lines.add(g, delta_lines);
        self.fen_rows.add(g, delta_rows);
        self.lines = self.lines.wrapping_add_signed(delta_lines as isize);
        self.rows = self.rows.wrapping_add_signed(delta_rows);
        true
    }

    /// Writes `new` over the same number of lines from `at` on, block by block, when every block
    /// stays valid. Returns whether it was done; nothing changes when not.
    fn overwrite(&mut self, (g0, b0, off0): Place, new: &[u32]) -> bool {
        // First pass: would every block stay valid?
        let (mut g, mut b, mut off, mut done) = (g0, b0, off0, 0);
        while done < new.len() {
            let Some(group) = self.groups.get(g) else {
                return false;
            };
            let Some(block) = group.blocks.get(b) else {
                g += 1;
                b = 0;
                continue;
            };
            let n = (block.len() - off).min(new.len() - done);
            if n == 0 {
                return false;
            }
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
        let (mut g, mut b, mut off, mut done) = (g0, b0, off0, 0);
        while done < new.len() {
            if b >= self.groups[g].blocks.len() {
                g += 1;
                b = 0;
                continue;
            }
            let block = &mut self.groups[g].blocks[b];
            let n = (block.len() - off).min(new.len() - done);
            let part = &new[done..done + n];
            if let Block::Counts { counts, rows } = block {
                let delta_rows = sum(part) as i64 - sum(&counts[off..off + n]) as i64;
                counts[off..off + n].copy_from_slice(part);
                *rows = rows.wrapping_add_signed(delta_rows);
                let group = &mut self.groups[g];
                group.rows = group.rows.wrapping_add_signed(delta_rows);
                self.fen_rows.add(g, delta_rows);
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
        self.groups
            .iter()
            .flat_map(|g| g.blocks.iter())
            .flat_map(|b| match b {
                Block::Ones(n) => vec![1; *n],
                Block::Counts { counts, .. } => counts.clone(),
            })
            .collect()
    }
}

/// Cuts the blocks into groups of `GROUP` blocks (the last one may have fewer).
fn regroup(blocks: Vec<Block>) -> Vec<Group> {
    let mut groups = Vec::with_capacity(blocks.len().div_ceil(GROUP));
    let mut blocks = blocks.into_iter().peekable();
    while blocks.peek().is_some() {
        groups.push(Group::new(blocks.by_ref().take(GROUP).collect()));
    }
    groups
}

fn sum(counts: &[u32]) -> u64 {
    counts.iter().map(|&c| u64::from(c)).sum()
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

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

    /// How lines far ahead of the layout become exact while it goes on (`make_exact`, after a
    /// jump or a drag of the scroll bar).
    #[derive(Clone, Copy, Debug)]
    enum Ahead {
        /// No line.
        Nothing,
        /// One line, 1000 lines from the end.
        One,
        /// A line at a random place ahead of the layout, before every other chunk.
        Random,
    }

    /// The steps (of the trees and of the walks over blocks) of laying out `lines` lines that all
    /// wrap, chunk by chunk from the top as `layout_for` does: each chunk of 512 lines is stored
    /// over the front of the lines that are left.
    fn first_layout_work(lines: usize, ahead: Ahead) -> u64 {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let mut seq = RowSeq::ones(lines);
        TREE_WORK.with(|w| w.set(0));
        if let Ahead::One = ahead {
            seq.replace(lines - 1000, 1, &[2]);
        }
        let mut done = 0;
        let mut step = 0;
        while done < lines {
            let n = 512.min(lines - done);
            if matches!(ahead, Ahead::Random) && step % 2 == 0 && done + n < lines {
                let at = done + n + rng.below(lines - done - n);
                seq.replace(at, 1, &[2]);
            }
            seq.replace(done, n, &vec![2; n]);
            done += n;
            step += 1;
        }
        assert_eq!(seq.len(), lines);
        assert_eq!(seq.total_rows(), 2 * lines as u64);
        assert_eq!(seq.first_row(lines - 1), 2 * (lines as u64 - 1));
        assert_eq!(seq.locate_row(2 * lines as u64 - 1), Some((lines - 1, 1)));
        TREE_WORK.with(Cell::get)
    }

    /// The first layout of a document that wraps in every line takes work in proportion to the
    /// lines (times a log), not to their square: four times the lines may take much less than 16
    /// times the work, with nothing laid out ahead of the layout and with lines that are.
    #[test]
    fn the_first_layout_of_a_wrapped_document_is_not_quadratic() {
        for ahead in [Ahead::Nothing, Ahead::One, Ahead::Random] {
            let small = first_layout_work(40_000, ahead);
            let large = first_layout_work(160_000, ahead);
            eprintln!("{ahead:?}: {small} steps for 40,000 lines, {large} for 160,000");
            assert!(
                large < small * 6,
                "{ahead:?}: {small} steps for 40,000 lines, {large} for 160,000"
            );
        }
    }

    #[test]
    fn the_trees_after_edits_are_the_trees_of_a_rebuild() {
        // Edits keep the trees equal to the trees built from the groups.
        let mut seq = RowSeq::ones(5000);
        let mut model = vec![1u32; 5000];
        for k in 0..40 {
            let at = k * 100;
            seq.replace(at, 60, &[2; 60]);
            model[at..at + 60].fill(2);
            check(&seq, &model);
            let lines = Fenwick::build(seq.groups.iter().map(|g| g.lines as u64));
            let rows = Fenwick::build(seq.groups.iter().map(|g| g.rows));
            assert_eq!(seq.fen_lines.tree, lines.tree);
            assert_eq!(seq.fen_rows.tree, rows.tree);
        }
    }
}
