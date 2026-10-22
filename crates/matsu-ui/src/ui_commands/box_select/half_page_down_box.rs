//! `F_HalfPageDown_BOX`: `Command_HalfPageDown` with the box selection extended.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_HalfPageDown` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`); the rules are in `view/paging.rs`. The macro
//! arguments (the number of rows and the lock option) are not supported, as for the other page
//! commands.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HalfPageDownBox;

impl UiCommand for HalfPageDownBox {
    const ID: &'static str = "half_page_down_box";
    const SAKURA: &'static str = "F_HalfPageDown_BOX";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.half_page_by_box(1, cx)
    }
}

register_ui_command!(HalfPageDownBox);
