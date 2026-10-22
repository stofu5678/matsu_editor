//! `F_1PageDown_Sel`: `Command_1PageDown` with the selection extended.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnePageDownSel;

impl UiCommand for OnePageDownSel {
    const ID: &'static str = "one_page_down_sel";
    const SAKURA: &'static str = "F_1PageDown_Sel";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.page_by(1, true, cx)
    }
}

register_ui_command!(OnePageDownSel);
