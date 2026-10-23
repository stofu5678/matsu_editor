//! `F_GROUPCLOSE`: closes every tab of this window (グループを閉じる).
//!
//! Sakura: `CViewCommander::Command_GROUPCLOSE` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls
//! `CControlTray::CloseAllEditor` for the group when the tab bar is shown and the windows are
//! grouped; each document with changes asks "save?" and "cancel" stops it. A group is a window
//! here (ADR-0015). In any other mode it does nothing.

use crate::file_ops::start;
use crate::tabs::settings;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::close_windows;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Groupclose;

impl UiCommand for Groupclose {
    const ID: &'static str = "groupclose";
    const SAKURA: &'static str = "F_GROUPCLOSE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if !settings::current(cx).bound() {
            return Ok(());
        }
        let current = window.window_handle();
        start(window, cx, async move |_view, cx| {
            close_windows(cx, current, false).await;
        });
        Ok(())
    }
}

register_ui_command!(Groupclose);
