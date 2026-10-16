//! The document with its undo history: what `ume-ui` wraps in `Entity<DocumentModel>`
//! (ADR-0013 section 3).
//!
//! All edits go through [`DocumentModel::replace`], which tells the history, so that the
//! document and the undo blocks cannot disagree.

use crate::bookmarks::{Bookmarks, LineDelta, LineEdit, TextBreaks};
use crate::buffer::{Document, Edit, Eol, Pos, Result};
use crate::cursor::Cursor;
use crate::undo::History;

/// A document, its undo history, and the line ending that new line breaks get.
#[derive(Debug)]
pub struct DocumentModel {
    doc: Document,
    history: History,
    new_line: Eol,
    bookmarks: Bookmarks,
}

impl DocumentModel {
    /// A model over `doc`. The line ending for new line breaks is the first one in the document,
    /// and the line ending of the OS if there is none ([`Eol::platform_default`]). This is
    /// an approximation of Sakura's rule (`CDocEditor::GetNewLineCode`, set when the file is
    /// loaded); the file loader will set it with [`set_new_line`](Self::set_new_line).
    pub fn new(doc: Document) -> Self {
        let new_line = doc
            .line(0)
            .ok()
            .flatten()
            .and_then(|l| l.eol)
            .unwrap_or_else(Eol::platform_default);
        Self {
            doc,
            history: History::new(),
            new_line,
            bookmarks: Bookmarks::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new(Document::empty())
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// The bookmarked lines. They follow the edits, also those of undo and redo (see
    /// [`crate::bookmarks`]).
    pub fn bookmarks(&self) -> &Bookmarks {
        &self.bookmarks
    }

    /// The bookmarks, to set and clear them. Bookmarks are not part of the undo history.
    pub fn bookmarks_mut(&mut self) -> &mut Bookmarks {
        &mut self.bookmarks
    }

    /// The line ending that Enter inserts.
    pub fn new_line(&self) -> Eol {
        self.new_line
    }

    pub fn set_new_line(&mut self, eol: Eol) {
        self.new_line = eol;
    }

    /// Opens an undo block; see [`History::begin`]. `cursor` is the cursor before the command.
    pub fn begin_block(&mut self, cursor: &Cursor) {
        self.history.begin(&self.doc, cursor);
    }

    /// Closes an undo block; see [`History::end`]. `cursor` is the cursor after the command.
    pub fn end_block(&mut self, cursor: &Cursor) {
        self.history.end(&self.doc, cursor);
    }

    /// Replaces the text between two positions (in either order, clamped) with `text`.
    pub fn replace(&mut self, from: Pos, to: Pos, text: &str) -> Result<Edit> {
        let (from, to) = (self.doc.clamp_pos(from)?, self.doc.clamp_pos(to)?);
        if from == to && text.is_empty() {
            return Ok(Edit {
                start: from,
                end: from,
            });
        }
        // The document is left as it was when the replace fails, so the history is told after.
        let rows_before = self.rows();
        let edit = self.doc.replace(from, to, text)?;
        self.history.note_edit();
        let (a, b) = (from.min(to), from.max(to));
        let line_edit = LineEdit::of_replace(
            (a.line, a.col),
            (b.line, b.col),
            (edit.end.line, edit.end.col),
            TextBreaks {
                count: b.line - a.line,
                ends_with_break: b.col == 0 && b.line > a.line,
            },
            TextBreaks::of(text),
            rows_before,
            self.rows(),
        );
        for delta in line_edit.forward() {
            self.bookmarks.apply(delta);
        }
        self.history.note_lines(line_edit);
        // A last line that lost all its text is gone (Sakura deletes the line object), and its
        // bookmark with it.
        self.bookmarks.truncate(self.doc.line_count());
        Ok(edit)
    }

    /// The number of rows of the text: its line breaks plus one, so the empty row after a final
    /// line break counts (which `line_count` leaves out).
    fn rows(&self) -> usize {
        self.doc
            .end_pos()
            .map_or_else(|_| self.doc.line_count(), |p| p.line + 1)
    }

    /// Undoes one block, and puts the cursor back. `false` if there is nothing to undo.
    pub fn undo(&mut self, cursor: &mut Cursor) -> bool {
        let Some(lines) = self.history.undo_lines(&mut self.doc, cursor) else {
            return false;
        };
        self.apply_lines(&lines);
        true
    }

    /// Redoes one block, and puts the cursor back. `false` if there is nothing to redo.
    pub fn redo(&mut self, cursor: &mut Cursor) -> bool {
        let Some(lines) = self.history.redo_lines(&mut self.doc, cursor) else {
            return false;
        };
        self.apply_lines(&lines);
        true
    }

    fn apply_lines(&mut self, lines: &[LineDelta]) {
        for &delta in lines {
            self.bookmarks.apply(delta);
        }
        self.bookmarks.truncate(self.doc.line_count());
    }

    pub fn is_modified(&self) -> bool {
        self.history.is_modified()
    }

    /// Records that the document was saved as it is now. `cursor` is the current cursor.
    pub fn mark_saved(&mut self, cursor: &Cursor) {
        self.history.mark_saved(&self.doc, cursor);
    }
}

/// The text of a line ending, to insert it.
pub fn eol_text(eol: Eol) -> &'static str {
    match eol {
        Eol::CrLf => "\r\n",
        Eol::Lf => "\n",
        Eol::Cr => "\r",
        Eol::Nel => "\u{85}",
        Eol::Ls => "\u{2028}",
        Eol::Ps => "\u{2029}",
    }
}

impl From<Document> for DocumentModel {
    fn from(doc: Document) -> Self {
        Self::new(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(s: &str) -> DocumentModel {
        DocumentModel::new(Document::from_utf8(s.as_bytes().to_vec()).unwrap())
    }

    #[test]
    fn the_new_line_is_the_first_one_of_the_document() {
        assert_eq!(model("a\nb\r\n").new_line(), Eol::Lf);
        assert_eq!(model("a\r\nb\n").new_line(), Eol::CrLf);
        assert_eq!(
            model("abc").new_line(),
            if cfg!(windows) { Eol::CrLf } else { Eol::Lf }
        );
    }

    #[test]
    fn edits_outside_a_block_cannot_be_undone() {
        let mut m = model("ab");
        m.replace(Pos::new(0, 1), Pos::new(0, 1), "x").unwrap();
        assert!(!m.history().can_undo());
        assert!(m.is_modified());
    }

    #[test]
    fn an_empty_replace_is_not_an_edit() {
        let mut m = model("ab");
        let cursor = Cursor::default();
        m.begin_block(&cursor);
        m.replace(Pos::new(0, 1), Pos::new(0, 1), "").unwrap();
        m.end_block(&cursor);
        assert!(!m.history().can_undo());
        assert!(!m.is_modified());
    }

    #[test]
    fn eol_text_round_trips_through_the_buffer() {
        for eol in [Eol::CrLf, Eol::Lf, Eol::Cr, Eol::Nel, Eol::Ls, Eol::Ps] {
            let mut m = model("");
            m.replace(
                Pos::new(0, 0),
                Pos::new(0, 0),
                &format!("a{}b", eol_text(eol)),
            )
            .unwrap();
            assert_eq!(m.doc().line(0).unwrap().unwrap().eol, Some(eol));
        }
    }

    /// Runs one edit as one undo block.
    fn edit(m: &mut DocumentModel, from: (usize, usize), to: (usize, usize), text: &str) {
        let cursor = Cursor::default();
        m.begin_block(&cursor);
        m.replace(Pos::new(from.0, from.1), Pos::new(to.0, to.1), text)
            .unwrap();
        m.end_block(&cursor);
    }

    fn marked(m: &DocumentModel) -> Vec<usize> {
        m.bookmarks().iter().collect()
    }

    #[test]
    fn bookmarks_follow_their_lines() {
        let mut m = model(
            "a
b
c
d",
        );
        m.bookmarks_mut().set(2, true);
        edit(
            &mut m,
            (0, 0),
            (0, 0),
            "x
",
        ); // a line above
        assert_eq!(marked(&m), [3]);
        edit(&mut m, (0, 0), (1, 0), ""); // and gone again
        assert_eq!(marked(&m), [2]);
        edit(&mut m, (2, 0), (3, 0), ""); // the bookmarked line itself
        assert_eq!(marked(&m), Vec::<usize>::new());
    }

    #[test]
    fn undo_and_redo_move_bookmarks_like_the_inverse_edit() {
        let mut m = model(
            "a
b
c",
        );
        let mut cursor = Cursor::default();
        m.bookmarks_mut().set(2, true);
        edit(
            &mut m,
            (0, 0),
            (0, 0),
            "x
y
",
        );
        assert_eq!(marked(&m), [4]);
        assert!(m.undo(&mut cursor));
        assert_eq!(marked(&m), [2]);
        assert!(m.redo(&mut cursor));
        assert_eq!(marked(&m), [4]);
        // a deleted line loses its bookmark for good: undo brings the text back without it
        m.bookmarks_mut().set(1, true);
        edit(&mut m, (1, 0), (2, 0), "");
        assert_eq!(marked(&m), [3]);
        assert!(m.undo(&mut cursor));
        assert_eq!(marked(&m), [4]);
    }

    #[test]
    fn enter_at_the_start_of_a_line_moves_its_bookmark_down() {
        let mut m = model("abc");
        m.bookmarks_mut().set(0, true);
        edit(&mut m, (0, 0), (0, 0), "\n");
        assert_eq!(marked(&m), [1]);
        let mut m = model("abc");
        m.bookmarks_mut().set(0, true);
        edit(&mut m, (0, 1), (0, 1), "\n");
        assert_eq!(marked(&m), [0]);
    }

    #[test]
    fn crlf_lines_count_once() {
        let mut m = model("a\r\nb\r\nc");
        m.bookmarks_mut().set(2, true);
        edit(&mut m, (0, 1), (1, 0), ""); // joins "a" and "b"
        assert_eq!(marked(&m), [1]);
    }

    #[test]
    fn undo_keeps_the_bookmark_of_the_line_that_survives_the_inverse_edit() {
        // "\nx" typed at the start of "abc": "" above, "xabc" below; the bookmark is on "xabc"
        let mut m = model("abc");
        let mut cursor = Cursor::default();
        edit(&mut m, (0, 0), (0, 0), "\nx");
        m.bookmarks_mut().set(1, true);
        assert!(m.undo(&mut cursor));
        assert_eq!(marked(&m), [0]);
        // redo runs the edit again: the line above keeps the bookmark of the line it was
        assert!(m.redo(&mut cursor));
        assert_eq!(marked(&m), [0]);
    }

    #[test]
    fn a_cr_and_an_lf_that_meet_are_one_line_break() {
        // "a\rb\nc": deleting "b" makes "a\r\nc"
        let mut m = model("a\rb\nc");
        m.bookmarks_mut().set(2, true);
        edit(&mut m, (1, 0), (1, 1), "");
        assert_eq!(m.doc().line_count(), 2);
        assert_eq!(marked(&m), [1]);
    }
}
