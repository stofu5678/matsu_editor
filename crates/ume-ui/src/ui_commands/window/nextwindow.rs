//! `F_NEXTWINDOW`: brings the next editor window to the front (次のウィンドウ).
//!
//! Sakura: `CEditDoc::HandleCommand` (`sakura_core/doc/CEditDoc.cpp`) first moves to the next pane
//! of a split window and, when there is none, calls `CControlTray::ActivePrevWindow`
//! (`sakura_core/_main/CControlTray.cpp`), which activates the window after this one in the list
//! of editor windows (in the order they were opened), and the first after the last. Split panes
//! are not there yet.

use crate::file_ops::windows::activate;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{all_handles, window_after};
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_platform::window::arrange::Direction;

#[derive(Default)]
pub struct Nextwindow;

impl UiCommand for Nextwindow {
    const ID: &'static str = "nextwindow";
    const SAKURA: &'static str = "F_NEXTWINDOW";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let current = window.window_handle();
        let all = all_handles(cx);
        if let Some(target) = window_after(&all, current, Direction::Next) {
            if target == current {
                window.activate_window();
            } else {
                activate(cx, target);
            }
        }
        Ok(())
    }
}

register_ui_command!(Nextwindow);
