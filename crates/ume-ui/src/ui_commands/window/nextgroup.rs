//! `F_NEXTGROUP`: brings the next group (window) to the front (次のグループ).
//!
//! Sakura: `CViewCommander::Command_NEXTGROUP` calls `CTabWnd::NextGroup`
//! (`sakura_core/window/CTabWnd.cpp`), which activates the first window of the next group, the
//! first group after the last; `GetNextGroupWnd` finds none unless the tab bar is shown and the
//! windows are grouped. A group is a window here (ADR-0015), in the order they were opened.

use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Nextgroup;

impl UiCommand for Nextgroup {
    const ID: &'static str = "nextgroup";
    const SAKURA: &'static str = "F_NEXTGROUP";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        flows::go_to_group(cx, window.window_handle(), true);
        Ok(())
    }
}

register_ui_command!(Nextgroup);
