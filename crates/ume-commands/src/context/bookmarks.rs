//! Bookmarks (Sakura's ブックマーク): toggling the lines of the selection, clearing them all, and
//! the jumps to the next and the previous one. The set of lines is in the document
//! ([`ume_core::bookmarks`]).
//!
//! Sakura: `Command_BOOKMARK_SET`, `Command_BOOKMARK_NEXT`, `Command_BOOKMARK_PREV` and
//! `Command_BOOKMARK_RESET` in `sakura_core/cmd/CViewCommander_Bookmark.cpp`, and
//! `CBookmarkManager` in `sakura_core/docplus/CBookmarkManager.cpp`.

use ume_core::buffer::Pos;
use ume_core::search::Direction;

use super::CommandContext;
use crate::command::Result;

/// How a jump to a bookmark ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BookmarkJump {
    /// The caret is on the bookmarked line now. `wrapped`: the search went on from the other end
    /// of the file (Sakura says so in the status bar: `STR_ERR_SRNEXT1`, `STR_ERR_SRPREV1`).
    Found { wrapped: bool },
    /// There is no bookmark in that direction.
    NotFound,
}

impl CommandContext<'_> {
    /// The lines that `F_BOOKMARK_SET` works on: with a selection that reaches over more than
    /// one row, the logical lines from the one of the start row to the one of the end row,
    /// both included (a selection that ends at the start of a line includes that line); else the
    /// line of the caret. For a box selection the rows are the lines of its corners.
    fn bookmark_target_lines(&self) -> Result<(usize, usize)> {
        let caret = self.caret();
        let doc = self.doc();
        let span = if self.cursor.is_box() {
            self.cursor
                .anchor()
                .filter(|_| self.cursor.has_selection())
                .map(|a| (a.min(caret), a.max(caret)))
        } else {
            self.selection()
        };
        if let Some((a, b)) = span {
            // `GetFrom().y < GetTo().y` in layout rows (a wrapped line has more than one)
            let rows_differ = if self.cursor.is_box() {
                a.line != b.line
            } else {
                self.layout.row_start(doc, a)? != self.layout.row_start(doc, b)?
            };
            if rows_differ {
                return Ok((a.line, b.line));
            }
        }
        Ok((caret.line, caret.line))
    }

    /// `F_BOOKMARK_SET`: sets the bookmark of every target line that has none, and clears it on
    /// the others, line by line.
    pub fn toggle_bookmarks(&mut self) -> Result<()> {
        let (first, last) = self.bookmark_target_lines()?;
        let count = self.doc().line_count();
        for line in first..=last.min(count.saturating_sub(1)) {
            // `GetLine` is null past the last line: nothing to mark there
            if line < count {
                self.model.bookmarks_mut().toggle(line);
            }
        }
        self.changes.bookmarks_changed = true;
        Ok(())
    }

    /// `F_BOOKMARK_RESET`: removes every bookmark.
    pub fn clear_bookmarks(&mut self) {
        self.model.bookmarks_mut().clear();
        self.changes.bookmarks_changed = true;
    }

    /// Bookmarks the given lines (`F_BOOKMARK_PATTERN`); those already marked stay marked, and
    /// lines that do not exist are ignored.
    pub fn add_bookmarks(&mut self, lines: impl IntoIterator<Item = usize>) {
        let count = self.doc().line_count();
        for line in lines.into_iter().filter(|&l| l < count) {
            self.model.bookmarks_mut().set(line, true);
        }
        self.changes.bookmarks_changed = true;
    }

    /// `F_BOOKMARK_NEXT` and `F_BOOKMARK_PREV`: moves the caret to the start of the next (or
    /// previous) bookmarked line, as a plain move (it extends the selection while the selection
    /// is locked, and it is not put in the jump history). With `wrap` (Sakura's option 先頭（末尾）
    /// から再検索, `m_bSearchAll`) a search that finds nothing goes on from the other end of the
    /// file, and may end on the line of the caret.
    pub fn jump_to_bookmark(&mut self, direction: Direction, wrap: bool) -> Result<BookmarkJump> {
        let line = self.caret().line;
        let marks = self.model.bookmarks();
        let (found, wrapped) = match direction {
            Direction::Forward => match marks.next_after(line) {
                Some(l) => (Some(l), false),
                None if wrap => (marks.first(), true),
                None => (None, false),
            },
            Direction::Backward => match marks.prev_before(line) {
                Some(l) => (Some(l), false),
                None if wrap => (marks.last(), true),
                None => (None, false),
            },
        };
        let Some(target) = found else {
            return Ok(BookmarkJump::NotFound);
        };
        let extend = self.extends(false);
        self.move_caret(Pos::new(target, 0), extend)?;
        Ok(BookmarkJump::Found { wrapped })
    }
}
