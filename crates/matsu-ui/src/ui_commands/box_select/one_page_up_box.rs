//! `F_1PageUp_BOX`: `Command_1PageUp` with the box selection extended.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) runs
//! `Sub_BoxSelectLock` (begins a box if there is none) and then `Command_1PageUp` with `bSelect` set
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`); the rules are in `view/paging.rs`. The macro
//! argument for the number of rows is read (`HalfPageUp_BoxSel(n)`); the lock option is not
//! supported.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnePageUpBox;

impl UiCommand for OnePageUpBox {
    const ID: &'static str = "one_page_up_box";
    const SAKURA: &'static str = "F_1PageUp_BOX";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.page_by_box(-1, EditorView::rows_arg(args), cx)
    }
}

register_ui_command!(OnePageUpBox);
