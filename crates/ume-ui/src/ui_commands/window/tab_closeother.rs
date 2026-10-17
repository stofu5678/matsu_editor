//! `F_TAB_CLOSEOTHER`: closes the other windows (このタブ／ウィンドウ以外を閉じる).
//!
//! Sakura: `CViewCommander::Command_TAB_CLOSEOTHER` (`sakura_core/cmd/CViewCommander_Window.cpp`)
//! asks every window of the group except this one to close with `RequestCloseEditor`, so each
//! window with changes asks "save?" and "cancel" stops it. Without tabs all windows are in one
//! group. The window that runs the command stays. In tab mode the group is this window: the
//! other tabs close and this one stays (`crate::tabs::flows::group_views`).

use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::close_windows;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabCloseother;

impl UiCommand for TabCloseother {
    const ID: &'static str = "tab_closeother";
    const SAKURA: &'static str = "F_TAB_CLOSEOTHER";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let current = window.window_handle();
        start(window, cx, async move |_view, cx| {
            close_windows(cx, current, true).await;
        });
        Ok(())
    }
}

register_ui_command!(TabCloseother);
