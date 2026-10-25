//! `F_HalfPageDown`: `Command_HalfPageDown`: the caret goes down half the rows of the view.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HalfPageDown;

impl UiCommand for HalfPageDown {
    const ID: &'static str = "half_page_down";
    const SAKURA: &'static str = "F_HalfPageDown";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.half_page_by(1, EditorView::rows_arg(args), false, cx)
    }
}

register_ui_command!(HalfPageDown);
