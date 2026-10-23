//! `F_WIN_CLOSEALL`: closes every window (すべて閉じる), asking about unsaved changes.
//!
//! Sakura: `CViewCommander::Command_FILECLOSEALL` (`sakura_core/cmd/CViewCommander_Window.cpp`)
//! calls `CControlTray::CloseAllEditor` and `CAppNodeGroupHandle::RequestCloseEditor`
//! (`sakura_core/env/CAppNodeManager.cpp`), which ask each window to close, one after the other.
//! A window with changes asks "save?"; "cancel" stops it and keeps the windows that are left.
//! Sakura's extra question "close several windows?" is asked only when the setting
//! `bCloseAllConfirm` is on (off by default); this program does not read that setting yet.
//! With no tray process, closing the last window ends the program.

use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::close_windows;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct WinCloseall;

impl UiCommand for WinCloseall {
    const ID: &'static str = "win_closeall";
    const SAKURA: &'static str = "F_WIN_CLOSEALL";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let current = window.window_handle();
        start(window, cx, async move |_view, cx| {
            close_windows(cx, current, false).await;
        });
        Ok(())
    }
}

register_ui_command!(WinCloseall);
