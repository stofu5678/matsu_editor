//! `F_MINIMIZE_ALL`: minimizes every editor window (すべて最小化).
//!
//! Sakura: `Command_MINIMIZE_ALL` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls
//! `ShowWindow(SW_MINIMIZE)` on each visible editor window. GPUI can minimize a window on every
//! OS, so this needs nothing of `matsu_platform`.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{minimize_windows, unavailable, with_screen};
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct MinimizeAll;

impl UiCommand for MinimizeAll {
    const ID: &'static str = "minimize_all";
    const SAKURA: &'static str = "F_MINIMIZE_ALL";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        with_screen(window, cx, |screen, all, _| {
            minimize_windows(screen, all);
            Ok(())
        })
        .map_err(unavailable)
    }
}

register_ui_command!(MinimizeAll);
