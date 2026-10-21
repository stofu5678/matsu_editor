//! Modified lines: which lines of a document changed since it was saved (Sakura's 変更行, drawn in
//! the line number area and reached with `F_MODIFYLINE_NEXT` / `F_MODIFYLINE_PREV`).
//!
//! Sakura keeps a sequence number in each line object (`CDocLine::m_sMark.m_cModified`,
//! `sakura_core/docplus/CModifyManager.cpp`). An edit gives the lines it touched the number of
//! the undo block it belongs to (`COpeBuf::GetNextSeq`, the current block pointer plus one;
//! `CSearchAgent::ReplaceData`, `sakura_core/agent/CSearchAgent.cpp`). A line is modified when its
//! number is not the number the document had when it was saved (`COpeBuf::GetNoModifiedSeq`,
//! `CModifyVisitor::IsLineModified`). Saving sets every line to the current pointer
//! (`CModifyManager::OnAfterSave`, `ResetAllModifyFlag`). Undo puts back the numbers the lines had
//! before the edit (`nOrgSeq`, `nDelSeq`, `nInsSeq` in `sakura_core/cmd/COpe.h`), so undoing back
//! to the saved version leaves no line modified, and undoing past the saved version marks only the
//! lines the undone edits touched.
//!
//! The buffer here has no line objects (ADR-0005), so the numbers are a run-length list next to
//! the text ([`ModifiedLines`]): lines that nothing touched since the last save share the `base`
//! number. Each edit replaces the numbers of the lines it covers ([`MarkEdit`], worked out by
//! [`ModifiedLines::edit_of_replace`]); the history keeps the [`MarkEdit`]s of a block to undo and
//! redo them, like it keeps the [`LineEdit`](crate::bookmarks::LineEdit)s for the bookmarks.

use crate::bookmarks::TextBreaks;

/// A sequence number: the position in the undo history (counted over the whole life of the
/// history, also when the oldest blocks are forgotten).
pub type Seq = u64;

/// Run-length coded numbers of consecutive lines: `(count, number)`.
pub type Segments = Vec<(usize, Seq)>;

fn push_segment(segments: &mut Segments, len: usize, seq: Seq) {
    if len == 0 {
        return;
    }
    match segments.last_mut() {
        Some((n, s)) if *s == seq => *n += len,
        _ => segments.push((len, seq)),
    }
}

/// The number of lines in `segments`.
fn total(segments: &[(usize, Seq)]) -> usize {
    segments.iter().map(|&(n, _)| n).sum()
}

/// A run of lines `start..end` that have the number `seq`, which is not `base`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Run {
    start: usize,
    end: usize,
    seq: Seq,
}

/// The sequence numbers of the lines of a document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModifiedLines {
    /// The number of the lines that are in no run.
    base: Seq,
    /// Sorted, without overlap, never adjacent with the same number, never with the base number.
    runs: Vec<Run>,
}

/// The shape of a replacement of text, for [`ModifiedLines::edit_of_replace`].
#[derive(Clone, Copy, Debug)]
pub struct Replaced {
    /// The start of the replaced range, as `(line, byte column)`.
    pub from: (usize, usize),
    /// The end of the replaced range.
    pub to: (usize, usize),
    /// The line breaks of the new text.
    pub inserted: TextBreaks,
    /// Whether the new text is empty.
    pub inserted_empty: bool,
    /// The number of rows of the whole text (its line breaks plus one) before the edit.
    pub rows_before: usize,
    /// And after it.
    pub rows_after: usize,
}

/// What one edit did to the numbers: the lines `start..start + len(old)` had the numbers `old`,
/// and `new` replaces them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkEdit {
    start: usize,
    old: Segments,
    new: Segments,
}

impl MarkEdit {
    /// Does the edit.
    pub fn apply(&self, lines: &mut ModifiedLines) {
        lines.splice(self.start, total(&self.old), &self.new);
    }

    /// Undoes the edit: the lines get their old numbers back.
    pub fn revert(&self, lines: &mut ModifiedLines) {
        lines.splice(self.start, total(&self.new), &self.old);
    }
}

impl ModifiedLines {
    /// Every line has the number 0, the one of a document that was just loaded.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of `line`.
    pub fn seq_of(&self, line: usize) -> Seq {
        let i = self.runs.partition_point(|r| r.end <= line);
        match self.runs.get(i) {
            Some(r) if r.start <= line => r.seq,
            _ => self.base,
        }
    }

    /// The numbers of the lines `start..start + len`.
    pub fn segments(&self, start: usize, len: usize) -> Segments {
        let end = start + len;
        let mut out = Segments::new();
        let mut at = start;
        let first = self.runs.partition_point(|r| r.end <= start);
        for r in &self.runs[first..] {
            if r.start >= end {
                break;
            }
            let (from, to) = (r.start.max(start), r.end.min(end));
            push_segment(&mut out, from - at, self.base);
            push_segment(&mut out, to - from, r.seq);
            at = to;
        }
        push_segment(&mut out, end - at, self.base);
        out
    }

    /// Replaces the numbers of the lines `start..start + old_len` with `new`; the lines after
    /// them move by the difference of the lengths.
    pub fn splice(&mut self, start: usize, old_len: usize, new: &[(usize, Seq)]) {
        let end = start + old_len;
        let new_len = total(new);
        let mut out: Vec<Run> = Vec::with_capacity(self.runs.len() + new.len() + 1);
        let mut add = |run: Run, base: Seq| {
            if run.start >= run.end || run.seq == base {
                return;
            }
            match out.last_mut() {
                Some(last) if last.end == run.start && last.seq == run.seq => last.end = run.end,
                _ => out.push(run),
            }
        };
        let base = self.base;
        let mut inserted = false;
        let insert_new = |add: &mut dyn FnMut(Run, Seq)| {
            let mut at = start;
            for &(n, seq) in new {
                add(
                    Run {
                        start: at,
                        end: at + n,
                        seq,
                    },
                    base,
                );
                at += n;
            }
        };
        for r in &self.runs {
            if r.end <= start {
                add(*r, base);
                continue;
            }
            if r.start < start {
                add(Run { end: start, ..*r }, base);
            }
            if !inserted && r.end > start {
                insert_new(&mut add);
                inserted = true;
            }
            if r.end > end {
                add(
                    Run {
                        start: r.start.max(end) - old_len + new_len,
                        end: r.end - old_len + new_len,
                        seq: r.seq,
                    },
                    base,
                );
            }
        }
        if !inserted {
            insert_new(&mut add);
        }
        self.runs = out;
    }

    /// Every line gets the number `seq`: the document was saved at that point of the history.
    pub fn reset(&mut self, seq: Seq) {
        self.base = seq;
        self.runs.clear();
    }

    /// Forgets the numbers of lines from `rows` on (a safety net: they do not exist).
    pub fn truncate(&mut self, rows: usize) {
        self.runs.retain(|r| r.start < rows);
        if let Some(last) = self.runs.last_mut() {
            last.end = last.end.min(rows);
        }
    }

    /// The numbers an edit gives: replacing `from..to` (as `(line, byte column)`, in order) with
    /// a text of the shape given, in the block that has the number `seq`.
    ///
    /// Following `CSearchAgent::ReplaceData`: the lines that the new text or the cut touches get
    /// `seq`. A line that only moved keeps its number: a line that is deleted whole takes nobody
    /// along, and when the new text ends with a line break at the start of a line, the new lines
    /// go in *front of* that line (`bInsertLineMode`), so the line keeps its number.
    pub fn edit_of_replace(&self, r: Replaced, seq: Seq) -> MarkEdit {
        let Replaced {
            from,
            to,
            inserted,
            inserted_empty: is_empty,
            rows_before,
            rows_after,
        } = r;
        let old_len = to.0 - from.0 + 1;
        let old = self.segments(from.0, old_len);
        let new_len = rows_after.saturating_sub(rows_before.saturating_sub(old_len));
        // The last line of the result is the end of the cut line, if nothing was cut from the
        // start of it and nothing was added in front of it.
        let tail_alone = to.1 == 0
            && if inserted.count == 0 {
                from.1 == 0 && is_empty
            } else {
                inserted.ends_with_break
            };
        let mut new = Segments::new();
        if tail_alone && new_len > 0 {
            push_segment(&mut new, new_len - 1, seq);
            let tail = old.last().map_or(self.base, |&(_, s)| s);
            push_segment(&mut new, 1, tail);
        } else {
            push_segment(&mut new, new_len, seq);
        }
        MarkEdit {
            start: from.0,
            old,
            new,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(m: &ModifiedLines, n: usize) -> Vec<Seq> {
        (0..n).map(|i| m.seq_of(i)).collect()
    }

    fn set(m: &mut ModifiedLines, start: usize, len: usize, seq: Seq) {
        let old = m.segments(start, len);
        MarkEdit {
            start,
            old,
            new: vec![(len, seq)],
        }
        .apply(m);
    }

    #[test]
    fn numbers_default_to_the_base() {
        let mut m = ModifiedLines::new();
        assert_eq!(lines(&m, 3), [0, 0, 0]);
        set(&mut m, 1, 2, 5);
        assert_eq!(lines(&m, 5), [0, 5, 5, 0, 0]);
        m.reset(7);
        assert_eq!(lines(&m, 3), [7, 7, 7]);
    }

    #[test]
    fn splice_moves_the_lines_after_it() {
        let mut m = ModifiedLines::new();
        set(&mut m, 1, 1, 1);
        set(&mut m, 4, 1, 2);
        // two lines replaced by three: line 4 becomes line 5
        m.splice(2, 2, &[(3, 9)]);
        assert_eq!(lines(&m, 7), [0, 1, 9, 9, 9, 2, 0]);
        // three replaced by none: back
        m.splice(2, 3, &[]);
        assert_eq!(lines(&m, 5), [0, 1, 2, 0, 0]);
    }

    #[test]
    fn splice_inside_a_run_cuts_it() {
        let mut m = ModifiedLines::new();
        set(&mut m, 0, 6, 3);
        m.splice(2, 2, &[(1, 8)]);
        assert_eq!(lines(&m, 6), [3, 3, 8, 3, 3, 0]);
        // an insertion in the middle of a run
        m.splice(1, 0, &[(2, 4)]);
        assert_eq!(lines(&m, 8), [3, 4, 4, 3, 8, 3, 3, 0]);
    }

    #[test]
    fn equal_neighbors_are_one_run_and_base_runs_vanish() {
        let mut m = ModifiedLines::new();
        set(&mut m, 0, 2, 3);
        set(&mut m, 2, 2, 3);
        assert_eq!(m.runs.len(), 1);
        set(&mut m, 0, 4, 0);
        assert!(m.runs.is_empty());
    }

    #[test]
    fn segments_cover_gaps_with_the_base() {
        let mut m = ModifiedLines::new();
        set(&mut m, 2, 2, 5);
        assert_eq!(m.segments(0, 6), [(2, 0), (2, 5), (2, 0)]);
        assert_eq!(m.segments(3, 1), [(1, 5)]);
        assert_eq!(m.segments(3, 0), []);
    }

    #[test]
    fn mark_edits_undo_exactly() {
        let mut m = ModifiedLines::new();
        set(&mut m, 1, 2, 1);
        let before = m.clone();
        let e = m.edit_of_replace(
            Replaced {
                from: (0, 1),
                to: (2, 1),
                inserted: TextBreaks::of("", true),
                inserted_empty: true,
                rows_before: 4,
                rows_after: 2,
            },
            2,
        );
        e.apply(&mut m);
        e.revert(&mut m);
        assert_eq!(m, before);
    }

    /// The numbers of a 4-line text after replacing `from..to` with `text`.
    fn replaced(
        from: (usize, usize),
        to: (usize, usize),
        text: &str,
        rows_before: usize,
        rows_after: usize,
    ) -> Vec<Seq> {
        let mut m = ModifiedLines::new();
        // lines 0..rows_before have the numbers 10, 11, ...
        for i in 0..rows_before {
            set(&mut m, i, 1, 10 + i as Seq);
        }
        let e = m.edit_of_replace(
            Replaced {
                from,
                to,
                inserted: TextBreaks::of(text, true),
                inserted_empty: text.is_empty(),
                rows_before,
                rows_after,
            },
            99,
        );
        e.apply(&mut m);
        lines(&m, rows_after)
    }

    #[test]
    fn typing_touches_the_line() {
        assert_eq!(replaced((1, 2), (1, 2), "x", 4, 4), [10, 99, 12, 13]);
    }

    #[test]
    fn enter_in_a_line_touches_both_halves() {
        assert_eq!(replaced((1, 2), (1, 2), "\n", 4, 5), [10, 99, 99, 12, 13]);
    }

    #[test]
    fn enter_at_the_start_of_a_line_keeps_that_line() {
        // the new line goes in front; the old line (and its number) moves down
        assert_eq!(replaced((1, 0), (1, 0), "\n", 4, 5), [10, 99, 11, 12, 13]);
    }

    #[test]
    fn deleting_a_whole_line_touches_nothing() {
        assert_eq!(replaced((1, 0), (2, 0), "", 4, 3), [10, 12, 13]);
    }

    #[test]
    fn joining_lines_touches_the_joined_one() {
        assert_eq!(replaced((1, 3), (2, 0), "", 4, 3), [10, 99, 13]);
    }

    #[test]
    fn cutting_the_start_of_a_line_touches_it() {
        assert_eq!(replaced((1, 0), (1, 2), "", 4, 4), [10, 99, 12, 13]);
        assert_eq!(replaced((1, 0), (2, 2), "", 4, 3), [10, 99, 13]);
    }

    #[test]
    fn pasting_lines_in_the_middle() {
        // "x\ny" in the middle of line 1: three lines, the last one is the tail of the old line
        assert_eq!(replaced((1, 2), (1, 2), "x\ny", 4, 5), [10, 99, 99, 12, 13]);
        // a text that ends with a line break in the middle of a line: the tail is a new line
        assert_eq!(replaced((1, 2), (1, 2), "x\n", 4, 5), [10, 99, 99, 12, 13]);
    }

    #[test]
    fn replacing_up_to_the_start_of_a_line_keeps_that_line() {
        // (1,2)..(2,0) replaced by "x\n": the line 2 is not touched
        assert_eq!(replaced((1, 2), (2, 0), "x\n", 4, 4), [10, 99, 12, 13]);
    }

    #[test]
    fn truncate_drops_missing_lines() {
        let mut m = ModifiedLines::new();
        set(&mut m, 0, 10, 4);
        m.truncate(3);
        assert_eq!(m.segments(0, 5), [(3, 4), (2, 0)]);
    }
}
