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

mod clipboard;
mod convert;
mod indent;
mod lines;
mod motion;
mod paragraph;
mod word;

use std::path::Path;

use ume_core::buffer::{Document, Edit, Eol, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;

use crate::clipboard::Clipboard;
use crate::command::Result;
use crate::layout::Layout;

pub(crate) use clipboard::QUOTE;
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

/// Line endings that an edit joins into one CRLF. The buffer reports positions in the new text,
/// where the CR and the LF of a CRLF are one line ending, so the reported change has to be
/// widened to the whole line ending.
#[derive(Clone, Copy, Debug, Default)]
struct Joined {
    /// The end of the text of the line before the edit, if the CR that ends that line is joined
    /// with an LF (an inserted one, or the one after the deleted range).
    cr_before: Option<Pos>,
    /// Whether the LF that follows the edited range is joined with a CR (an inserted one, or the
    /// one before the deleted range).
    lf_after: bool,
}

/// The change that `replace(a, b, ..)` made, as a [`TextChange`], widened over the line endings
/// that were joined.
fn widen(a: Pos, b: Pos, edit: Edit, joined: Joined) -> TextChange {
    let start = edit.start.min(a).min(joined.cr_before.unwrap_or(a));
    let (mut old_end, mut new_end) = (b, edit.end);
    if start < a {
        // the CRLF ends at the start of the next line
        new_end = new_end.max(Pos::new(start.line + 1, 0));
    }
    if joined.lf_after {
        old_end = Pos::new(b.line + 1, 0);
        new_end = new_end.max(Pos::new(edit.end.line + 1, 0));
    }
    TextChange {
        start,
        old_end,
        new_end,
    }
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
    /// Whether typing overwrites after the command (Sakura: not `IsInsMode`). `ume-ui` keeps the
    /// mode in the document and writes this back after every command.
    pub overwrite: bool,
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
    layout: &'a mut dyn Layout,
    path: Option<&'a Path>,
    /// The clipboard, if the caller attached one ([`with_clipboard`](Self::with_clipboard)).
    clipboard: Option<&'a mut dyn Clipboard>,
    /// The cursor when the context was made, to tell what moved.
    start: Cursor,
    changes: Changes,
    /// Typing overwrites the char at the caret instead of inserting before it. See
    /// [`set_overwrite`](Self::set_overwrite).
    overwrite: bool,
}

impl<'a> CommandContext<'a> {
    pub fn new(
        model: &'a mut DocumentModel,
        cursor: &'a mut Cursor,
        layout: &'a mut dyn Layout,
        path: Option<&'a Path>,
    ) -> Self {
        let start = cursor.clone();
        Self {
            model,
            cursor,
            layout,
            path,
            clipboard: None,
            start,
            changes: Changes::default(),
            overwrite: false,
        }
    }

    /// Attaches the clipboard that the clipboard commands use. A context without one answers
    /// those commands with [`CommandError::Unavailable`](crate::command::CommandError::Unavailable).
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: &'a mut dyn Clipboard) -> Self {
        self.clipboard = Some(clipboard);
        self
    }

    /// Sets the typing mode before the command runs (the default is insert mode). Not part of
    /// [`new`](Self::new) so that its callers need no change.
    #[must_use]
    pub fn with_overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Ends the context and returns what changed.
    pub fn finish(mut self) -> Changes {
        self.changes.caret_moved = self.cursor.caret() != self.start.caret();
        self.changes.selection_changed =
            self.cursor.anchor() != self.start.anchor() || self.cursor.mode() != self.start.mode();
        self.changes.overwrite = self.overwrite;
        self.changes
    }

    // ---- reading ----

    /// Whether typing overwrites (Sakura: `!CEditView::IsInsMode`).
    pub fn is_overwrite(&self) -> bool {
        self.overwrite
    }

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
        &*self.layout
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
        if done {
            self.layout.reset(self.model.doc());
        }
        self.changes.replaced_all |= done;
        done
    }

    /// Redoes one block. Returns `false` if there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let done = self.model.redo(self.cursor);
        if done {
            self.layout.reset(self.model.doc());
        }
        self.changes.replaced_all |= done;
        done
    }

    // ---- editing ----

    /// Replaces the text between two positions (in either order) with `text`. The caret and the
    /// selection stay as they are; the other methods below move them as a command expects.
    pub fn replace(&mut self, from: Pos, to: Pos, text: &str) -> Result<Edit> {
        let (a, b) = (self.doc().clamp_pos(from)?, self.doc().clamp_pos(to)?);
        let (a, b) = (a.min(b), a.max(b));
        let joined = self.joined_by(a, b, text)?;
        let edit = self.model.replace(a, b, text)?;
        if a != b || !text.is_empty() {
            let change = widen(a, b, edit, joined);
            // The layout is updated now, not after the command: the command may move vertically
            // next, or a macro may run more commands.
            self.layout.edited(self.model.doc(), &change);
            self.changes.edits.push(change);
        }
        Ok(edit)
    }

    /// Which line endings an edit that replaces `a..b` with `text` joins (see [`Joined`]).
    fn joined_by(&self, a: Pos, b: Pos, text: &str) -> Result<Joined> {
        // The CR that ends the line before `a` and the LF that ends the line of `b`, if `a` is at
        // the start of a line and `b` at the end of the text of one.
        let cr_before = if a.col == 0 && a.line > 0 {
            self.doc()
                .line(a.line - 1)?
                .filter(|l| l.eol == Some(Eol::Cr))
                .map(|l| Pos::new(a.line - 1, l.text.len()))
        } else {
            None
        };
        let lf_after = self
            .doc()
            .line(b.line)?
            .is_some_and(|l| l.eol == Some(Eol::Lf) && l.text.len() == b.col);
        // They are joined by the new text if it starts with an LF or ends with a CR, and by each
        // other if nothing is put between them.
        Ok(Joined {
            cr_before: cr_before
                .filter(|_| text.starts_with('\n') || (text.is_empty() && lf_after)),
            lf_after: lf_after
                && (text.ends_with('\r') || (text.is_empty() && cr_before.is_some())),
        })
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

    /// Switches between insert and overwrite mode (Sakura: `SetInsMode`).
    pub fn set_overwrite(&mut self, overwrite: bool) {
        self.overwrite = overwrite;
    }

    /// The end of the text that typing `chars` chars at `at` overwrites in overwrite mode: the
    /// next `chars` chars of the line, fewer at the end of the text of the line. A line ending is
    /// never overwritten, and neither is the end of the document, so there the typed text is
    /// inserted. Sakura: `CViewCommander::DelCharForOverwrite` in
    /// `sakura_core/cmd/CViewCommander_Edit.cpp` with the defaults `m_bNotOverWriteCRLF` on and
    /// `m_bOverWriteFixMode` off (`sakura_core/env/CShareData.cpp`).
    pub fn overwrite_end(&self, at: Pos, chars: usize) -> Result<Pos> {
        let Some(line) = self.doc().line(at.line)? else {
            return Ok(at);
        };
        let rest = line.text.get(at.col..).unwrap_or("");
        let len: usize = rest.chars().take(chars).map(char::len_utf8).sum();
        Ok(Pos::new(at.line, at.col + len))
    }

    /// Types `text` at the caret as the typing mode says: insert mode inserts it, overwrite mode
    /// replaces as many chars after the caret as `text` has (see
    /// [`overwrite_end`](Self::overwrite_end)). A selection is replaced in both modes. The caret
    /// ends after the text.
    pub fn type_text(&mut self, text: &str) -> Result<Edit> {
        if self.overwrite && self.selection().is_none() {
            let at = self.caret();
            let end = self.overwrite_end(at, text.chars().count())?;
            let edit = self.replace(at, end, text)?;
            self.cursor.move_to(edit.end, false);
            Ok(edit)
        } else {
            self.insert_text(text)
        }
    }

    // ---- feedback ----

    /// Tells the UI that the command could not do what was asked (Sakura: `ErrorBeep()`).
    pub fn beep(&mut self) {
        self.changes.beep = true;
    }
}

#[cfg(test)]
mod tests {
    use ume_core::buffer::Document;

    use super::*;
    use crate::layout::PlainLayout;

    /// Runs `f` on a context over `text` and returns the changes.
    fn changes_of(text: &str, f: impl FnOnce(&mut CommandContext<'_>)) -> Changes {
        let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
        let mut cursor = Cursor::default();
        let mut layout = PlainLayout::default();
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        f(&mut cx);
        cx.finish()
    }

    fn change(
        start: (usize, usize),
        old_end: (usize, usize),
        new_end: (usize, usize),
    ) -> TextChange {
        TextChange {
            start: Pos::new(start.0, start.1),
            old_end: Pos::new(old_end.0, old_end.1),
            new_end: Pos::new(new_end.0, new_end.1),
        }
    }

    #[test]
    fn an_edit_is_reported_with_the_old_and_the_new_end() {
        let c = changes_of("abcd", |cx| {
            cx.replace(Pos::new(0, 3), Pos::new(0, 1), "XYZ").unwrap();
        });
        assert_eq!(c.edits, [change((0, 1), (0, 3), (0, 4))]);
        assert!(c.text_changed());
    }

    #[test]
    fn an_edit_that_changes_nothing_is_not_reported() {
        let c = changes_of("abcd", |cx| {
            cx.replace(Pos::new(0, 1), Pos::new(0, 1), "").unwrap();
        });
        assert!(c.edits.is_empty());
        assert!(!c.text_changed());
    }

    #[test]
    fn a_deletion_that_joins_a_cr_and_an_lf_starts_at_the_cr() {
        // "a\r" "b" "\nc": deleting b makes the CR and the LF one CRLF
        let c = changes_of("a\rb\nc", |cx| {
            cx.replace(Pos::new(1, 0), Pos::new(1, 1), "").unwrap();
        });
        assert_eq!(c.edits, [change((0, 1), (2, 0), (1, 0))]);
    }

    #[test]
    fn an_inserted_lf_that_joins_a_cr_starts_at_the_cr() {
        let c = changes_of("a\r", |cx| {
            cx.replace(Pos::new(1, 0), Pos::new(1, 0), "\nb").unwrap();
        });
        assert_eq!(c.edits, [change((0, 1), (1, 0), (1, 1))]);
    }

    #[test]
    fn an_inserted_cr_that_joins_an_lf_ends_after_the_crlf() {
        let c = changes_of("a\nb", |cx| {
            cx.replace(Pos::new(0, 1), Pos::new(0, 1), "\r").unwrap();
        });
        assert_eq!(c.edits, [change((0, 1), (1, 0), (1, 0))]);
    }

    /// A layout that keeps its own copy of the number of lines, as a real layout keeps rows.
    /// Moving through it while the copy is stale panics.
    struct CachedLayout {
        inner: PlainLayout,
        lines: usize,
        edits: usize,
        resets: usize,
    }

    impl Layout for CachedLayout {
        fn tab_width(&self) -> usize {
            self.inner.tab_width()
        }
        fn edited(&mut self, doc: &Document, _change: &TextChange) {
            self.lines = doc.line_count();
            self.edits += 1;
        }
        fn reset(&mut self, doc: &Document) {
            self.lines = doc.line_count();
            self.resets += 1;
        }
        fn display_col(&self, doc: &Document, pos: Pos) -> ume_core::buffer::Result<usize> {
            self.inner.display_col(doc, pos)
        }
        fn row_start(&self, doc: &Document, pos: Pos) -> ume_core::buffer::Result<Pos> {
            self.inner.row_start(doc, pos)
        }
        fn row_end(&self, doc: &Document, pos: Pos) -> ume_core::buffer::Result<Pos> {
            self.inner.row_end(doc, pos)
        }
        fn next_row_start(
            &self,
            doc: &Document,
            pos: Pos,
        ) -> ume_core::buffer::Result<Option<Pos>> {
            self.inner.next_row_start(doc, pos)
        }
        fn pos_at_col(
            &self,
            doc: &Document,
            row_pos: Pos,
            col: usize,
        ) -> ume_core::buffer::Result<Pos> {
            self.inner.pos_at_col(doc, row_pos, col)
        }
        fn move_rows(
            &self,
            doc: &Document,
            pos: Pos,
            rows: isize,
            goal_col: usize,
        ) -> ume_core::buffer::Result<Pos> {
            assert_eq!(self.lines, doc.line_count(), "the layout is stale");
            self.inner.move_rows(doc, pos, rows, goal_col)
        }
    }

    #[test]
    fn the_layout_is_updated_right_after_each_edit() {
        // one command: insert a line break, then move down through the layout
        let mut model = DocumentModel::new(Document::from_utf8(b"ab".to_vec()).unwrap());
        let mut cursor = Cursor::new(Pos::new(0, 1));
        let mut layout = CachedLayout {
            inner: PlainLayout::default(),
            lines: 1,
            edits: 0,
            resets: 0,
        };
        {
            let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
            cx.begin_block();
            cx.insert_text("\nxyz\n").unwrap();
            cx.move_vertical(-1, false).unwrap();
            cx.move_vertical(1, false).unwrap();
            // undo resyncs the layout too
            assert!(cx.undo());
            cx.move_vertical(1, false).unwrap();
            assert!(cx.redo());
            cx.move_vertical(-1, false).unwrap();
            cx.end_block();
        }
        assert_eq!((layout.edits, layout.resets), (1, 2));
    }

    #[test]
    fn edits_are_reported_in_order() {
        let c = changes_of("abc", |cx| {
            cx.replace(Pos::new(0, 0), Pos::new(0, 0), "X").unwrap();
            cx.replace(Pos::new(0, 2), Pos::new(0, 3), "").unwrap();
        });
        assert_eq!(c.edits.len(), 2);
        assert_eq!(c.edits[1], change((0, 2), (0, 3), (0, 2)));
    }
}
