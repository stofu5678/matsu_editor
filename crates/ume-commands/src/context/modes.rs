//! The functions of Sakura's mode group (モード切り替え系): the selection lock, cancelling the
//! modes, and the line ending that Enter inserts.

use ume_core::buffer::Eol;

use super::CommandContext;
use crate::command::Result;

/// The line ending for the value of Sakura's `EEolType` (`sakura_core/basis/CEol.h`): 1 CRLF, 2 LF,
/// 3 CR, 4 NEL, 5 LS, 6 PS. `None` for 0 (no line ending) and for values out of range, which
/// `CEol::IsValid` refuses.
pub fn eol_of_type(value: i64) -> Option<Eol> {
    match value {
        1 => Some(Eol::CrLf),
        2 => Some(Eol::Lf),
        3 => Some(Eol::Cr),
        4 => Some(Eol::Nel),
        5 => Some(Eol::Ls),
        6 => Some(Eol::Ps),
        _ => None,
    }
}

impl CommandContext<'_> {
    /// `F_BEGIN_SEL`: switches the selection lock on or off. While it is on, the plain moves
    /// extend the selection from where the caret was.
    ///
    /// Sakura: `CViewCommander::Command_BEGIN_SELECT` in `sakura_core/cmd/CViewCommander_Select.cpp`.
    /// With no text selected it starts the selection at the caret (`BeginSelectArea`); the cursor
    /// here starts it with the first move that extends. Then it flips `m_bSelectingLock`, also
    /// when a selection is there.
    pub fn toggle_selecting_lock(&mut self) {
        let lock = self.cursor.selecting_lock();
        self.cursor.set_selecting_lock(!lock);
    }

    /// `F_CANCEL_MODE`: drops the selection (also a box that covers nothing) and the selection
    /// lock. The caret stays where it is.
    ///
    /// Sakura: `CViewCommander::Command_CANCEL_MODE` in
    /// `sakura_core/cmd/CViewCommander_ModeChange.cpp`, with the default `whereCursorIs = 0`
    /// ("do not move"). The goal column becomes the column of the caret
    /// (`m_nCaretPosX_Prev`). With a box selection Sakura moves the caret back to the end of the
    /// text of its row if it was in the empty space to the right (unless free cursor mode is on);
    /// the caret position of a box here is that end already.
    pub fn cancel_mode(&mut self) -> Result<()> {
        if self.cursor.has_selection() {
            let caret = self.caret();
            self.cursor.clear_selection();
            self.move_caret(caret, false)
        } else {
            // a lock or a box that covers nothing still has to go (`IsTextSelecting`)
            if self.cursor.selecting_lock() || self.cursor.is_box() {
                self.cursor.clear_selection();
                self.cursor.set_goal_col(None);
            }
            Ok(())
        }
    }

    /// `F_CHGMOD_EOL`: sets the line ending that Enter inserts (`CDocEditor::SetNewLineCode`).
    pub fn set_new_line(&mut self, eol: Eol) {
        self.model.set_new_line(eol);
    }
}
