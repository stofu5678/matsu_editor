//! `F_DIFF_PREV`: moves to the previous line with a DIFF mark (前の差分へ).
//!
//! Sakura: `CViewCommander::Command_Diff_Prev` (`sakura_core/cmd/CViewCommander_Diff.cpp`). See
//! `diff_next` for the rules.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct DiffPrev;

impl UiCommand for DiffPrev {
    const ID: &'static str = "diff_prev";
    const SAKURA: &'static str = "F_DIFF_PREV";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_diff(Direction::Backward, window, cx);
        Ok(())
    }
}

register_ui_command!(DiffPrev);
