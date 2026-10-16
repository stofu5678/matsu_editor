//! `F_WndScrollDown`: `Command_WndScrollDown`: the text goes one row down.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct WndScrollDown;

impl UiCommand for WndScrollDown {
    const ID: &'static str = "wnd_scroll_down";
    const SAKURA: &'static str = "F_WndScrollDown";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.scroll_window_by(-1, cx)
    }
}

register_ui_command!(WndScrollDown);
