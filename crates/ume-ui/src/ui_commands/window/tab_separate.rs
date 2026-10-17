//! `F_TAB_SEPARATE`: moves the tab to a new window (新規グループ).
//!
//! Sakura: `CViewCommander::Command_TAB_SEPARATE` calls `CTabWnd::Separate`
//! (`sakura_core/window/CTabWnd.cpp`), which takes the window out of its group into a new group
//! and shows it a little lower and to the right. A group is a window here (ADR-0015): the tab
//! moves to a new window with its panes, caret and undo history. Only in tab mode, and not for
//! the only tab of a window.

use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabSeparate;

impl UiCommand for TabSeparate {
    const ID: &'static str = "tab_separate";
    const SAKURA: &'static str = "F_TAB_SEPARATE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.defer_in_window(window, cx, |window, app| {
            flows::separate_tab(app, window);
        })
    }
}

register_ui_command!(TabSeparate);
