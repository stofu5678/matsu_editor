//! `F_SETFONTSIZEUP`: `Command_SETFONTSIZE` in mode 2 with a shift of +1: the next larger size of the zoom table, for this view only (`sakura_core/cmd/CViewCommander.cpp`: `HandleCommand( F_SETFONTSIZE, bRedraw, 0, 1, 2, 0)`; `sakura_core/cmd/CViewCommander_Settings.cpp`). Ctrl+wheel up in Sakura's key table.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SetFontSizeUp;

impl UiCommand for SetFontSizeUp {
    const ID: &'static str = "set_font_size_up";
    const SAKURA: &'static str = "F_SETFONTSIZEUP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.zoom_font(1, cx);
        Ok(())
    }
}

register_ui_command!(SetFontSizeUp);
