//! `F_TAB_CLOSERIGHT`: closes the tabs to the right of this one (右をすべて閉じる).
//!
//! Sakura: `CViewCommander::Command_TAB_CLOSERIGHT` (`sakura_core/cmd/CViewCommander_Window.cpp`);
//! see `F_TAB_CLOSELEFT`.

use crate::file_ops::start;
use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabCloseright;

impl UiCommand for TabCloseright {
    const ID: &'static str = "tab_closeright";
    const SAKURA: &'static str = "F_TAB_CLOSERIGHT";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let current = window.window_handle();
        start(window, cx, async move |_view, cx| {
            flows::close_side(cx, current, false).await;
        });
        Ok(())
    }
}

register_ui_command!(TabCloseright);
