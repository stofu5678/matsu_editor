//! `F_1PageUp`: `Command_1PageUp`: the caret and the view go up by a page less one row.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnePageUp;

impl UiCommand for OnePageUp {
    const ID: &'static str = "one_page_up";
    const SAKURA: &'static str = "F_1PageUp";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.page_by(-1, false, cx)
    }
}

register_ui_command!(OnePageUp);
