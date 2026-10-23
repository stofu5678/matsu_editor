//! `F_CASCADE`: overlaps the windows (重ねて表示).
//!
//! Sakura: `Command_CASCADE` (`sakura_core/cmd/CViewCommander_Window.cpp`). Minimized windows are
//! left alone; the others are four fifths of the work area of this window's screen, each one
//! title bar lower and to the right of the one before, this window last and on top.
//! The arithmetic is `matsu_platform::window::arrange::cascade`. Moving a window needs the OS
//! (`matsu_platform::window`): Wayland has no way to do it, so Linux reports that it cannot.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{cascade_windows, unavailable, with_screen};
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Cascade;

impl UiCommand for Cascade {
    const ID: &'static str = "cascade";
    const SAKURA: &'static str = "F_CASCADE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        with_screen(window, cx, |screen, all, current| {
            cascade_windows(screen, all, current)
        })
        .map_err(unavailable)
    }
}

register_ui_command!(Cascade);
