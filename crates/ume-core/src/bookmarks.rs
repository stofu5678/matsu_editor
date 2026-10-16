//! Bookmarks: the lines of a document that the user marked (Sakura: 「ブックマーク」).
//!
//! Sakura keeps a flag in each line object (`CDocLine::m_sMark.m_cBookmarked`,
//! `sakura_core/doc/logic/CDocLine.h`, set through `CBookmarkSetter` in
//! `sakura_core/docplus/CBookmarkManager.cpp`). The flag lives and dies with the line object:
//! lines above that come and go do not matter, and a deleted line takes its flag along.
//!
//! The buffer here has no line objects (it is a persistent tree of pieces, ADR-0005), so the
//! bookmarks are a set of line numbers next to the text, and every edit tells the set which lines
//! it removed and inserted ([`LineDelta`]). [`LineDelta::of_edit`] works out which line objects
//! Sakura would keep, from `CSearchAgent::ReplaceData` (`sakura_core/agent/CSearchAgent.cpp`).
//!
//! # Undo and redo
//!
//! Sakura undoes an edit by running the inverse operation through the same `ReplaceData`, so a
//! line that undo creates has no flag, and a flag that was dropped does not come back (the
//! bookmarks are not part of the undo history). The history here stores the [`LineDelta`]s of
//! each block and undo applies the inverse ones, which gives the same result.

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
    /// The change that undoes this one.
    pub fn inverse(self) -> Self {
        Self {
            start: self.start,
            removed: self.inserted,
            inserted: self.removed,
        }
    }

    pub fn is_noop(self) -> bool {
        self.removed == 0 && self.inserted == 0
    }

    /// The changes of the line list that replacing `from..to` (in order, on valid positions, as
    /// `(line, byte column)`) with `text` makes, given the number of lines before and after. At
    /// most two: the deletion, then the insertion (Sakura's `ReplaceData` deletes first).
    ///
    /// * Deleting a range that starts at the start of a line removes that line (and the ones up
    ///   to the line where the range ends, which stays with what is left of it); a range that
    ///   starts inside a line joins that line with the line where the range ends, and the latter
    ///   goes (`nWorkLen >= nLineLen`, the whole-line and join branches).
    /// * Inserting text that ends with a line break at the start of a line puts the new lines
    ///   *before* that line, so its bookmark moves down with its text (`bInsertLineMode`);
    ///   anywhere else the new lines come after the line, and the tail of the split line is new.
    pub fn of_edit(
        from: (usize, usize),
        to: (usize, usize),
        text: &str,
        lines_before: usize,
        lines_after: usize,
    ) -> [Option<Self>; 2] {
        let deleted = to.0.saturating_sub(from.0);
        let delete = (deleted > 0).then(|| Self {
            start: if from.1 == 0 { from.0 } else { from.0 + 1 },
            removed: deleted,
            inserted: 0,
        });
        // lines_after = lines_before - deleted + inserted
        let inserted = (lines_after + deleted).saturating_sub(lines_before);
        let insert = (inserted > 0).then(|| Self {
            start: if from.1 == 0 && ends_with_line_break(text) {
                from.0
            } else {
                from.0 + 1
            },
            removed: 0,
            inserted,
        });
        [delete, insert]
    }
}

fn ends_with_line_break(text: &str) -> bool {
    text.ends_with(['\n', '\r', '\u{85}', '\u{2028}', '\u{2029}'])
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

    #[test]
    fn a_delta_and_its_inverse_cancel_when_nothing_was_lost() {
        let mut b = marks(&[0, 5]);
        let d = delta(2, 0, 2);
        b.apply(d);
        b.apply(d.inverse());
        assert_eq!(lines(&b), [0, 5]);
    }

    #[test]
    fn of_edit_follows_sakura() {
        // Enter inside a line: the new line comes after, the tail has no bookmark
        let d = LineDelta::of_edit((2, 3), (2, 3), "\n", 5, 6);
        assert_eq!(d, [None, Some(delta(3, 0, 1))]);
        // Enter at the start of a line: the line (and its bookmark) moves down
        let d = LineDelta::of_edit((2, 0), (2, 0), "\n", 5, 6);
        assert_eq!(d, [None, Some(delta(2, 0, 1))]);
        // text that does not end with a line break at the start of a line: the line stays
        let d = LineDelta::of_edit((2, 0), (2, 0), "a\nb", 5, 6);
        assert_eq!(d, [None, Some(delta(3, 0, 1))]);
        // deleting a whole line: that line goes
        let d = LineDelta::of_edit((2, 0), (3, 0), "", 5, 4);
        assert_eq!(d, [Some(delta(2, 1, 0)), None]);
        // joining two lines: the second one goes
        let d = LineDelta::of_edit((2, 4), (3, 0), "", 5, 4);
        assert_eq!(d, [Some(delta(3, 1, 0)), None]);
        // typing in one line changes nothing
        assert_eq!(LineDelta::of_edit((2, 1), (2, 1), "x", 5, 5), [None, None]);
        // replacing two lines with one line of text: a deletion, no insertion
        let d = LineDelta::of_edit((2, 1), (3, 1), "z", 5, 4);
        assert_eq!(d, [Some(delta(3, 1, 0)), None]);
        // replacing one line's end and the next with two lines of text: delete, then insert
        let d = LineDelta::of_edit((2, 1), (3, 1), "z\ny\nw", 5, 6);
        assert_eq!(d, [Some(delta(3, 1, 0)), Some(delta(3, 0, 2))]);
    }

    #[test]
    fn truncate_drops_missing_lines() {
        let mut b = marks(&[0, 3, 9]);
        b.truncate(4);
        assert_eq!(lines(&b), [0, 3]);
    }
}
