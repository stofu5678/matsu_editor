//! `F_MODIFYLINE_PREV_SEL`: moves to the previous modified line and extends the selection.
//!
//! Sakura: `CViewCommander::Command_MODIFYLINE_PREV` (`sakura_core/cmd/CViewCommander_Cursor.cpp`),
//! macro `GoModifyLinePrev_Sel`. The move is a plain move unless it extends (it also extends while the
//! selection is locked) and is not put in the jump history. The rules are in
//! `EditorView::jump_to_modified_line`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ModifyLinePrevSel;

impl UiCommand for ModifyLinePrevSel {
    const ID: &'static str = "modifyline_prev_sel";
    const SAKURA: &'static str = "F_MODIFYLINE_PREV_SEL";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_modified_line(Direction::Backward, true, window, cx);
        Ok(())
    }
}

register_ui_command!(ModifyLinePrevSel);
