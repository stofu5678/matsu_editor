//! `F_SETFONTSIZEDOWN`: The same, one size smaller. Ctrl+wheel down in Sakura's key table.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SetFontSizeDown;

impl UiCommand for SetFontSizeDown {
    const ID: &'static str = "set_font_size_down";
    const SAKURA: &'static str = "F_SETFONTSIZEDOWN";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.zoom_font(-1, cx);
        Ok(())
    }
}

register_ui_command!(SetFontSizeDown);
