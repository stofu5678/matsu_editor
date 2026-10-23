//! `F_MAXIMIZE_V`: makes the window as high as the screen (縦方向に最大化).
//!
//! Sakura: `Command_MAXIMIZE_V` (`sakura_core/cmd/CViewCommander_Window.cpp`) keeps the left edge
//! and the width and takes the top and the height of the work area. Moving a window needs the OS
//! (`matsu_platform::window`): Linux reports that it cannot.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{maximize_window, unavailable, with_screen};
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct MaximizeV;

impl UiCommand for MaximizeV {
    const ID: &'static str = "maximize_v";
    const SAKURA: &'static str = "F_MAXIMIZE_V";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        with_screen(window, cx, |screen, _, current| {
            maximize_window(screen, current, true)
        })
        .map_err(unavailable)
    }
}

register_ui_command!(MaximizeV);
