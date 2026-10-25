//! `F_1PageUp_Sel`: `Command_1PageUp` with the selection extended.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnePageUpSel;

impl UiCommand for OnePageUpSel {
    const ID: &'static str = "one_page_up_sel";
    const SAKURA: &'static str = "F_1PageUp_Sel";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.page_by(-1, EditorView::rows_arg(args), true, cx)
    }
}

register_ui_command!(OnePageUpSel);
