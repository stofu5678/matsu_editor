//! `F_TAB_JOINTPREV`: moves the tab to the previous window (前のグループに移動).
//!
//! Sakura: `CViewCommander::Command_TAB_JOINTPREV` calls `CTabWnd::JoinPrev`
//! (`sakura_core/window/CTabWnd.cpp`); see `F_TAB_JOINTNEXT`.

use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabJointprev;

impl UiCommand for TabJointprev {
    const ID: &'static str = "tab_jointprev";
    const SAKURA: &'static str = "F_TAB_JOINTPREV";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.defer_in_window(window, cx, |window, app| {
            flows::join_tab(app, window, false);
        })
    }
}

register_ui_command!(TabJointprev);
