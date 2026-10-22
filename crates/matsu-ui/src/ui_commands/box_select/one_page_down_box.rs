//! `F_1PageDown_BOX`: `Command_1PageDown` with the box selection extended.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_1PageDown` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`); the rules are in `view/paging.rs`. The macro
//! arguments (the number of rows and the lock option) are not supported, as for the other page
//! commands.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnePageDownBox;

impl UiCommand for OnePageDownBox {
    const ID: &'static str = "one_page_down_box";
    const SAKURA: &'static str = "F_1PageDown_BOX";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.page_by_box(1, cx)
    }
}

register_ui_command!(OnePageDownBox);
