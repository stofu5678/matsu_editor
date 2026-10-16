//! `F_HalfPageUp`: `Command_HalfPageUp`: the caret goes up half the rows of the view.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct HalfPageUp;

impl UiCommand for HalfPageUp {
    const ID: &'static str = "half_page_up";
    const SAKURA: &'static str = "F_HalfPageUp";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.half_page_by(-1, false, cx)
    }
}

register_ui_command!(HalfPageUp);
