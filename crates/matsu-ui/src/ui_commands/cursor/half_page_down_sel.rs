//! `F_HalfPageDown_Sel`: `Command_HalfPageDown` with the selection extended.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HalfPageDownSel;

impl UiCommand for HalfPageDownSel {
    const ID: &'static str = "half_page_down_sel";
    const SAKURA: &'static str = "F_HalfPageDown_Sel";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.half_page_by(1, EditorView::rows_arg(args), true, cx)
    }
}

register_ui_command!(HalfPageDownSel);
