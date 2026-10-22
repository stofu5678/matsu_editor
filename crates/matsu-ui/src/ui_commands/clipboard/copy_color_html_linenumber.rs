//! `F_COPY_COLOR_HTML_LINENUMBER`: like `F_COPY_COLOR_HTML`, with the line number in front of
//! each row.
//!
//! Sakura: `CViewCommander::Command_COPY_COLOR_HTML_LINENUMBER`
//! (`sakura_core/cmd/CViewCommander_Clipboard.cpp`), which calls `Command_COPY_COLOR_HTML(true)`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CopyColorHtmlLinenumber;

impl UiCommand for CopyColorHtmlLinenumber {
    const ID: &'static str = "copy_color_html_linenumber";
    const SAKURA: &'static str = "F_COPY_COLOR_HTML_LINENUMBER";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.copy_color_html(true, cx);
        Ok(())
    }
}

register_ui_command!(CopyColorHtmlLinenumber);
