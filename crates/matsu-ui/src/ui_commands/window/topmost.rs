//! `F_TOPMOST`: keeps the window above the others (常に手前に表示).
//!
//! Sakura: `Command_WINTOPMOST` calls `CEditWnd::WindowTopMost` (`sakura_core/window/CEditWnd.cpp`).
//! The argument is 0 to switch (what the menu does), 1 to turn it on, 2 to turn it off; the
//! macro function `WindowTopMost(n)` passes it, and any other number does nothing. GPUI has no
//! always-on-top window, so `matsu_platform::window` sets the level of the window on Windows
//! (`HWND_TOPMOST`) and macOS (the floating window level); Linux reports that it cannot (a
//! Wayland compositor does not let a program decide that).

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{topmost, unavailable, with_screen};
use gpui::{Context, Window};
use matsu_commands::command::CommandError;
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Topmost;

impl UiCommand for Topmost {
    const ID: &'static str = "topmost";
    const SAKURA: &'static str = "F_TOPMOST";

    fn run(
        &self,
        _view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let how = match args.first() {
            None => 0,
            Some(Arg::Int(n)) => *n,
            Some(_) => {
                return Err(CommandError::BadArgs(
                    "the argument of topmost is a number (0, 1 or 2)".to_owned(),
                ));
            }
        };
        with_screen(window, cx, |screen, _, current| {
            topmost(screen, current, how)
        })
        .map_err(unavailable)
    }
}

register_ui_command!(Topmost);
