//! `F_MODIFYLINE_NEXT`: moves to the next modified line.
//!
//! Sakura: `CViewCommander::Command_MODIFYLINE_NEXT` (`sakura_core/cmd/CViewCommander_Cursor.cpp`),
//! macro `GoModifyLineNext`. The move is a plain move unless it extends (it also extends while the
//! selection is locked) and is not put in the jump history. The rules are in
//! `EditorView::jump_to_modified_line`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ModifyLineNext;

impl UiCommand for ModifyLineNext {
    const ID: &'static str = "modifyline_next";
    const SAKURA: &'static str = "F_MODIFYLINE_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_modified_line(Direction::Forward, false, window, cx);
        Ok(())
    }
}

register_ui_command!(ModifyLineNext);
