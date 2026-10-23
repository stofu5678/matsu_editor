//! `F_MAXIMIZE_H`: makes the window as wide as the screen (横方向に最大化).
//!
//! Sakura: `Command_MAXIMIZE_H` (`sakura_core/cmd/CViewCommander_Window.cpp`) keeps the top edge
//! and the height and takes the left and the width of the work area. Moving a window needs the
//! OS (`matsu_platform::window`): Linux reports that it cannot.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{maximize_window, unavailable, with_screen};
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct MaximizeH;

impl UiCommand for MaximizeH {
    const ID: &'static str = "maximize_h";
    const SAKURA: &'static str = "F_MAXIMIZE_H";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        with_screen(window, cx, |screen, _, current| {
            maximize_window(screen, current, false)
        })
        .map_err(unavailable)
    }
}

register_ui_command!(MaximizeH);
