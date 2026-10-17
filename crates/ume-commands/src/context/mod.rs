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

mod bookmarks;
mod box_edit;
#[cfg(test)]
mod box_edit_tests;
mod boxsel;
mod clipboard;
mod convert;
mod diff;
mod funclist;
mod indent;
mod lines;
mod macro_support;
mod modes;
mod modified_lines;
#[cfg(test)]
mod modified_lines_tests;
mod motion;
mod paragraph;
mod view_mode;
mod word;

use std::path::Path;

use ume_core::buffer::{Document, Edit, Eol, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;

use crate::clipboard::Clipboard;
use crate::command::{CommandError, Result};
use crate::layout::Layout;

pub use bookmarks::BookmarkJump;
pub use boxsel::MAX_BOX_COL;
pub(crate) use clipboard::QUOTE;
pub use diff::DiffJump;
pub use funclist::FuncListJump;
pub use modes::eol_of_type;
pub use modified_lines::ModifiedLineJump;
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
    /// `F_SET_QUOTESTRING` set this quote string. `ume-ui` keeps it for the commands after (it is
    /// shared by all windows in Sakura) and hands it to the next context ([`CommandContext::
    /// with_quote_string`]).
    pub quote_string: Option<String>,
    /// The bookmarks of the document changed (set, cleared, or marked by a search). Edits move
    /// them too, but those are reported as edits. `ume-ui` repaints every view of the document.
    pub bookmarks_changed: bool,
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
    /// View mode: every edit is refused. See [`with_view_mode`](Self::with_view_mode).
    view_mode: bool,
    /// The quote string of `F_COPYLINESASPASSAGE`. See [`with_quote_string`](Self::with_quote_string).
    quote: String,
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
            view_mode: false,
            quote: QUOTE.to_owned(),
        }
    }

    /// Attaches the clipboard that the clipboard commands use. A context without one answers
    /// those commands with [`CommandError::Unavailable`].
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

    /// Sets view mode before the command runs (the default is off). While it is on, the text
    /// cannot be changed: an edit, undo, redo and the switch between insert and overwrite mode
    /// beep and fail with [`CommandError::Unavailable`]. Sakura (`F_VIEWMODE`,
    /// `CEditDoc::IsEditable` and `EIsModificationForbidden` in `sakura_core/doc/CEditDoc.cpp`)
    /// greys out those functions; here the one place every edit goes through refuses them, so keys,
    /// macros and plugins are covered alike.
    #[must_use]
    pub fn with_view_mode(mut self, view_mode: bool) -> Self {
        self.view_mode = view_mode;
        self
    }

    /// Sets the quote string that `F_COPYLINESASPASSAGE` puts before each row (the default is
    /// Sakura's `"> "`). The window keeps the value that `F_SET_QUOTESTRING` set.
    #[must_use]
    pub fn with_quote_string(mut self, quote: impl Into<String>) -> Self {
        self.quote = quote.into();
        self
    }

    /// Ends the context and returns what changed.
    pub fn finish(mut self) -> Changes {
        self.changes.caret_moved = self.cursor.caret() != self.start.caret()
            || self.cursor.box_caret_col() != self.start.box_caret_col();
        self.changes.selection_changed = self.cursor.anchor() != self.start.anchor()
            || self.cursor.mode() != self.start.mode()
            || self.cursor.box_anchor_col() != self.start.box_anchor_col();
        self.changes.overwrite = self.overwrite;
        self.changes
    }

    // ---- reading ----

    /// Whether view mode is on ([`with_view_mode`](Self::with_view_mode)).
    pub fn is_view_mode(&self) -> bool {
        self.view_mode
    }

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

    /// Whether there is a block to undo (Sakura: `CDocEditor::IsEnableUndo`). View mode does not
    /// change the answer; `undo` itself refuses there.
    pub fn can_undo(&self) -> bool {
        self.model.history().can_undo()
    }

    /// Whether there is a block to redo (Sakura: `CDocEditor::IsEnableRedo`).
    pub fn can_redo(&self) -> bool {
        self.model.history().can_redo()
    }

    /// Whether line `line` has a bookmark (Sakura: `CBookmarkGetter::IsBookmarked`).
    pub fn is_bookmarked(&self, line: usize) -> bool {
        self.model.bookmarks().contains(line)
    }

    /// Whether line `line` differs from the saved version (Sakura: `CModifyVisitor::IsLineModified`).
    pub fn is_line_modified(&self, line: usize) -> bool {
        self.model.is_line_modified(line)
    }

    /// The modification sequence number of `line` (Sakura: `CDocLine::m_sMark.m_cModified.GetSeq`).
    pub fn line_modify_seq(&self, line: usize) -> u64 {
        self.model.history().marks().seq_of(line)
    }

    /// Whether the document has changes that were not saved (Sakura: `CDocEditor::IsModified`).
    pub fn is_modified(&self) -> bool {
        self.model.is_modified()
    }

    /// Undoes one block. Returns `false` if there is nothing to undo, or in view mode.
    pub fn undo(&mut self) -> bool {
        if self.view_mode {
            return false;
        }
        let done = self.model.undo(self.cursor);
        if done {
            self.layout.reset(self.model.doc());
        }
        self.changes.replaced_all |= done;
        done
    }

    /// Redoes one block. Returns `false` if there is nothing to redo, or in view mode.
    pub fn redo(&mut self) -> bool {
        if self.view_mode {
            return false;
        }
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
    ///
    /// Fails while a box selection covers something or the caret is to the right of the text in
    /// one: this edits a range, not a box, so a command that wants to edit a box goes through
    /// [`delete_selection`](Self::delete_selection), [`insert_text`](Self::insert_text) or the
    /// methods of `box_edit`, which end the box first.
    pub fn replace(&mut self, from: Pos, to: Pos, text: &str) -> Result<Edit> {
        self.refuse_edit_in_box()?;
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

    /// While a box covers something, or the caret of a box is to the right of the text of its
    /// row, a range edit beeps and fails, so that a command that does not know boxes stops before
    /// it changes anything (it would take the stream between the corners for the selection).
    /// A box that covers nothing, with its caret on the text, is only a mode and the edit ends
    /// it.
    fn refuse_edit_in_box(&mut self) -> Result<()> {
        self.refuse_in_view_mode()?;
        if self.cursor.is_box() && self.caret_is_in_empty_space()? {
            self.beep();
            return Err(CommandError::Unavailable("editing a box selection"));
        }
        self.refuse_box_selection()
    }

    /// In view mode: beeps and fails. For what changes the text or the typing mode.
    pub(crate) fn refuse_in_view_mode(&mut self) -> Result<()> {
        if self.view_mode {
            self.beep();
            return Err(CommandError::Unavailable("editing in view mode"));
        }
        Ok(())
    }

    /// For the commands that work on the rows of a selection, before they look at it: they would
    /// take the stream between the corners of a box for their selection. They beep and fail.
    pub(crate) fn refuse_box_selection(&mut self) -> Result<()> {
        if self.cursor.is_box() && self.cursor.has_selection() {
            self.beep();
            return Err(CommandError::Unavailable("editing a box selection"));
        }
        Ok(())
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
    ///
    /// With a box selected the first line of `text` goes into every row of the box instead
    /// (`insert_into_box`), and with a box caret to the right of the
    /// text the row is padded with spaces first.
    pub fn insert_text(&mut self, text: &str) -> Result<Edit> {
        if self.cursor.is_box() {
            if self.cursor.has_selection() {
                return self.insert_into_box(text);
            }
            if self.caret_is_in_empty_space()? {
                return self.insert_in_empty_space(text);
            }
        }
        self.refuse_edit_in_box()?;
        let (from, to) = self.selection().unwrap_or((self.caret(), self.caret()));
        let edit = self.replace(from, to, text)?;
        self.cursor.move_to(edit.end, false);
        Ok(edit)
    }

    /// Deletes the text between two positions and puts the caret where it was.
    pub fn delete_range(&mut self, from: Pos, to: Pos) -> Result<()> {
        self.refuse_edit_in_box()?;
        let edit = self.replace(from, to, "")?;
        self.cursor.move_to(edit.start, false);
        Ok(())
    }

    /// Deletes the selected text. Returns `false` if nothing is selected.
    ///
    /// A box selection is deleted as a box: the text of each row of it (see
    /// `delete_box`).
    pub fn delete_selection(&mut self) -> Result<bool> {
        if self.cursor.is_box() && self.cursor.has_selection() {
            self.delete_box()?;
            return Ok(true);
        }
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
        // In a box the typing mode does not matter (see `box_edit`).
        if self.cursor.is_box()
            && (self.cursor.has_selection() || self.caret_is_in_empty_space()?)
        {
            return self.insert_text(text);
        }
        self.refuse_edit_in_box()?;
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
