//! `F_PREVWINDOW`: brings the previous editor window to the front (前のウィンドウ).
//!
//! Sakura: `CEditDoc::HandleCommand` (`sakura_core/doc/CEditDoc.cpp`) first moves to the previous
//! pane of a split window and, when there is none, calls `CControlTray::ActiveNextWindow`
//! (`sakura_core/_main/CControlTray.cpp`), which activates the window before this one in the list
//! of editor windows (in the order they were opened), and the last before the first. Split panes
//! are not there yet.

use crate::file_ops::windows::activate;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{all_handles, window_after};
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_platform::window::arrange::Direction;

#[derive(Default)]
pub struct Prevwindow;

impl UiCommand for Prevwindow {
    const ID: &'static str = "prevwindow";
    const SAKURA: &'static str = "F_PREVWINDOW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // In tab mode the group is this window, and its windows are the tabs
        // (`ActiveNextWindow` / `ActivePrevWindow` walk the windows of the group).
        if crate::tabs::settings::current(cx).bound() {
            return view.with_workspace(window, cx, |ws, window, cx| {
                let to = crate::tabs::order::prev_tab(ws.active_tab(), ws.tab_count());
                ws.activate_tab(to, window, cx);
            });
        }
        let current = window.window_handle();
        let all = all_handles(cx);
        if let Some(target) = window_after(&all, current, Direction::Previous) {
            if target == current {
                window.activate_window();
            } else {
                activate(cx, target);
            }
        }
        Ok(())
    }
}

register_ui_command!(Prevwindow);
