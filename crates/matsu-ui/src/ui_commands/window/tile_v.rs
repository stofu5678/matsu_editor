//! `F_TILE_V`: puts the windows one above the other (上下に並べて表示).
//!
//! Sakura: `Command_TILE_V` (`sakura_core/cmd/CViewCommander_Window.cpp`). Minimized windows are
//! left alone; the others share the work area of this window's screen in equal heights, this
//! window at the top, and get the full width. This window gets the focus. Moving a window needs
//! the OS (`matsu_platform::window`): Linux reports that it cannot.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::{tile_windows, unavailable, with_screen};
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TileV;

impl UiCommand for TileV {
    const ID: &'static str = "tile_v";
    const SAKURA: &'static str = "F_TILE_V";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        with_screen(window, cx, |screen, all, current| {
            tile_windows(screen, all, current, true)
        })
        .map_err(unavailable)
    }
}

register_ui_command!(TileV);
