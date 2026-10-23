//! `F_DIFF_NEXT`: moves to the next line with a DIFF mark (次の差分へ).
//!
//! Sakura: `CViewCommander::Command_Diff_Next` (`sakura_core/cmd/CViewCommander_Diff.cpp`). It is a
//! plain move (it extends the selection while the selection is locked). The rules are in
//! `EditorView::jump_to_diff`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct DiffNext;

impl UiCommand for DiffNext {
    const ID: &'static str = "diff_next";
    const SAKURA: &'static str = "F_DIFF_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_diff(Direction::Forward, window, cx);
        Ok(())
    }
}

register_ui_command!(DiffNext);
