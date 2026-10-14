//! [`CommandContext`]: what a command can read and change while it runs (ADR-0013 section 4).
//!
//! The context is a bundle of borrows that lives for one command (or one macro run). It holds no
//! GPUI type, so commands are tested on plain structs. It has
//!
//! * the document and its undo history ([`DocumentModel`]),
//! * the cursor of the view that runs the command (caret, selection, goal column),
//! * the [`Layout`], for vertical moves and display columns,
//! * the path of the document, if it has a file,
//! * and a record of what the command changed ([`Changes`]), for `ume-ui`.
//!
//! Edits and moves go through the methods below, never through the fields. The methods keep the
//! document, the cursor and the record consistent, and the document records undo.
//! The methods that several commands share (moves, delete, insert at the caret) live here, so a
//! command file only says *which* of them to call (see `crate::commands`).

mod motion;

use std::path::Path;

use ume_core::buffer::{Document, Edit, Eol, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;

use crate::command::Result;
use crate::layout::Layout;

pub(crate) use motion::is_indent_char;

/// One edit of the text, in the coordinates of the document before the edit for `old_end` and
/// after it for `new_end`: the text from `start` to `old_end` is gone, and the text from `start`
/// to `new_end` is new.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextChange {
    pub start: Pos,
    pub old_end: Pos,
    pub new_end: Pos,
}

/// What a command changed. `ume-ui` reads it after the command: it drops the IME composition
/// range when the text or the caret changed, scrolls the caret into view, repaints, and beeps.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// The edits of the text, in the order they were made.
    pub edits: Vec<TextChange>,
    /// Undo or redo replaced the document with an older or newer version; the edits are not
    /// known, so the whole text counts as changed.
    pub replaced_all: bool,
    /// The caret is somewhere else than before the command.
    pub caret_moved: bool,
    /// The selection (its anchor) is different from before the command.
    pub selection_changed: bool,
    /// The command could not do what was asked in this state (Sakura: `ErrorBeep()`).
    pub beep: bool,
}

impl Changes {
    /// Whether the text may be different from before the command.
    pub fn text_changed(&self) -> bool {
        self.replaced_all || !self.edits.is_empty()
    }
}

/// The state that one command works on. See the [module documentation](self).
pub struct CommandContext<'a> {
    model: &'a mut DocumentModel,
    cursor: &'a mut Cursor,
    layout: &'a dyn Layout,
    path: Option<&'a Path>,
    /// The cursor when the context was made, to tell what moved.
    start: Cursor,
    changes: Changes,
}

impl<'a> CommandContext<'a> {
    pub fn new(
        model: &'a mut DocumentModel,
        cursor: &'a mut Cursor,
        layout: &'a dyn Layout,
        path: Option<&'a Path>,
    ) -> Self {
        let start = cursor.clone();
        Self {
            model,
            cursor,
            layout,
            path,
            start,
            changes: Changes::default(),
        }
    }

    /// Ends the context and returns what changed.
    pub fn finish(mut self) -> Changes {
        self.changes.caret_moved = self.cursor.caret() != self.start.caret();
        self.changes.selection_changed =
            self.cursor.anchor() != self.start.anchor() || self.cursor.mode() != self.start.mode();
        self.changes
    }

    // ---- reading ----

    /// The document, to read by lines and ranges.
    pub fn doc(&self) -> &Document {
        self.model.doc()
    }

    pub fn cursor(&self) -> &Cursor {
        self.cursor
    }

    pub fn caret(&self) -> Pos {
        self.cursor.caret()
    }

    /// The selected range, smaller position first, or `None`.
    pub fn selection(&self) -> Option<(Pos, Pos)> {
        self.cursor.selection()
    }

    pub fn layout(&self) -> &dyn Layout {
        self.layout
    }

    /// The path of the document, or `None` if it has no file yet.
    pub fn path(&self) -> Option<&Path> {
        self.path
    }

    /// The line ending that Enter inserts.
    pub fn new_line(&self) -> Eol {
        self.model.new_line()
    }

    /// The display column of the caret.
    pub fn caret_display_col(&self) -> Result<usize> {
        Ok(self.layout.display_col(self.doc(), self.caret())?)
    }

    // ---- undo blocks ----

    /// Opens the undo block of the command. [`Registration::run`](crate::command::Registration::run)
    /// does it; call it yourself only to make several commands one undo step.
    pub fn begin_block(&mut self) {
        self.model.begin_block(self.cursor);
    }

    pub fn end_block(&mut self) {
        self.model.end_block(self.cursor);
    }

    /// Undoes one block. Returns `false` if there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let done = self.model.undo(self.cursor);
        self.changes.replaced_all |= done;
        done
    }

    /// Redoes one block. Returns `false` if there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let done = self.model.redo(self.cursor);
        self.changes.replaced_all |= done;
        done
    }

    // ---- editing ----

    /// Replaces the text between two positions (in either order) with `text`. The caret and the
    /// selection stay as they are; the other methods below move them as a command expects.
    pub fn replace(&mut self, from: Pos, to: Pos, text: &str) -> Result<Edit> {
        let (a, b) = (self.doc().clamp_pos(from)?, self.doc().clamp_pos(to)?);
        let (a, b) = (a.min(b), a.max(b));
        let edit = self.model.replace(a, b, text)?;
        if a != b || !text.is_empty() {
            self.changes.edits.push(TextChange {
                start: edit.start,
                old_end: b,
                new_end: edit.end,
            });
        }
        Ok(edit)
    }

    /// Inserts `text` at the caret, or in place of the selection, and puts the caret after it.
    pub fn insert_text(&mut self, text: &str) -> Result<Edit> {
        let (from, to) = self.selection().unwrap_or((self.caret(), self.caret()));
        let edit = self.replace(from, to, text)?;
        self.cursor.move_to(edit.end, false);
        Ok(edit)
    }

    /// Deletes the text between two positions and puts the caret where it was.
    pub fn delete_range(&mut self, from: Pos, to: Pos) -> Result<()> {
        let edit = self.replace(from, to, "")?;
        self.cursor.move_to(edit.start, false);
        Ok(())
    }

    /// Deletes the selected text. Returns `false` if nothing is selected.
    pub fn delete_selection(&mut self) -> Result<bool> {
        match self.selection() {
            Some((from, to)) => self.delete_range(from, to).map(|()| true),
            None => Ok(false),
        }
    }

    // ---- feedback ----

    /// Tells the UI that the command could not do what was asked (Sakura: `ErrorBeep()`).
    pub fn beep(&mut self) {
        self.changes.beep = true;
    }
}
