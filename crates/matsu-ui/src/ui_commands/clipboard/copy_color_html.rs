//! `F_COPY_COLOR_HTML`: copies the selection as HTML with the colors of the document type.
//!
//! Sakura: `CViewCommander::Command_COPY_COLOR_HTML` (`sakura_core/cmd/CViewCommander_Clipboard.cpp`).
//! See [`crate::copy_html`] for what is copied.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CopyColorHtml;

impl UiCommand for CopyColorHtml {
    const ID: &'static str = "copy_color_html";
    const SAKURA: &'static str = "F_COPY_COLOR_HTML";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.copy_color_html(false, cx);
        Ok(())
    }
}

register_ui_command!(CopyColorHtml);
