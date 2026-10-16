//! `F_TILE_H`: puts the windows side by side (左右に並べて表示).
//!
//! Sakura: `Command_TILE_H` (`sakura_core/cmd/CViewCommander_Window.cpp`). Minimized windows are
//! left alone; the others share the work area of this window's screen in equal widths, this
//! window at the left, and get the full height. This window gets the focus. Moving a window needs
//! the OS (`ume_platform::window`): Linux reports that it cannot.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{tile_windows, unavailable, with_screen};
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TileH;

impl UiCommand for TileH {
    const ID: &'static str = "tile_h";
    const SAKURA: &'static str = "F_TILE_H";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        with_screen(window, cx, |screen, all, current| {
            tile_windows(screen, all, current, false)
        })
        .map_err(unavailable)
    }
}

register_ui_command!(TileH);
