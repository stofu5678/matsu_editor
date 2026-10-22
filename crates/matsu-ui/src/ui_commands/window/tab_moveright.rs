//! `F_TAB_MOVERIGHT`: moves the tab one place to the right (タブを右に移動).
//!
//! Sakura: `CViewCommander::Command_TAB_MOVERIGHT` calls `CTabWnd::MoveRight`
//! (`sakura_core/window/CTabWnd.cpp`): the tab swaps with its right neighbour, and nothing
//! happens at the right end. Only the tabs of a window are ordered here: with the windows not
//! grouped the windows have no order to change.

use crate::tabs::{order, settings};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabMoveright;

impl UiCommand for TabMoveright {
    const ID: &'static str = "tab_moveright";
    const SAKURA: &'static str = "F_TAB_MOVERIGHT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if !settings::current(cx).bound() {
            return Ok(());
        }
        view.with_workspace(window, cx, |ws, _, cx| {
            let at = ws.active_tab();
            if let Some(to) = order::right_of(at, ws.tab_count()) {
                ws.move_tab(at, to, cx);
            }
        })
    }
}

register_ui_command!(TabMoveright);
