//! Undo and redo.
//!
//! Sakura keeps a list of operation blocks (`COpeBuf` of `COpeBlk`s, each a list of `COpe`s:
//! insert, delete, replace, caret move) and runs them backwards or forwards
//! (`sakura_core/cmd/COpeBuf.cpp`, `COpeBlk.cpp`, `CViewCommander_Edit.cpp` `Command_UNDO`). One
//! command is one block: `CViewCommander::HandleCommand` opens a block with a reference count,
//! the command's operations are appended, and `CEditView::SetUndoBuffer` stores the block when
//! the count drops to zero and the block is not empty. A new block drops the redo tail. Typing is
//! not merged: every typed character is a command and so a block of its own.
//!
//! # Versions instead of an operation log
//!
//! The buffer is persistent (ADR-0005): cloning a [`Document`] copies one pointer, and an edit
//! shares everything it did not touch. So a block here is the pair of document versions before and
//! after the command, plus the cursor before and after.
//!
//! * Undo and redo are O(1) and cannot go wrong: they swap in a version that existed. An operation
//!   log has to invert each operation exactly, including the case that a CRLF is cut in two pieces.
//! * A block whose command made many edits (a macro, replace all) costs the same as one edit.
//! * Memory: a block keeps the new tree path of its edits alive, O(log n) nodes per edit. The text
//!   itself is not copied twice: added text lives in the append-only add buffer that all versions
//!   share, and deleted original text stays in the source. A log would copy every deleted text
//!   into the block, which for "select all, delete" on a large file is the whole file. The add
//!   buffer is not trimmed by dropping old blocks (it only grows, also with a log); see
//!   [`Document::added_bytes`]. For very long sessions [`History::set_max_blocks`] forgets the
//!   oldest blocks.
//!
//! # Blocks
//!
//! [`History::begin`] and [`History::end`] nest like Sakura's reference count: only the outermost
//! `end` stores the block, so a macro that runs many commands is one undo step when it wraps them
//! in `begin`/`end`.

use std::collections::VecDeque;

use crate::bookmarks::{LineDelta, LineEdit};
use crate::buffer::Document;
use crate::cursor::Cursor;
use crate::modified::{MarkEdit, ModifiedLines, Seq};

/// A document version and the cursor that went with it.
#[derive(Clone, Debug)]
struct Snapshot {
    doc: Document,
    cursor: Cursor,
}

impl Snapshot {
    fn of(doc: &Document, cursor: &Cursor) -> Self {
        Self {
            doc: doc.clone(),
            cursor: cursor.clone(),
        }
    }
}

/// One undo step.
#[derive(Debug)]
struct Block {
    before: Snapshot,
    after: Snapshot,
    /// The changes of the line list, in the order the edits were made (for the bookmarks).
    lines: Vec<LineEdit>,
    /// The changes of the sequence numbers of the lines (for the modified lines).
    marks: Vec<MarkEdit>,
}

/// The block that is being made.
#[derive(Debug)]
struct Open {
    depth: usize,
    before: Snapshot,
    /// The block has something to store: an edit, or a caret step.
    edited: bool,
    /// The block has an edit of the text (not only a caret step).
    real: bool,
    lines: Vec<LineEdit>,
    marks: Vec<MarkEdit>,
}

/// The undo and redo history of a document.
#[derive(Debug)]
pub struct History {
    blocks: VecDeque<Block>,
    /// The number of blocks that are applied; `blocks[pos..]` can be redone.
    pos: usize,
    /// `pos` when the document was last saved (Sakura: `m_nNoModifiedIndex`).
    saved: Option<usize>,
    /// The document is unmodified although `pos` is not `saved`: a step that only holds the caret
    /// was stored while the document was unmodified (`F_APPENDUNDOBUFFERCURSOR`). Sakura's flag
    /// of the modified state (`CDocEditor::m_bIsDocModified`) is set by the edits, not by the
    /// position of the undo buffer; any edit, undo, redo or save clears this.
    caret_step_clean: bool,
    open: Option<Open>,
    max_blocks: Option<usize>,
    /// How many blocks were forgotten from the front, so that the sequence numbers of the
    /// lines (`origin + pos`) stay the same when `trim` forgets blocks.
    origin: Seq,
    /// The sequence number of each line (see [`crate::modified`]).
    marks: ModifiedLines,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// An empty history of an unmodified document.
    pub fn new() -> Self {
        Self {
            blocks: VecDeque::new(),
            pos: 0,
            saved: Some(0),
            caret_step_clean: false,
            open: None,
            max_blocks: None,
            origin: 0,
            marks: ModifiedLines::new(),
        }
    }

    /// Keeps at most `max` blocks, forgetting the oldest ones. `None` (the default, as in
    /// Sakura) keeps all.
    pub fn set_max_blocks(&mut self, max: Option<usize>) {
        self.max_blocks = max;
        self.trim();
    }

    pub fn can_undo(&self) -> bool {
        self.pos > 0 || self.open.as_ref().is_some_and(|o| o.edited)
    }

    pub fn can_redo(&self) -> bool {
        self.pos < self.blocks.len() && !self.open.as_ref().is_some_and(|o| o.edited)
    }

    /// Whether the document differs from the last saved version.
    pub fn is_modified(&self) -> bool {
        self.open.as_ref().is_some_and(|o| o.real) || self.is_stored_modified()
    }

    /// Whether the steps that are stored differ from the saved version, as far as the modified
    /// flag goes (a step that holds only the caret does not make the document modified).
    fn is_stored_modified(&self) -> bool {
        !self.caret_step_clean && self.saved != Some(self.pos)
    }

    /// Records that the document is saved as it is now. `doc` and `cursor` are the current state:
    /// the edits of a running block so far are stored as a step first (as `undo` does), so the
    /// saved version is a step that can be undone to and redone from. No line is modified
    /// afterwards (Sakura: `CModifyManager::OnAfterSave`).
    pub fn mark_saved(&mut self, doc: &Document, cursor: &Cursor) {
        self.store_open_edits(doc, cursor);
        self.saved = Some(self.pos);
        self.caret_step_clean = false;
        self.marks.reset(self.current_seq());
    }

    /// The sequence number of the version that is applied now (Sakura: `m_nCurrentPointer`).
    fn current_seq(&self) -> Seq {
        self.origin + self.pos as Seq
    }

    /// The sequence number the lines of the next edit get (`COpeBuf::GetNextSeq`).
    pub fn next_seq(&self) -> Seq {
        self.current_seq() + 1
    }

    /// The sequence number of the saved version, if it can still be reached.
    pub fn saved_seq(&self) -> Option<Seq> {
        self.saved.map(|s| self.origin + s as Seq)
    }

    /// The sequence numbers of the lines.
    pub fn marks(&self) -> &ModifiedLines {
        &self.marks
    }

    /// Whether `line` differs from the saved version (Sakura: `CModifyVisitor::IsLineModified`,
    /// drawn only while the document is modified at all). When there is no saved version to
    /// compare with, every line is modified.
    pub fn is_line_modified(&self, line: usize) -> bool {
        self.is_modified() && self.saved_seq() != Some(self.marks.seq_of(line))
    }

    /// Does an edit of the sequence numbers and keeps it in the block, so that undo can revert it.
    /// Call it after [`note_edit`](Self::note_edit).
    pub fn apply_marks(&mut self, edit: MarkEdit) {
        edit.apply(&mut self.marks);
        if let Some(open) = &mut self.open {
            open.marks.push(edit);
        }
    }

    /// Forgets the numbers of lines from `rows` on.
    pub fn truncate_marks(&mut self, rows: usize) {
        self.marks.truncate(rows);
    }

    /// Opens a block (or enters the open one). `doc` and `cursor` are the state before the
    /// command.
    pub fn begin(&mut self, doc: &Document, cursor: &Cursor) {
        match &mut self.open {
            Some(open) => open.depth += 1,
            None => {
                self.open = Some(Open {
                    depth: 1,
                    before: Snapshot::of(doc, cursor),
                    edited: false,
                    real: false,
                    lines: Vec::new(),
                    marks: Vec::new(),
                });
            }
        }
    }

    /// Tells the history that the document is about to change. Outside a block the change cannot
    /// be undone, and the older blocks would no longer match the document, so they are dropped.
    pub fn note_edit(&mut self) {
        match &mut self.open {
            Some(open) => {
                open.edited = true;
                open.real = true;
            }
            None => {
                self.caret_step_clean = false;
                self.blocks.clear();
                self.pos = 0;
                self.saved = None;
            }
        }
    }

    /// Tells the history how an edit changed the line list (and how undoing it will), after [`note_edit`](Self::note_edit),
    /// so that undo and redo can tell the bookmarks. Outside a block there is nothing to undo, so
    /// nothing is kept.
    pub fn note_lines(&mut self, edit: LineEdit) {
        if let Some(open) = &mut self.open {
            open.lines.push(edit);
        }
    }

    /// Leaves a block. The outermost `end` stores the block if the command edited the document.
    /// `doc` and `cursor` are the state after the command.
    pub fn end(&mut self, doc: &Document, cursor: &Cursor) {
        let Some(open) = &mut self.open else { return };
        open.depth -= 1;
        if open.depth > 0 {
            return;
        }
        if let Some(open) = self.open.take()
            && open.edited
        {
            self.push(
                open.before,
                Snapshot::of(doc, cursor),
                open.lines,
                open.marks,
                !open.real,
            );
        }
    }

    /// How many times the open block was entered and not yet left (Sakura: `COpeBlk::GetRefCount`),
    /// or 0 when no block is open.
    pub fn depth(&self) -> usize {
        self.open.as_ref().map_or(0, |open| open.depth)
    }

    /// Stores what the open block has so far as an undo step and goes on with a new block at the
    /// same depth (Sakura: `F_COMMITUNDOBUFFER` in `CMacro::HandleCommand`, which sets the
    /// reference count to 1, stores the block with `SetUndoBuffer`, and makes a new block with the
    /// old count). Nothing is stored when the block has no edit. Does nothing outside a block.
    pub fn commit(&mut self, doc: &Document, cursor: &Cursor) {
        self.store_open_edits(doc, cursor);
        // A block without an edit is dropped and a new one made (Sakura), so the next step starts
        // from here, not from where the block was entered.
        if let Some(open) = &mut self.open {
            open.before = Snapshot::of(doc, cursor);
        }
    }

    /// Makes the open block a step that undo goes back over even if it has no edit.
    ///
    /// Sakura: `F_APPENDUNDOBUFFERCURSOR` (`CMacro::HandleCommand`, `sakura_core/macro/CMacro.cpp`)
    /// appends a `CMoveCaretOpe`, an operation that holds only a caret position, and calls
    /// `CEditView::SetUndoBuffer` (`view/CEditView.cpp`), which stores the block with
    /// `COpeBuf::AppendOpeBlk` (the pointer goes up by one). Neither calls
    /// `CDocEditor::SetModified`. The modified flag (`m_bIsDocModified`) is set by the edit
    /// commands, cleared by a save, and set again by undo and redo as `m_nCurrentPointer !=
    /// m_nNoModifiedIndex` (`COpeBuf::DoUndo`/`DoRedo`, `cmd/COpeBuf.cpp`; used in
    /// `cmd/CViewCommander_Edit.cpp`). So a document that is unmodified stays so after the step;
    /// undoing the step is unmodified again, redoing it is modified (the pointer is not at the
    /// saved one), and an edit after it is modified. A document that is modified stays so. The
    /// marks of the modified lines are not touched. Does nothing outside a block.
    pub fn note_caret_step(&mut self) {
        if let Some(open) = &mut self.open {
            open.edited = true;
        }
    }

    /// Leaves the open block however deep it is, which stores it if it has an edit (Sakura:
    /// `CMacroBeforeAfter::ExecKeyMacroAfter` puts the reference count back to what it was before
    /// the macro, so a macro that called `AddRefUndoBuffer` more often than `SetUndoBuffer` does
    /// not keep the block open).
    pub fn close_all(&mut self, doc: &Document, cursor: &Cursor) {
        while self.open.is_some() {
            self.end(doc, cursor);
        }
    }

    /// Undoes one block: puts back the document and the cursor from before it. Returns `false`
    /// when there is nothing to undo.
    pub fn undo(&mut self, doc: &mut Document, cursor: &mut Cursor) -> bool {
        self.undo_lines(doc, cursor).is_some()
    }

    /// [`undo`](Self::undo) that also returns the changes of the line list that the undo makes,
    /// in the order to apply them. `None` when there is nothing to undo.
    pub fn undo_lines(
        &mut self,
        doc: &mut Document,
        cursor: &mut Cursor,
    ) -> Option<Vec<LineDelta>> {
        self.store_open_edits(doc, cursor);
        if self.pos == 0 {
            return None;
        }
        self.pos -= 1;
        self.caret_step_clean = false;
        for edit in self.blocks[self.pos].marks.iter().rev() {
            edit.revert(&mut self.marks);
        }
        let lines = self.blocks[self.pos]
            .lines
            .iter()
            .rev()
            .flat_map(LineEdit::inverse)
            .collect();
        self.restore(self.blocks[self.pos].before.clone(), doc, cursor);
        Some(lines)
    }

    /// Redoes one block: puts back the document and the cursor from after it.
    pub fn redo(&mut self, doc: &mut Document, cursor: &mut Cursor) -> bool {
        self.redo_lines(doc, cursor).is_some()
    }

    /// [`redo`](Self::redo) that also returns the changes of the line list, like
    /// [`undo_lines`](Self::undo_lines).
    pub fn redo_lines(
        &mut self,
        doc: &mut Document,
        cursor: &mut Cursor,
    ) -> Option<Vec<LineDelta>> {
        self.store_open_edits(doc, cursor);
        let block = self.blocks.get(self.pos)?;
        let after = block.after.clone();
        let lines = block.lines.iter().flat_map(LineEdit::forward).collect();
        for edit in &block.marks {
            edit.apply(&mut self.marks);
        }
        self.pos += 1;
        self.caret_step_clean = false;
        self.restore(after, doc, cursor);
        Some(lines)
    }

    /// Sakura closes the block of the running command before it undoes (`Command_UNDO` calls
    /// `SetUndoBuffer`), so that the edits so far are one step and the undo does not lose them.
    fn store_open_edits(&mut self, doc: &Document, cursor: &Cursor) {
        let Some(open) = &mut self.open else { return };
        if !open.edited {
            return;
        }
        let before = std::mem::replace(&mut open.before, Snapshot::of(doc, cursor));
        open.edited = false;
        let lines = std::mem::take(&mut open.lines);
        let marks = std::mem::take(&mut open.marks);
        let caret_only = !std::mem::replace(&mut open.real, false);
        self.push(before, Snapshot::of(doc, cursor), lines, marks, caret_only);
    }

    fn restore(&mut self, snap: Snapshot, doc: &mut Document, cursor: &mut Cursor) {
        *doc = snap.doc;
        cursor.restore(snap.cursor);
        cursor.set_goal_col(None);
        // The rest of the running command starts from this version.
        if let Some(open) = &mut self.open {
            open.before = Snapshot::of(doc, cursor);
        }
    }

    fn push(
        &mut self,
        before: Snapshot,
        after: Snapshot,
        lines: Vec<LineEdit>,
        marks: Vec<MarkEdit>,
        caret_only: bool,
    ) {
        // A caret step on a document that is unmodified leaves it so; any other step is an edit.
        self.caret_step_clean = caret_only && !self.is_stored_modified();
        self.blocks.truncate(self.pos);
        // The saved version was in the redo tail that is gone now.
        if self.saved.is_some_and(|s| s > self.pos) {
            self.saved = None;
        }
        self.blocks.push_back(Block {
            before,
            after,
            lines,
            marks,
        });
        self.pos += 1;
        self.trim();
    }

    fn trim(&mut self) {
        let Some(max) = self.max_blocks else { return };
        while self.blocks.len() > max {
            if self.pos == 0 {
                // Nothing applied is left to forget: drop what could be redone instead, so that
                // every kept block still starts from the version before it.
                self.blocks.pop_back();
                self.saved = self.saved.filter(|&s| s <= self.blocks.len());
            } else {
                self.blocks.pop_front();
                self.pos -= 1;
                self.origin += 1;
                // A saved version that is forgotten can never be reached again.
                self.saved = self.saved.and_then(|s| s.checked_sub(1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Pos;

    /// A document and a cursor with a history, edited by whole-line appends.
    struct Fixture {
        doc: Document,
        cursor: Cursor,
        history: History,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                doc: Document::empty(),
                cursor: Cursor::default(),
                history: History::new(),
            }
        }

        fn type_text(&mut self, text: &str) {
            self.history.begin(&self.doc, &self.cursor);
            self.history.note_edit();
            let edit = self.doc.insert(self.cursor.caret(), text).unwrap();
            self.cursor.move_to(edit.end, false);
            self.history.end(&self.doc, &self.cursor);
        }

        fn text(&self) -> String {
            self.doc
                .text_between(Pos::new(0, 0), self.doc.end_pos().unwrap())
                .unwrap()
        }

        fn undo(&mut self) -> bool {
            self.history.undo(&mut self.doc, &mut self.cursor)
        }

        fn redo(&mut self) -> bool {
            self.history.redo(&mut self.doc, &mut self.cursor)
        }
    }

    #[test]
    fn undo_and_redo_restore_text_and_cursor() {
        let mut f = Fixture::new();
        f.type_text("ab");
        f.type_text("cd");
        assert_eq!(f.text(), "abcd");
        assert!(f.undo());
        assert_eq!(
            (f.text().as_str(), f.cursor.caret()),
            ("ab", Pos::new(0, 2))
        );
        assert!(f.undo());
        assert_eq!((f.text().as_str(), f.cursor.caret()), ("", Pos::new(0, 0)));
        assert!(!f.undo());
        assert!(f.redo());
        assert!(f.redo());
        assert_eq!(
            (f.text().as_str(), f.cursor.caret()),
            ("abcd", Pos::new(0, 4))
        );
        assert!(!f.redo());
    }

    #[test]
    fn a_new_edit_drops_the_redo_tail() {
        let mut f = Fixture::new();
        f.type_text("a");
        f.type_text("b");
        f.undo();
        assert!(f.history.can_redo());
        f.type_text("c");
        assert!(!f.history.can_redo());
        assert_eq!(f.text(), "ac");
    }

    #[test]
    fn a_selection_comes_back_with_the_text() {
        let mut f = Fixture::new();
        f.type_text("abcd");
        f.cursor.select(Pos::new(0, 1), Pos::new(0, 3));
        f.history.begin(&f.doc, &f.cursor);
        f.history.note_edit();
        f.doc.delete(Pos::new(0, 1), Pos::new(0, 3)).unwrap();
        f.cursor.move_to(Pos::new(0, 1), false);
        f.history.end(&f.doc, &f.cursor);
        assert_eq!(f.text(), "ad");
        f.undo();
        assert_eq!(f.text(), "abcd");
        assert_eq!(f.cursor.selection(), Some((Pos::new(0, 1), Pos::new(0, 3))));
    }

    #[test]
    fn nested_blocks_are_one_step() {
        let mut f = Fixture::new();
        f.history.begin(&f.doc, &f.cursor);
        f.type_text("a");
        f.type_text("b");
        f.history.end(&f.doc, &f.cursor);
        assert_eq!(f.text(), "ab");
        f.undo();
        assert_eq!(f.text(), "");
        assert!(!f.undo());
    }

    #[test]
    fn a_block_without_edits_is_not_stored() {
        let mut f = Fixture::new();
        f.history.begin(&f.doc, &f.cursor);
        f.history.end(&f.doc, &f.cursor);
        assert!(!f.history.can_undo());
    }

    #[test]
    fn undo_inside_a_block_keeps_the_edits_made_before_it() {
        let mut f = Fixture::new();
        f.type_text("a");
        // a macro: type "b", then undo, then type "c", all in one block
        f.history.begin(&f.doc, &f.cursor);
        f.history.note_edit();
        let e = f.doc.insert(f.cursor.caret(), "b").unwrap();
        f.cursor.move_to(e.end, false);
        assert!(f.undo()); // stores "b" as a step, then undoes it
        assert_eq!(f.text(), "a");
        f.history.note_edit();
        let e = f.doc.insert(f.cursor.caret(), "c").unwrap();
        f.cursor.move_to(e.end, false);
        f.history.end(&f.doc, &f.cursor);
        assert_eq!(f.text(), "ac");
        f.undo();
        assert_eq!(f.text(), "a");
        f.undo();
        assert_eq!(f.text(), "");
    }

    #[test]
    fn modified_flag_follows_the_saved_version() {
        let mut f = Fixture::new();
        assert!(!f.history.is_modified());
        f.type_text("a");
        assert!(f.history.is_modified());
        f.history.mark_saved(&f.doc, &f.cursor);
        assert!(!f.history.is_modified());
        f.type_text("b");
        assert!(f.history.is_modified());
        f.undo();
        assert!(!f.history.is_modified());
        f.undo();
        assert!(f.history.is_modified());
        f.redo();
        assert!(!f.history.is_modified());
        // an edit while the saved version is in the redo tail: it can never be reached again
        f.undo();
        f.type_text("z");
        f.redo();
        f.undo();
        f.undo();
        assert!(f.history.is_modified());
    }

    #[test]
    fn saving_inside_a_running_block() {
        let mut f = Fixture::new();
        f.history.begin(&f.doc, &f.cursor);
        f.history.note_edit();
        let e = f.doc.insert(f.cursor.caret(), "a").unwrap();
        f.cursor.move_to(e.end, false);
        f.history.mark_saved(&f.doc, &f.cursor);
        assert!(!f.history.is_modified());
        // an edit after the save, in the same block, is a step of its own
        f.history.note_edit();
        let e = f.doc.insert(f.cursor.caret(), "b").unwrap();
        f.cursor.move_to(e.end, false);
        f.history.end(&f.doc, &f.cursor);
        assert!(f.history.is_modified());
        f.undo();
        assert_eq!(f.text(), "a");
        assert!(!f.history.is_modified());
        f.undo();
        assert!(f.history.is_modified());
    }

    #[test]
    fn the_oldest_blocks_are_forgotten() {
        let mut f = Fixture::new();
        f.history.set_max_blocks(Some(2));
        f.type_text("a");
        f.type_text("b");
        f.type_text("c");
        assert!(f.undo());
        assert!(f.undo());
        assert!(!f.undo());
        assert_eq!(f.text(), "a");
        // the version that was saved (the empty document) is gone, so the document counts as modified
        assert!(f.history.is_modified());
    }

    #[test]
    fn lowering_the_limit_after_undo_keeps_the_blocks_consistent() {
        let mut f = Fixture::new();
        for t in ["a", "b", "c"] {
            f.type_text(t);
        }
        while f.undo() {}
        f.history.set_max_blocks(Some(2));
        assert!(f.redo());
        assert!(f.redo());
        assert!(!f.redo());
        assert_eq!(f.text(), "ab");
        assert!(f.undo());
        assert!(f.undo());
        assert_eq!(f.text(), "");
    }

    #[test]
    fn an_edit_outside_a_block_clears_the_history() {
        let mut f = Fixture::new();
        f.type_text("a");
        f.history.note_edit();
        assert!(!f.history.can_undo());
        assert!(f.history.is_modified());
    }
}
