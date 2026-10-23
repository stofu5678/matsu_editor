//! `F_WINRESTORE` (the macro `WinRestore()`): brings the window back to its normal size.
//!
//! Sakura: `cmd::window::Command_WINRESTORE` (`sakura_core/cmd/CViewCommander_Window.cpp`) is
//! `ShowWindow(SW_RESTORE)` on the edit window, which restores a minimized and a maximized
//! window. GPUI cannot restore a minimized window, so Windows and macOS do it through
//! `matsu_platform::window::restore` (the user checks it: CI has no desktop). Where the platform
//! cannot (Linux), a maximized window is zoomed back with GPUI, and a minimized window stays
//! as it is.
//!
//! The call into the OS makes it call GPUI back (`WM_SIZE` and the like), which needs the
//! application the command holds, so it runs after the command, as the placing of windows does
//! (`crate::window_ops`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_platform::window;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::raw_handle;

#[derive(Default)]
pub struct WinRestore;

impl UiCommand for WinRestore {
    const ID: &'static str = "winrestore";
    const SAKURA: &'static str = "F_WINRESTORE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        win: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if window::unsupported().is_none()
            && let Some(raw) = raw_handle(win)
        {
            cx.spawn(async move |_, _| {
                // The command has ended; a refusal of the OS has nowhere to go.
                let _ = window::restore(raw);
            })
            .detach();
        } else {
            super::zoom_back(win);
        }
        Ok(())
    }
}

register_ui_command!(WinRestore);
