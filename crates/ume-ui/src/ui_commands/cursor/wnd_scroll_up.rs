//! `F_WndScrollUp`: `Command_WndScrollUp`: the text goes one row up.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct WndScrollUp;

impl UiCommand for WndScrollUp {
    const ID: &'static str = "wnd_scroll_up";
    const SAKURA: &'static str = "F_WndScrollUp";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.scroll_window_by(1, cx)
    }
}

register_ui_command!(WndScrollUp);
