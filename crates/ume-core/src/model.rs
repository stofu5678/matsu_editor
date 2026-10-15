//! The document with its undo history: what `ume-ui` wraps in `Entity<DocumentModel>`
//! (ADR-0013 section 3).
//!
//! All edits go through [`DocumentModel::replace`], which tells the history, so that the
//! document and the undo blocks cannot disagree.

use crate::buffer::{Document, Edit, Eol, Pos, Result};
use crate::cursor::Cursor;
use crate::undo::History;

/// A document, its undo history, and the line ending that new line breaks get.
#[derive(Debug)]
pub struct DocumentModel {
    doc: Document,
    history: History,
    new_line: Eol,
}

impl DocumentModel {
    /// A model over `doc`. The line ending for new line breaks is the first one in the document,
    /// and CRLF if there is none (the default of a new document in Sakura on Windows). This is
    /// an approximation of Sakura's rule (`CDocEditor::GetNewLineCode`, set when the file is
    /// loaded); the file loader will set it with [`set_new_line`](Self::set_new_line).
    pub fn new(doc: Document) -> Self {
        let new_line = doc
            .line(0)
            .ok()
            .flatten()
            .and_then(|l| l.eol)
            .unwrap_or(Eol::CrLf);
        Self {
            doc,
            history: History::new(),
            new_line,
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
        let edit = self.doc.replace(from, to, text)?;
        self.history.note_edit();
        Ok(edit)
    }

    /// Undoes one block, and puts the cursor back. `false` if there is nothing to undo.
    pub fn undo(&mut self, cursor: &mut Cursor) -> bool {
        self.history.undo(&mut self.doc, cursor)
    }

    /// Redoes one block, and puts the cursor back. `false` if there is nothing to redo.
    pub fn redo(&mut self, cursor: &mut Cursor) -> bool {
        self.history.redo(&mut self.doc, cursor)
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
        assert_eq!(model("abc").new_line(), Eol::CrLf);
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
}
