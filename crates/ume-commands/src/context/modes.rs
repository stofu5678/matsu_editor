//! The functions of Sakura's mode group (モード切り替え系): the selection lock, cancelling the
//! modes, and the line ending that Enter inserts.

use ume_core::buffer::Eol;

use super::CommandContext;
use crate::command::Result;

/// The line ending for the number that the macro `ChgmodEOL(n)` takes: 1 CRLF, 3 LF, 4 CR, 5 NEL,
/// 6 LS, 7 PS. It is not Sakura's `EEolType` (1 CRLF, 2 LF, 3 CR, ...): `CMacro::HandleCommand`
/// converts it ("マクロ引数値をEOLタイプ値に変換する", `case F_CHGMOD_EOL` in
/// `sakura_core/macro/CMacro.cpp`). `None` for 0, 2 and everything else, which Sakura turns into
/// `EEolType::none` and ignores.
pub fn eol_of_macro_value(value: i64) -> Option<Eol> {
    match value {
        1 => Some(Eol::CrLf),
        3 => Some(Eol::Lf),
        4 => Some(Eol::Cr),
        5 => Some(Eol::Nel),
        6 => Some(Eol::Ls),
        7 => Some(Eol::Ps),
        _ => None,
    }
}

/// The inverse of [`eol_of_macro_value`]: the number that a key macro records for the line
/// ending (`case F_CHGMOD_EOL` in `CMacro::AddLParam`, `sakura_core/macro/CMacro.cpp`).
pub fn macro_value_of_eol(eol: Eol) -> i64 {
    match eol {
        Eol::CrLf => 1,
        Eol::Lf => 3,
        Eol::Cr => 4,
        Eol::Nel => 5,
        Eol::Ls => 6,
        Eol::Ps => 7,
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
