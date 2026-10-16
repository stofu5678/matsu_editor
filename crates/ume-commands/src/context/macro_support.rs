//! What the macro-only functions (`F_MOVECURSOR`, `F_SET_QUOTESTRING`, `F_COMMITUNDOBUFFER` and
//! the others, see `crate::commands::macro_only`) need from the context: moving the caret by a
//! line and a column the way a macro names them, the undo block of a macro, the quote string and
//! the clipboard.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp`, `CViewCommander::
//! Command_MOVECURSOR` and `Command_MOVECURSORLAYOUT` in `sakura_core/cmd/CViewCommander_Cursor.cpp`,
//! `Command_SET_QUOTESTRING` in `sakura_core/cmd/CViewCommander_Settings.cpp`.

use ume_core::buffer::Pos;

use super::CommandContext;
use crate::command::{CommandError, Result};

/// The bits of the option of `MoveCursor` and `MoveCursorLayout`
/// (`help/macro/source/reference/cursor/S_MoveCursor.html`).
pub(crate) mod move_option {
    /// Select from the old position.
    pub const SELECT: i64 = 0x01;
    /// Select a box.
    pub const BOX: i64 = 0x02;
}

/// `m_szInyouKigou` is `StaticString<32>` (`CommonSetting_Format` in `sakura_core/env/CommonSetting.h`):
/// 31 UTF-16 units and the terminator. `wcsncpy_s(.., _TRUNCATE)` keeps as many as fit.
const QUOTE_UNITS: usize = 31;

/// The byte offset in `line` of the UTF-16 unit `units` (counted from 0). A unit in the middle of
/// a surrogate pair goes to the start of the character; one past the end goes to the end.
fn byte_offset_for_utf16(line: &str, units: usize) -> usize {
    let mut seen = 0;
    for (offset, ch) in line.char_indices() {
        let width = ch.len_utf16();
        if seen + width > units {
            return offset;
        }
        seen += width;
    }
    line.len()
}

impl CommandContext<'_> {
    // ---- moving by line and column ----

    /// `Command_MOVECURSOR`: moves the caret to a logical position, `line` and `unit` counted from
    /// 0 (the macro passes the numbers minus 1). `unit` counts UTF-16 units, Sakura's `wchar_t`.
    /// A line past the last one is the end of the text, a unit past the end of the line is the end
    /// of the line (`CCaret::GetAdjustCursorPos`, `LogicToLayout`). A negative number beeps and
    /// does nothing. `option` is 0 (no selection), 1 (select) or 2 (select a box).
    pub fn move_cursor_logical(&mut self, line: i64, unit: i64, option: i64) -> Result<()> {
        let (Ok(line), Ok(unit)) = (usize::try_from(line), usize::try_from(unit)) else {
            self.beep();
            return Ok(());
        };
        let pos = if line >= self.doc().line_count() {
            self.doc().end_pos()?
        } else {
            let text = self.doc().line(line)?.map(|l| l.text).unwrap_or_default();
            Pos::new(line, byte_offset_for_utf16(&text, unit))
        };
        let row_start = self.layout.row_start(self.doc(), pos)?;
        let col = self.layout.display_col(self.doc(), pos)?;
        self.move_macro_caret(row_start, pos, col, option)
    }

    /// `Command_MOVECURSORLAYOUT`: moves the caret to a row and a display column of the layout,
    /// both counted from 0. A row past the last one is the end of the text. The column is
    /// counted in cells of the layout (a tab and a full-width character take more than one). In
    /// a box selection the caret can be in the empty space right of the text; otherwise it stops
    /// at the end of the row. A negative number beeps and does nothing.
    pub fn move_cursor_layout(&mut self, row: i64, col: i64, option: i64) -> Result<()> {
        let (Ok(row), Ok(col)) = (usize::try_from(row), usize::try_from(col)) else {
            self.beep();
            return Ok(());
        };
        let (row_start, pos, col) = match self.layout.row_start_at(self.doc(), row)? {
            Some(start) => (start, self.layout.pos_at_col(self.doc(), start, col)?, col),
            None => {
                // past the last row: the end of the text, and its column for a box
                let end = self.doc().end_pos()?;
                let at = self.layout.display_col(self.doc(), end)?;
                (self.layout.row_start(self.doc(), end)?, end, at)
            }
        };
        self.move_macro_caret(row_start, pos, col, option)
    }

    /// The part that `Command_MOVECURSORLAYOUT` shares: the selection first, then the move.
    /// `row_start` is the first position of the target row, `pos` the target and `col` the display
    /// column that a box caret goes to.
    fn move_macro_caret(
        &mut self,
        row_start: Pos,
        pos: Pos,
        col: usize,
        option: i64,
    ) -> Result<()> {
        let select = option & move_option::SELECT != 0;
        let boxed = option & move_option::BOX != 0;
        if select || boxed {
            // `Command_BEGIN_BOXSELECT` when there is nothing selected yet, and when a plain
            // selection changes into a box (2014.01.08 in Sakura). A plain selection begins at
            // the caret with the move below (`BeginSelectArea`).
            if boxed && (!self.cursor.is_box() || !self.cursor.has_selection()) {
                self.begin_box()?;
            }
        } else {
            // `DisableSelectArea`, or `SetBoxSelect(false)` for a box that selects nothing yet.
            self.cursor.clear_selection();
        }
        if (select || boxed) && self.cursor.is_box() {
            return self.box_move_to_col(row_start, col);
        }
        self.move_caret(pos, select)
    }

    // ---- undo blocks of a macro ----

    /// `F_COMMITUNDOBUFFER`: stores what the open undo block has so far as an undo step and keeps
    /// the block open, so that the rest of the macro is a step of its own. The block is the one
    /// that [`begin_block`](Self::begin_block) opened; see the module documentation of
    /// `ume_core::undo`.
    pub fn commit_undo_block(&mut self) {
        self.model.commit_block(self.cursor);
    }

    /// `F_APPENDUNDOBUFFERCURSOR`: makes the open undo block a step that undo goes back over, even
    /// when the macro changed nothing. The caret is the one of the block's end.
    pub fn add_caret_step(&mut self) {
        self.model.note_caret_step();
    }

    /// How deep the open undo block is entered (0: none).
    pub fn undo_depth(&self) -> usize {
        self.model.block_depth()
    }

    /// Leaves every undo block that is still open (the end of a macro).
    pub fn close_undo_blocks(&mut self) {
        self.model.close_blocks(self.cursor);
    }

    // ---- the quote string ----

    /// The quote string of `F_COPYLINESASPASSAGE` (Sakura: `m_szInyouKigou`, `"> "` by default).
    pub fn quote_string(&self) -> &str {
        &self.quote
    }

    /// `F_SET_QUOTESTRING`: sets the quote string. Only the first 31 UTF-16 units are kept. The
    /// editor reads [`Changes::quote_string`](super::Changes::quote_string) to keep the value for
    /// the next commands (Sakura keeps it in the shared settings, so every window sees it).
    pub fn set_quote_string(&mut self, quote: &str) {
        let mut kept = String::new();
        let mut units = 0;
        for c in quote.chars() {
            units += c.len_utf16();
            if units > QUOTE_UNITS {
                break;
            }
            kept.push(c);
        }
        self.quote.clone_from(&kept);
        self.changes.quote_string = Some(kept);
    }

    // ---- the clipboard ----

    /// `F_CLIPBOARDEMPTY`: empties the clipboard (`CClipboard::Empty`). Fails if this context has
    /// no clipboard.
    pub fn clipboard_clear(&mut self) -> Result<()> {
        match self.clipboard.as_deref_mut() {
            Some(clipboard) => {
                clipboard.clear();
                Ok(())
            }
            None => Err(CommandError::Unavailable("the clipboard")),
        }
    }
}
