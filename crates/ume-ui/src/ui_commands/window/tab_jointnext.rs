//! `F_TAB_JOINTNEXT`: moves the tab to the next window (次のグループに移動).
//!
//! Sakura: `CViewCommander::Command_TAB_JOINTNEXT` calls `CTabWnd::JoinNext`
//! (`sakura_core/window/CTabWnd.cpp`): the window joins the next group. The tab becomes the
//! active tab of the next window, which comes to the front; a window left without tabs closes.

use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabJointnext;

impl UiCommand for TabJointnext {
    const ID: &'static str = "tab_jointnext";
    const SAKURA: &'static str = "F_TAB_JOINTNEXT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.defer_in_window(window, cx, |window, app| {
            flows::join_tab(app, window, true);
        })
    }
}

register_ui_command!(TabJointnext);
