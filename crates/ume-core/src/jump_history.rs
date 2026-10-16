//! The jump history of a view: the places the caret jumped from, which "history: previous" and
//! "history: next" walk through.
//!
//! Sakura: `CAutoMarkMgr` (`sakura_core/env/CMarkMgr.cpp`, zlib License), held by every view as
//! `CEditView::m_cHistory` with a limit of 30 entries (`CEditView.cpp`). Sakura's marks have an
//! `IsValid` that is always true, so this keeps only the positions. Like Sakura, the history does
//! not follow edits: a position can be past the end of the text by the time it is used, and the
//! caller clamps it.

use crate::buffer::Pos;

/// The most positions the history keeps (`m_cHistory->SetMax( 30 )` in `CEditView::CEditView`).
pub const MAX_ENTRIES: usize = 30;

/// A list of positions, oldest first, and the current place in it.
///
/// The current place is a number between 0 and the length: the position that
/// [`current`](Self::current) returns is the entry there, and the place after the last entry
/// (where new entries are added) has no entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JumpHistory {
    marks: Vec<Pos>,
    current: usize,
}

impl JumpHistory {
    /// Adds `pos` at the current place and forgets everything after it (`CAutoMarkMgr::Add`).
    /// The oldest entries go when there are more than [`MAX_ENTRIES`].
    pub fn add(&mut self, pos: Pos) {
        self.marks.truncate(self.current);
        self.marks.push(pos);
        self.current += 1;
        if self.marks.len() > MAX_ENTRIES {
            let extra = self.marks.len() - MAX_ENTRIES;
            self.marks.drain(..extra);
            self.current = self.current.saturating_sub(extra);
        }
    }

    /// Whether there is an entry before the current place (`CheckPrev`).
    pub fn has_prev(&self) -> bool {
        self.current > 0
    }

    /// Whether there is an entry after the current place (`CheckNext`).
    pub fn has_next(&self) -> bool {
        self.current + 1 < self.marks.len()
    }

    /// Moves the current place one entry back. False (and no move) at the first entry
    /// (`PrevValid`).
    pub fn go_prev(&mut self) -> bool {
        if self.current == 0 {
            return false;
        }
        self.current -= 1;
        true
    }

    /// Moves the current place one entry forward. False (and no move) at the last entry
    /// (`NextValid`).
    pub fn go_next(&mut self) -> bool {
        if !self.has_next() {
            return false;
        }
        self.current += 1;
        true
    }

    /// The entry at the current place, if there is one (`GetCurrent`).
    pub fn current(&self) -> Option<Pos> {
        self.marks.get(self.current).copied()
    }

    /// Forgets everything (`Flush`; Sakura does it when a file is loaded).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// How many entries there are.
    pub fn len(&self) -> usize {
        self.marks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(line: usize) -> Pos {
        Pos::new(line, 0)
    }

    #[test]
    fn add_puts_the_place_after_the_new_entry() {
        let mut h = JumpHistory::default();
        h.add(p(1));
        h.add(p(2));
        assert_eq!(h.len(), 2);
        assert_eq!(h.current(), None);
        assert!(h.has_prev());
        assert!(!h.has_next());
    }

    #[test]
    fn walking_back_and_forth() {
        let mut h = JumpHistory::default();
        for i in 1..=3 {
            h.add(p(i));
        }
        assert!(h.go_prev());
        assert_eq!(h.current(), Some(p(3)));
        assert!(h.go_prev());
        assert_eq!(h.current(), Some(p(2)));
        assert!(h.has_next());
        assert!(h.go_next());
        assert_eq!(h.current(), Some(p(3)));
        assert!(!h.has_next());
        assert!(!h.go_next());
    }

    #[test]
    fn add_in_the_middle_drops_the_newer_entries() {
        let mut h = JumpHistory::default();
        for i in 1..=3 {
            h.add(p(i));
        }
        h.go_prev();
        h.go_prev(); // at p(2)
        h.add(p(9));
        assert_eq!(h.len(), 2);
        assert_eq!(h.marks, vec![p(1), p(9)]);
        assert!(!h.has_next());
    }

    #[test]
    fn the_oldest_entries_go_over_the_limit() {
        let mut h = JumpHistory::default();
        for i in 0..MAX_ENTRIES + 5 {
            h.add(p(i));
        }
        assert_eq!(h.len(), MAX_ENTRIES);
        h.go_prev();
        assert_eq!(h.current(), Some(p(MAX_ENTRIES + 4)));
        for _ in 0..MAX_ENTRIES {
            h.go_prev();
        }
        assert_eq!(h.current(), Some(p(5)));
        assert!(!h.has_prev());
    }

    #[test]
    fn clear_forgets_everything() {
        let mut h = JumpHistory::default();
        h.add(p(1));
        h.clear();
        assert!(h.is_empty());
        assert!(!h.has_prev());
    }
}
