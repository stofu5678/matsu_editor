//! `F_1PageDown`: `Command_1PageDown`: the caret and the view go down by a page less one row.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnePageDown;

impl UiCommand for OnePageDown {
    const ID: &'static str = "one_page_down";
    const SAKURA: &'static str = "F_1PageDown";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.page_by(1, EditorView::rows_arg(args), false, cx)
    }
}

register_ui_command!(OnePageDown);
