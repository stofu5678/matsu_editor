//! `F_BIND_WINDOW`: gathers the windows into tabs, or spreads them again (グループ化).
//!
//! Sakura: `CViewCommander::Command_BIND_WINDOW` (`sakura_core/cmd/CViewCommander_Window.cpp`) flips
//! `m_bDispTabWndMultiWin` ("タブをまとめない") when the tab bar is shown, resets the group ids and
//! tells every window. Nothing happens while the tab bar is hidden.

use crate::tabs::{flows, settings};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct BindWindow;

impl UiCommand for BindWindow {
    const ID: &'static str = "bind_window";
    const SAKURA: &'static str = "F_BIND_WINDOW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.defer_in_window(window, cx, |window, app| {
            if settings::current(app).show_bar {
                flows::change_settings(app, window, |s| s.grouped = !s.grouped);
            }
        })
    }
}

register_ui_command!(BindWindow);
