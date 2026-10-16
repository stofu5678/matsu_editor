//! Bookmarks: the lines of a document that the user marked (Sakura: 「ブックマーク」).
//!
//! Sakura keeps a flag in each line object (`CDocLine::m_sMark.m_cBookmarked`,
//! `sakura_core/doc/logic/CDocLine.h`, set through `CBookmarkSetter` in
//! `sakura_core/docplus/CBookmarkManager.cpp`). The flag lives and dies with the line object:
//! lines above that come and go do not matter, and a deleted line takes its flag along.
//!
//! The buffer here has no line objects (it is a persistent tree of pieces, ADR-0005), so the
//! bookmarks are a set of line numbers next to the text, and every edit tells the set which lines
//! it removed and inserted ([`LineDelta`]). [`LineEdit`] works out which line objects Sakura
//! would keep, from `CSearchAgent::ReplaceData` (`sakura_core/agent/CSearchAgent.cpp`).
//!
//! # Undo and redo
//!
//! Sakura undoes an edit by running the inverse operation through the same `ReplaceData`, so a
//! line that undo creates has no flag, and a flag that was dropped does not come back (the
//! bookmarks are not part of the undo history). The history here stores the [`LineEdit`] of each
//! edit of a block, which has the deltas of the edit and of its inverse.

use std::collections::BTreeSet;

/// A change of the line list: the lines `start..start + removed` are gone and `inserted` new
/// lines (without bookmarks) stand in their place; the lines after them move accordingly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineDelta {
    pub start: usize,
    pub removed: usize,
    pub inserted: usize,
}

impl LineDelta {
    pub fn is_noop(self) -> bool {
        self.removed == 0 && self.inserted == 0
    }
}

/// What [`LineEdit::of_replace`] needs to know about the text that goes in or comes out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextBreaks {
    /// The number of line breaks in it (a CRLF is one).
    pub count: usize,
    /// Whether it ends with a line break.
    pub ends_with_break: bool,
}

impl TextBreaks {
    /// Counts the line breaks of `text`.
    pub fn of(text: &str) -> Self {
        let mut count = 0;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\r' => {
                    count += 1;
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                }
                '\n' | '\u{85}' | '\u{2028}' | '\u{2029}' => count += 1,
                _ => {}
            }
        }
        Self {
            count,
            ends_with_break: text.ends_with(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}']),
        }
    }
}

/// The changes of the line list that one edit makes, and the ones that undoing it makes.
///
/// Each is at most two [`LineDelta`]s: the deletion, then the insertion (Sakura's `ReplaceData`
/// deletes first, then inserts).
///
/// * Deleting a range that starts at the start of a line removes that line (and the ones up
///   to the line where the range ends, which stays with what is left of it); a range that
///   starts inside a line joins that line with the line where the range ends, and the latter
///   goes (`nWorkLen >= nLineLen`, the whole-line and join branches).
/// * Inserting text that ends with a line break at the start of a line puts the new lines
///   *before* that line, so its bookmark moves down with its text (`bInsertLineMode`);
///   anywhere else the new lines come after the line, and the tail of the split line is new.
/// * Undo runs the inverse edit through the same rules (Sakura does the same), so it can be a
///   different change of the line list than the mirror image of the edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEdit {
    forward: [Option<LineDelta>; 2],
    inverse: [Option<LineDelta>; 2],
}

impl LineEdit {
    /// The line changes of replacing `from..to` (as `(line, byte column)`, in order) with a text
    /// of the shape `inserted`, which ends at `new_end`. `removed` says the same of the replaced
    /// text. `rows_before` and `rows_after` are the number of rows of the whole text (its line
    /// breaks plus one, so the empty row after a final line break counts) before and after.
    pub fn of_replace(
        from: (usize, usize),
        to: (usize, usize),
        new_end: (usize, usize),
        removed: TextBreaks,
        inserted: TextBreaks,
        rows_before: usize,
        rows_after: usize,
    ) -> Self {
        Self {
            forward: Self::deltas(from, to, inserted, rows_before, rows_after),
            inverse: Self::deltas(from, new_end, removed, rows_after, rows_before),
        }
    }

    /// The deltas of replacing `from..to` with text of the shape `ins`.
    fn deltas(
        from: (usize, usize),
        to: (usize, usize),
        ins: TextBreaks,
        rows_before: usize,
        rows_after: usize,
    ) -> [Option<LineDelta>; 2] {
        let deleted = to.0.saturating_sub(from.0);
        // A CR at the end of one line and an LF at the start of the next make one line break
        // when they meet: lines that the numbers of breaks do not explain are gone.
        // (When the inserted text has breaks itself, the merge happens at its edge and only
        // means that it made fewer new lines.)
        let merged = if ins.count == 0 {
            rows_before.saturating_sub(deleted + rows_after)
        } else {
            0
        };
        let removed = deleted + merged;
        let delete = (removed > 0).then(|| LineDelta {
            start: if from.1 == 0 { from.0 } else { from.0 + 1 },
            removed,
            inserted: 0,
        });
        let inserted = (rows_after + removed).saturating_sub(rows_before);
        let insert = (inserted > 0).then(|| LineDelta {
            start: if from.1 == 0 && ins.ends_with_break {
                from.0
            } else {
                from.0 + 1
            },
            removed: 0,
            inserted,
        });
        [delete, insert]
    }

    /// The deltas to apply for the edit, in order.
    pub fn forward(&self) -> impl Iterator<Item = LineDelta> + '_ {
        self.forward.iter().flatten().copied()
    }

    /// The deltas to apply for undoing the edit, in order.
    pub fn inverse(&self) -> impl Iterator<Item = LineDelta> + '_ {
        self.inverse.iter().flatten().copied()
    }
}

/// The bookmarked lines of a document (0-based line numbers).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bookmarks {
    lines: BTreeSet<usize>,
}

impl Bookmarks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn contains(&self, line: usize) -> bool {
        self.lines.contains(&line)
    }

    /// The bookmarked lines, smallest first.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.lines.iter().copied()
    }

    /// Sets or clears the bookmark of `line` (Sakura: `CBookmarkSetter::SetBookmark`).
    pub fn set(&mut self, line: usize, on: bool) {
        if on {
            self.lines.insert(line);
        } else {
            self.lines.remove(&line);
        }
    }

    /// Turns the bookmark of `line` over (`SetBookmark(!IsBookmarked())`).
    pub fn toggle(&mut self, line: usize) {
        let on = !self.contains(line);
        self.set(line, on);
    }

    /// Removes every bookmark (`CBookmarkManager::ResetAllBookMark`).
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// The first bookmarked line.
    pub fn first(&self) -> Option<usize> {
        self.lines.first().copied()
    }

    /// The last bookmarked line.
    pub fn last(&self) -> Option<usize> {
        self.lines.last().copied()
    }

    /// The first bookmarked line after `line` (`CBookmarkManager::SearchBookMark`, forward).
    pub fn next_after(&self, line: usize) -> Option<usize> {
        self.lines.range(line.saturating_add(1)..).next().copied()
    }

    /// The last bookmarked line before `line` (`SearchBookMark`, backward).
    pub fn prev_before(&self, line: usize) -> Option<usize> {
        self.lines.range(..line).next_back().copied()
    }

    /// Follows a change of the line list.
    pub fn apply(&mut self, delta: LineDelta) {
        if delta.is_noop() {
            return;
        }
        let tail = self.lines.split_off(&delta.start);
        let end = delta.start + delta.removed;
        for line in tail {
            if line >= end {
                self.lines.insert(line - delta.removed + delta.inserted);
            }
        }
    }

    /// Drops the bookmarks of lines that do not exist (a safety net after a whole-text change).
    pub fn truncate(&mut self, line_count: usize) {
        let _ = self.lines.split_off(&line_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(lines: &[usize]) -> Bookmarks {
        Bookmarks {
            lines: lines.iter().copied().collect(),
        }
    }

    fn lines(b: &Bookmarks) -> Vec<usize> {
        b.iter().collect()
    }

    fn delta(start: usize, removed: usize, inserted: usize) -> LineDelta {
        LineDelta {
            start,
            removed,
            inserted,
        }
    }

    #[test]
    fn toggles_and_searches() {
        let mut b = Bookmarks::new();
        b.toggle(3);
        b.toggle(7);
        assert_eq!(lines(&b), [3, 7]);
        assert_eq!(b.next_after(3), Some(7));
        assert_eq!(b.next_after(7), None);
        assert_eq!(b.prev_before(7), Some(3));
        assert_eq!(b.prev_before(3), None);
        assert_eq!(b.next_after(usize::MAX), None);
        assert_eq!((b.first(), b.last()), (Some(3), Some(7)));
        b.toggle(3);
        assert_eq!(lines(&b), [7]);
        b.clear();
        assert!(b.is_empty());
    }

    #[test]
    fn lines_follow_inserted_and_removed_lines() {
        let mut b = marks(&[1, 4, 6]);
        b.apply(delta(2, 0, 3));
        assert_eq!(lines(&b), [1, 7, 9]);
        b.apply(delta(6, 2, 0));
        // the bookmark of line 7 went with its line
        assert_eq!(lines(&b), [1, 7]);
        b.apply(delta(1, 1, 1));
        assert_eq!(lines(&b), [7]);
    }

    /// The forward deltas of replacing `from..to` by `text` in a text of `before` rows that has
    /// `after` rows afterwards.
    fn forward(
        from: (usize, usize),
        to: (usize, usize),
        text: &str,
        before: usize,
        after: usize,
    ) -> [Option<LineDelta>; 2] {
        LineEdit::deltas(from, to, TextBreaks::of(text), before, after)
    }

    #[test]
    fn deltas_follow_sakura() {
        // Enter inside a line: the new line comes after, the tail has no bookmark
        let d = forward((2, 3), (2, 3), "\n", 5, 6);
        assert_eq!(d, [None, Some(delta(3, 0, 1))]);
        // Enter at the start of a line: the line (and its bookmark) moves down
        let d = forward((2, 0), (2, 0), "\n", 5, 6);
        assert_eq!(d, [None, Some(delta(2, 0, 1))]);
        // text that does not end with a line break at the start of a line: the line stays
        let d = forward((2, 0), (2, 0), "a\nb", 5, 6);
        assert_eq!(d, [None, Some(delta(3, 0, 1))]);
        // deleting a whole line: that line goes
        let d = forward((2, 0), (3, 0), "", 5, 4);
        assert_eq!(d, [Some(delta(2, 1, 0)), None]);
        // joining two lines: the second one goes
        let d = forward((2, 4), (3, 0), "", 5, 4);
        assert_eq!(d, [Some(delta(3, 1, 0)), None]);
        // typing in one line changes nothing
        assert_eq!(forward((2, 1), (2, 1), "x", 5, 5), [None, None]);
        // replacing two lines with one line of text: a deletion, no insertion
        let d = forward((2, 1), (3, 1), "z", 5, 4);
        assert_eq!(d, [Some(delta(3, 1, 0)), None]);
        // replacing one line's end and the next with two lines of text: delete, then insert
        let d = forward((2, 1), (3, 1), "z\ny\nw", 5, 6);
        assert_eq!(d, [Some(delta(3, 1, 0)), Some(delta(3, 0, 2))]);
    }

    #[test]
    fn a_cr_that_meets_an_lf_loses_a_line() {
        // "a\rb\nc": deleting "b" (line 1) makes "a\r\nc": the rows go from 3 to 2
        let d = forward((1, 0), (1, 1), "", 3, 2);
        assert_eq!(d, [Some(delta(1, 1, 0)), None]);
    }

    #[test]
    fn an_inserted_cr_that_meets_an_lf_makes_no_new_line() {
        // "a\nb": inserting "\r" after the "a" makes "a\r\nb": two rows before and after
        let d = forward((0, 1), (0, 1), "\r", 2, 2);
        assert_eq!(d, [None, None]);
    }

    #[test]
    fn undo_runs_the_inverse_edit_through_the_same_rules() {
        // insert "\nx" at the start of "abc": the line stays above, the rest is new below
        let edit = LineEdit::of_replace(
            (0, 0),
            (0, 0),
            (1, 1),
            TextBreaks::of(""),
            TextBreaks::of("\nx"),
            1,
            2,
        );
        assert_eq!(edit.forward().collect::<Vec<_>>(), [delta(1, 0, 1)]);
        // undo deletes (0,0)..(1,1): the first line goes, the bookmark of "xabc" survives
        assert_eq!(edit.inverse().collect::<Vec<_>>(), [delta(0, 1, 0)]);
        let mut b = marks(&[1]);
        for d in edit.inverse() {
            b.apply(d);
        }
        assert_eq!(lines(&b), [0]);
    }

    #[test]
    fn counts_the_breaks_of_a_text() {
        assert_eq!(TextBreaks::of("").count, 0);
        assert_eq!(TextBreaks::of("a\r\nb\rc\nd\u{2028}").count, 4);
        assert!(TextBreaks::of("a\r\n").ends_with_break);
        assert!(!TextBreaks::of("a").ends_with_break);
    }

    #[test]
    fn truncate_drops_missing_lines() {
        let mut b = marks(&[0, 3, 9]);
        b.truncate(4);
        assert_eq!(lines(&b), [0, 3]);
    }
}
