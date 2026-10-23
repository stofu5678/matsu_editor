// Derived from Sakura Editor (zlib License):
//   sakura_core/doc/layout/CLayoutMgr.cpp (the line counts, the EOF row, LogicToLayout and
//   LayoutToLogicEx for the whole document),
//   sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp (_DoLayout, DoLayout_Range: lay out again only
//   the lines an edit touched)
// Modified: rewritten in Rust. Sakura keeps a linked list of `CLayout` rows and walks it; this
// keeps one row count per line in a tree-indexed sequence and lays a line out on demand.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The layout of a whole document: how many rows each line has, which row is which line, and
//! where a position is on the screen.
//!
//! # The model
//!
//! A *row* is a line on the screen. A logical line of the document has one row, or more when it
//! wraps ([`layout_line`]). [`DocumentLayout`] keeps the **number of rows of every line** and
//! nothing else per line (4 bytes), in a sequence that finds "the first row of line n" and "the
//! line of row m" in O(log n) (`seq.rs`). The rows of a line itself (where it breaks, the columns)
//! are made when somebody asks for the line and kept in a small cache, so painting the visible
//! lines does not lay them out again and a document of 100 MB does not hold millions of rows.
//!
//! The index does not hold the [`Document`]. Every query takes the current document, as the
//! `Layout` trait of `matsu-commands` does, and the caller tells the index about every edit
//! ([`DocumentLayout::edited`]) right after it, so that the index matches the text before the next
//! command runs (ADR-0013, `CommandContext`), not at the next paint.
//!
//! # The EOF row
//!
//! Sakura draws `[EOF]` at the end of the text. When the last line ends with a line ending, and
//! in an empty document, the EOF is on a row of its own below the last line, and the caret can go
//! there. Sakura has no `CLayout` for this row (the layout manager counts `GetLineCount()` rows
//! and the view draws the EOF at row `GetLineCount()`), and neither does [`Document`]: the empty
//! text after the last line ending is not a line, and `Document::end_pos` is `(line_count, 0)`.
//! This index follows both: [`DocumentLayout::row_count`] counts the rows of the lines **plus the
//! EOF row** when there is one ([`DocumentLayout::has_eof_row`]), the EOF row belongs to no line
//! (its [`RowRef::line`] is `line_count`), and `layout_line` is never called for an empty last line
//! (which `layout_line` itself would answer with one row, see its documentation). When the last
//! line has no line ending, the EOF is drawn after its text on its last row, and there is no
//! extra row.
//!
//! # The first layout, in chunks
//!
//! [`DocumentLayout::new`] does not read the document. Lines that have not been laid out yet count
//! one row; that is right for almost every line (a line wraps only when it is wider than the wrap
//! column), so the row count and the scroll bar are close at once and exact for the lines in front
//! of the first long one. [`DocumentLayout::layout_chunk`] and [`DocumentLayout::layout_for`] lay
//! out the next lines from the top; [`DocumentLayout::progress`] says how far they are. The lines
//! that are done are exact, so the UI can show the top while the rest goes on. Every query agrees
//! with the counts, exact or not: a line that is not done shows the rows counted for it (one), so
//! painting, clicking and moving use the same row numbers. To show a part that the layout has not
//! reached (the scroll bar was dragged), call [`DocumentLayout::layout_rows`] for those rows
//! first: it makes the lines there exact, and the row numbers after them move.
//!
//! With no wrapping, a line is one row unless it is longer than 10240 columns (Sakura's
//! `MAXLINEKETAS`). Lines that cannot reach that many columns (the byte length times the widest
//! character cannot) are counted as one row without laying them out, so a no-wrap document needs
//! only a cheap pass over the line lengths, and no layout for any line but the very long ones.
//!
//! # After an edit
//!
//! [`DocumentLayout::edited`] takes the [`Edit`] that `Document::replace` returned. It lays out
//! again the lines the edit touched (plus the one before and the one after, because a line ending
//! that joins or splits can change the neighbours) and moves the lines after them without
//! looking at them: O(log n) per changed line. A change of the settings re-lays out everything
//! ([`DocumentLayout::set_config`], then the chunks again).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use matsu_core::buffer::{Document, Edit, Eol, Pos, Result};

use crate::config::LayoutConfig;
use crate::map::{offset_to_position, position_to_offset};
use crate::wrap::{Row, layout_line};

mod seq;
#[cfg(test)]
mod tests;

use seq::RowSeq;

/// How many lines the cache holds. A screen shows about a hundred.
const CACHE_LINES: usize = 512;

/// An edit that adds more lines than this is not laid out at once; the lines count one row each
/// until [`DocumentLayout::layout_chunk`] reaches them. It keeps the time of one edit short.
const EDIT_EXACT_LIMIT: usize = 4096;

/// The lines per step of [`DocumentLayout::layout_for`] between two looks at the clock.
const STEP_LINES: usize = 512;

/// One line laid out: its text, whether it ends with a line ending, and its rows.
#[derive(Clone, Debug)]
pub struct LineLayout {
    pub text: String,
    pub has_eol: bool,
    /// The kind of the line ending (`has_eol` is `eol.is_some()`).
    pub eol: Option<Eol>,
    /// At least one row ([`layout_line`]).
    pub rows: Vec<Row>,
}

/// A row of the document: a line and the index of the row in it. The EOF row below the last
/// line has `line == line_count` and `row_in_line == 0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowRef {
    pub line: usize,
    pub row_in_line: usize,
}

/// A place on the screen: a row of the document and a display column (the indent included).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPos {
    pub row: usize,
    pub column: u32,
}

/// The place in the text of a row and a column ([`DocumentLayout::row_to_pos`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowTarget {
    /// The row, after clamping to the last one.
    pub row: usize,
    pub pos: Pos,
    /// The columns that were asked for beyond the end of the row (`Located::extra_columns`).
    pub extra_columns: u32,
}

/// How far the first layout is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// The lines from the top that are laid out.
    pub done: usize,
    /// All lines.
    pub total: usize,
}

impl Progress {
    pub fn is_complete(&self) -> bool {
        self.done >= self.total
    }
}

/// What the painter needs to know about one row.
#[derive(Clone, Debug)]
pub struct RowInfo {
    /// The row number in the document.
    pub row: usize,
    /// The line, or `line_count` for the EOF row.
    pub line: usize,
    pub row_in_line: usize,
    /// The first and the end position of the text on the row (the line ending is not text). Both
    /// are `(line_count, 0)` on the EOF row.
    pub start: Pos,
    pub end: Pos,
    pub start_column: u32,
    pub end_column: u32,
    /// The row ends because the line wraps.
    pub wrapped: bool,
    /// The line ending that ends this row: set only on the last row of a line that has one. `None`
    /// on wrapped rows, on the last row of a last line without a line ending, and on the EOF row.
    pub eol: Option<Eol>,
    /// The row is the EOF row below the last line.
    pub is_eof_row: bool,
    layout: Option<Arc<LineLayout>>,
}

impl RowInfo {
    /// The text on the row, without the line ending.
    pub fn text(&self) -> &str {
        match &self.layout {
            Some(l) => &l.text[self.start.col..self.end.col],
            None => "",
        }
    }
}

#[derive(Default)]
struct Cache {
    lines: HashMap<usize, (u64, Arc<LineLayout>)>,
    tick: u64,
}

impl Cache {
    fn get(&mut self, line: usize) -> Option<Arc<LineLayout>> {
        self.tick += 1;
        let tick = self.tick;
        self.lines.get_mut(&line).map(|e| {
            e.0 = tick;
            Arc::clone(&e.1)
        })
    }

    fn insert(&mut self, line: usize, layout: Arc<LineLayout>) {
        if self.lines.len() >= CACHE_LINES {
            // Drop the older half: a scan over 512 entries, once per 256 inserts.
            let mut ticks: Vec<u64> = self.lines.values().map(|e| e.0).collect();
            ticks.sort_unstable();
            let cut = ticks[ticks.len() / 2];
            self.lines.retain(|_, e| e.0 >= cut);
        }
        self.tick += 1;
        self.lines.insert(line, (self.tick, layout));
    }

    /// Forgets lines `first..` (when the lines after an edit moved) or `first..end` (when they
    /// did not).
    fn forget(&mut self, first: usize, end: Option<usize>) {
        self.lines
            .retain(|&l, _| l < first || end.is_some_and(|e| l >= e));
    }
}

/// The layout index of a document. See the [module documentation](self).
pub struct DocumentLayout {
    cfg: LayoutConfig,
    seq: RowSeq,
    /// The lines `0..done` have their exact row count; the others count one row.
    done: usize,
    /// The EOF is on a row of its own.
    trailing: bool,
    cache: Mutex<Cache>,
}

impl std::fmt::Debug for DocumentLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentLayout")
            .field("lines", &self.seq.len())
            .field("done", &self.done)
            .field("trailing", &self.trailing)
            .finish_non_exhaustive()
    }
}

/// The rows of a line, without laying it out when it cannot have more than one. A character takes
/// at most `widest` columns, and a byte holds at most one character, so a line of `len` bytes
/// takes at most `len * widest` columns (plus one for the line ending).
fn count_rows(cfg: &LayoutConfig, text: &str, has_eol: bool, scratch: &mut Vec<Row>) -> u32 {
    let widest = u64::from(cfg.effective_tab_width().max(2).max(cfg.width.invalid_byte));
    if text.len() as u64 * widest + u64::from(has_eol) <= u64::from(cfg.max_columns()) {
        return 1;
    }
    layout_line(text, has_eol, cfg, scratch);
    scratch.len() as u32
}

fn trailing_row(doc: &Document) -> Result<bool> {
    Ok(doc.end_pos()?.line >= doc.line_count())
}

impl DocumentLayout {
    /// An index for `doc` with nothing laid out: every line counts one row. Call
    /// [`layout_chunk`](Self::layout_chunk) or [`layout_all`](Self::layout_all) to lay it out.
    pub fn new(doc: &Document, cfg: LayoutConfig) -> Result<Self> {
        Ok(Self {
            cfg,
            seq: RowSeq::ones(doc.line_count()),
            done: 0,
            trailing: trailing_row(doc)?,
            cache: Mutex::new(Cache::default()),
        })
    }

    /// [`new`](Self::new) and the whole layout.
    pub fn laid_out(doc: &Document, cfg: LayoutConfig) -> Result<Self> {
        let mut layout = Self::new(doc, cfg)?;
        layout.layout_all(doc)?;
        Ok(layout)
    }

    pub fn config(&self) -> &LayoutConfig {
        &self.cfg
    }

    fn cache(&self) -> MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    // ---- the first layout ----

    pub fn progress(&self) -> Progress {
        Progress {
            done: self.done,
            total: self.seq.len(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.progress().is_complete()
    }

    /// Throws the counts away and starts again from the document (one row per line). Used when
    /// the index finds that it does not match the document.
    pub fn resync(&mut self, doc: &Document) -> Result<()> {
        self.seq = RowSeq::ones(doc.line_count());
        self.done = 0;
        self.trailing = trailing_row(doc)?;
        self.cache().lines.clear();
        Ok(())
    }

    /// Changes the settings. When they differ, every line has to be laid out again: the counts
    /// start from one row per line and [`layout_chunk`](Self::layout_chunk) goes over the lines
    /// from the top.
    pub fn set_config(&mut self, doc: &Document, cfg: LayoutConfig) -> Result<()> {
        if cfg != self.cfg {
            self.cfg = cfg;
            self.resync(doc)?;
        }
        Ok(())
    }

    fn in_sync(&self, doc: &Document) -> bool {
        self.seq.len() == doc.line_count()
    }

    fn lay_out_lines(&self, doc: &Document, first: usize, n: usize) -> Result<Vec<u32>> {
        let mut counts = Vec::with_capacity(n);
        let mut scratch = Vec::new();
        for line in doc.lines(first..first + n) {
            let line = line?;
            counts.push(count_rows(
                &self.cfg,
                &line.text,
                line.eol.is_some(),
                &mut scratch,
            ));
        }
        counts.resize(n, 1);
        Ok(counts)
    }

    /// Lays out the next `max_lines` lines from where the layout stopped.
    pub fn layout_chunk(&mut self, doc: &Document, max_lines: usize) -> Result<Progress> {
        if !self.in_sync(doc) {
            self.resync(doc)?;
        }
        let n = max_lines.min(self.seq.len() - self.done);
        if n > 0 {
            let counts = self.lay_out_lines(doc, self.done, n)?;
            self.seq.replace(self.done, n, &counts);
            self.done += n;
        }
        Ok(self.progress())
    }

    /// Lays out the next lines for about `budget`, and returns how far it got. It checks the
    /// clock every few hundred lines, so it can pass `budget` by the time of that many lines.
    pub fn layout_for(&mut self, doc: &Document, budget: Duration) -> Result<Progress> {
        let start = Instant::now();
        loop {
            let progress = self.layout_chunk(doc, STEP_LINES)?;
            if progress.is_complete() || start.elapsed() >= budget {
                return Ok(progress);
            }
        }
    }

    /// Makes the lines on the rows `first_row..first_row + count` exact, wherever the layout of
    /// the whole document is. The row numbers of the lines before are not changed; those after
    /// them move when a line has more rows than were counted. Call it before painting a part
    /// that [`layout_chunk`](Self::layout_chunk) has not reached.
    pub fn layout_rows(&mut self, doc: &Document, first_row: usize, count: usize) -> Result<()> {
        let first_row = first_row.min(self.row_count() - 1);
        let at = self.locate_row(first_row);
        let (mut line, mut row) = (at.line, first_row - at.row_in_line);
        while row < first_row.saturating_add(count) && line < self.seq.len() {
            let mut n = self.seq.get(line).map_or(1, |c| c as usize);
            if line >= self.done {
                let Some(layout) = self.line_layout(doc, line)? else {
                    break;
                };
                if layout.rows.len() != n {
                    n = layout.rows.len();
                    self.seq.replace(line, 1, &[n as u32]);
                }
            }
            row += n;
            line += 1;
        }
        Ok(())
    }

    /// Lays out everything that is left.
    pub fn layout_all(&mut self, doc: &Document) -> Result<()> {
        self.layout_chunk(doc, usize::MAX).map(|_| ())
    }

    // ---- edits ----

    /// Updates the index after `doc.replace(..)` (or `insert`, `delete`) returned `edit`. The
    /// document must be the one that the edit made, and no other edit may have happened since the
    /// last update.
    pub fn edited(&mut self, doc: &Document, edit: &Edit) -> Result<()> {
        let old_lines = self.seq.len();
        let new_lines = doc.line_count();
        // The touched lines, one more on each side: a line ending that joins or splits can change
        // the line before it (its ending) and the one after it.
        let new_end = (edit.end.line + 2).min(new_lines);
        let first = edit.start.line.saturating_sub(1).min(new_end);
        let tail = new_lines - new_end;
        if tail + first > old_lines {
            return self.resync(doc);
        }
        self.lines_changed_inner(doc, first, old_lines - tail - first, new_end - first)
    }

    /// An edit happened but its `Edit` is not at hand: lines `first..first + old_count` (as the
    /// index knew them) were replaced by whatever lines are now at `first..`, and the lines after
    /// them did not change. Include the lines next to a changed line ending.
    pub fn lines_changed(&mut self, doc: &Document, first: usize, old_count: usize) -> Result<()> {
        let old_lines = self.seq.len();
        if first + old_count > old_lines {
            return self.resync(doc);
        }
        let tail = old_lines - first - old_count;
        let Some(new_count) = doc.line_count().checked_sub(first + tail) else {
            return self.resync(doc);
        };
        self.lines_changed_inner(doc, first, old_count, new_count)
    }

    fn lines_changed_inner(
        &mut self,
        doc: &Document,
        first: usize,
        old_count: usize,
        new_count: usize,
    ) -> Result<()> {
        let old_end = first + old_count;
        let exact = first < self.done && new_count <= EDIT_EXACT_LIMIT;
        let counts = if exact {
            self.lay_out_lines(doc, first, new_count)?
        } else {
            vec![1; new_count]
        };
        self.seq.replace(first, old_count, &counts);
        self.done = if !exact {
            self.done.min(first)
        } else if old_end <= self.done {
            self.done - old_count + new_count
        } else {
            first + new_count
        };
        if first + new_count >= self.seq.len() {
            // Only an edit that reaches the last line can change how the document ends. (The
            // end position of a document with a very long last line costs a read of the line.)
            self.trailing = trailing_row(doc)?;
        }
        if new_count == old_count {
            self.cache().forget(first, Some(first + new_count));
        } else {
            self.cache().forget(first, None);
        }
        Ok(())
    }

    // ---- counts and mapping between rows and lines ----

    /// The number of logical lines.
    pub fn line_count(&self) -> usize {
        self.seq.len()
    }

    /// The number of rows, the EOF row included ([`has_eof_row`](Self::has_eof_row)). Never 0.
    pub fn row_count(&self) -> usize {
        self.seq.total_rows() as usize + usize::from(self.trailing)
    }

    /// The EOF is on a row of its own: the document is empty or ends with a line ending.
    pub fn has_eof_row(&self) -> bool {
        self.trailing
    }

    /// The row where the EOF is drawn: the row below the last line, or the last row of the last
    /// line when that has no line ending.
    pub fn eof_row(&self) -> usize {
        self.row_count() - 1
    }

    /// The rows of line `line`, or `None` past the last line.
    pub fn rows_in_line(&self, line: usize) -> Option<usize> {
        self.seq.get(line).map(|c| c as usize)
    }

    /// The first row of `line`; `row_count()` minus the EOF row for `line == line_count()`.
    pub fn first_row_of_line(&self, line: usize) -> usize {
        self.seq.first_row(line) as usize
    }

    /// The line and the row in it of `row`; a row past the last one is the last one.
    pub fn locate_row(&self, row: usize) -> RowRef {
        let row = row.min(self.row_count() - 1);
        match self.seq.locate_row(row as u64) {
            Some((line, row_in_line)) => RowRef { line, row_in_line },
            None => RowRef {
                line: self.seq.len(),
                row_in_line: 0,
            },
        }
    }

    // ---- lines and rows ----

    /// The rows of line `line`, from the cache or laid out now. `None` past the last line.
    pub fn line_layout(&self, doc: &Document, line: usize) -> Result<Option<Arc<LineLayout>>> {
        if let Some(hit) = self.cache().get(line) {
            return Ok(Some(hit));
        }
        let Some(l) = doc.line(line)? else {
            return Ok(None);
        };
        let mut rows = Vec::new();
        let has_eol = l.eol.is_some();
        layout_line(&l.text, has_eol, &self.cfg, &mut rows);
        let layout = Arc::new(LineLayout {
            text: l.text,
            has_eol,
            eol: l.eol,
            rows,
        });
        self.cache().insert(line, Arc::clone(&layout));
        Ok(Some(layout))
    }

    fn info(&self, row: usize, line: usize, k: usize, layout: &Arc<LineLayout>) -> RowInfo {
        let r = &layout.rows[k.min(layout.rows.len() - 1)];
        RowInfo {
            row,
            line,
            row_in_line: k,
            start: Pos::new(line, r.range.start),
            end: Pos::new(line, r.range.end),
            start_column: r.start_column,
            end_column: r.end_column,
            wrapped: r.wrapped,
            eol: if k + 1 >= layout.rows.len() {
                layout.eol
            } else {
                None
            },
            is_eof_row: false,
            layout: Some(Arc::clone(layout)),
        }
    }

    fn eof_info(&self, row: usize) -> RowInfo {
        let p = Pos::new(self.seq.len(), 0);
        RowInfo {
            row,
            line: p.line,
            row_in_line: 0,
            start: p,
            end: p,
            start_column: 0,
            end_column: 0,
            wrapped: false,
            eol: None,
            is_eof_row: true,
            layout: None,
        }
    }

    /// Up to `count` rows from `first_row` (cut at the last row), for painting.
    pub fn rows(&self, doc: &Document, first_row: usize, count: usize) -> Result<Vec<RowInfo>> {
        let first_row = first_row.min(self.row_count() - 1);
        let r = self.locate_row(first_row);
        let (mut line, mut k, mut row) = (r.line, r.row_in_line, first_row);
        let mut out = Vec::with_capacity(count.min(1024));
        while out.len() < count {
            if line >= self.seq.len() {
                if self.trailing {
                    out.push(self.eof_info(row));
                }
                break;
            }
            let Some(layout) = self.line_layout(doc, line)? else {
                break;
            };
            // A line that is not laid out yet shows the rows that are counted for it.
            let shown = self.seq.get(line).map_or(1, |c| c as usize);
            while k < shown && out.len() < count {
                out.push(self.info(row, line, k, &layout));
                row += 1;
                k += 1;
            }
            line += 1;
            k = 0;
        }
        Ok(out)
    }

    /// The row `row` (clamped to the last one).
    pub fn row(&self, doc: &Document, row: usize) -> Result<RowInfo> {
        let rows = self.rows(doc, row, 1)?;
        Ok(rows.into_iter().next().unwrap_or_else(|| self.eof_info(0)))
    }

    // ---- positions ----

    /// Where `pos` (clamped into the document) is on the screen. The end of a wrapped row is the
    /// start of the next one, as in Sakura (`offset_to_position`).
    pub fn pos_to_row(&self, doc: &Document, pos: Pos) -> Result<RowPos> {
        let pos = doc.clamp_pos(pos)?;
        if pos.line >= self.seq.len() {
            return Ok(RowPos {
                row: self.seq.total_rows() as usize,
                column: 0,
            });
        }
        let Some(layout) = self.line_layout(doc, pos.line)? else {
            return Ok(RowPos {
                row: self.eof_row(),
                column: 0,
            });
        };
        let p = offset_to_position(&layout.text, &layout.rows, &self.cfg, pos.col);
        // On a line that is not laid out yet, stay on the rows that are counted for it.
        let shown = self.seq.get(pos.line).map_or(1, |c| c as usize);
        Ok(RowPos {
            row: self.first_row_of_line(pos.line) + p.row.min(shown - 1),
            column: p.column,
        })
    }

    /// The position at row `row` and display column `column`, for a click or a vertical move. A
    /// column inside a wide character or a tab is the left edge of it; a column past the end of
    /// the row is the end of the row, with the rest in [`RowTarget::extra_columns`]
    /// (`position_to_offset`).
    pub fn row_to_pos(&self, doc: &Document, row: usize, column: u32) -> Result<RowTarget> {
        let row = row.min(self.row_count() - 1);
        let r = self.locate_row(row);
        let eof = RowTarget {
            row,
            pos: Pos::new(self.seq.len(), 0),
            extra_columns: column,
        };
        if r.line >= self.seq.len() {
            return Ok(eof);
        }
        let Some(layout) = self.line_layout(doc, r.line)? else {
            return Ok(eof);
        };
        let at = position_to_offset(&layout.text, &layout.rows, &self.cfg, r.row_in_line, column);
        Ok(RowTarget {
            row,
            pos: Pos::new(r.line, at.offset),
            extra_columns: at.extra_columns,
        })
    }

    /// Moves `delta` rows from `pos` (negative: up), stopping at the first and the last row, and
    /// returns the position at display column `goal_column` there. The goal column is the one the
    /// caller remembers while the caret moves up and down (the cursor of `matsu-core` keeps it).
    pub fn move_rows(
        &self,
        doc: &Document,
        pos: Pos,
        delta: isize,
        goal_column: u32,
    ) -> Result<RowTarget> {
        let from = self.pos_to_row(doc, pos)?;
        let target = from.row.saturating_add_signed(delta).min(self.eof_row());
        self.row_to_pos(doc, target, goal_column)
    }
}
