//! `F_TAB_MOVELEFT`: moves the tab one place to the left (タブを左に移動).
//!
//! Sakura: `CViewCommander::Command_TAB_MOVELEFT` calls `CTabWnd::MoveLeft`
//! (`sakura_core/window/CTabWnd.cpp`); see `F_TAB_MOVERIGHT`.

use crate::tabs::{order, settings};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabMoveleft;

impl UiCommand for TabMoveleft {
    const ID: &'static str = "tab_moveleft";
    const SAKURA: &'static str = "F_TAB_MOVELEFT";

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
            if let Some(to) = order::left_of(at) {
                ws.move_tab(at, to, cx);
            }
        })
    }
}

register_ui_command!(TabMoveleft);
