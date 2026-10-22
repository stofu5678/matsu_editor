//! `F_MODIFYLINE_NEXT_SEL`: moves to the next modified line and extends the selection.
//!
//! Sakura: `CViewCommander::Command_MODIFYLINE_NEXT` (`sakura_core/cmd/CViewCommander_Cursor.cpp`),
//! macro `GoModifyLineNext_Sel`. The move is a plain move unless it extends (it also extends while the
//! selection is locked) and is not put in the jump history. The rules are in
//! `EditorView::jump_to_modified_line`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ModifyLineNextSel;

impl UiCommand for ModifyLineNextSel {
    const ID: &'static str = "modifyline_next_sel";
    const SAKURA: &'static str = "F_MODIFYLINE_NEXT_SEL";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_modified_line(Direction::Forward, true, window, cx);
        Ok(())
    }
}

register_ui_command!(ModifyLineNextSel);
