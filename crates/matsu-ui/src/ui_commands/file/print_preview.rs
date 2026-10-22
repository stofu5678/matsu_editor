//! `F_PRINT_PREVIEW`: shows the print preview in place of the text.
//!
//! Sakura: `CViewCommander::Command_PRINT_PREVIEW` in `sakura_core/cmd/CViewCommander_File.cpp`.
//!
//! Not like Sakura: Sakura's command turns the preview on and off. Here the preview closes by
//! its button and Esc (the keys of the text do not reach it), so the command only opens it, and
//! does nothing while it is open.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct PrintPreview;

impl UiCommand for PrintPreview {
    const ID: &'static str = "print_preview";
    const SAKURA: &'static str = "F_PRINT_PREVIEW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_print_preview(window, cx);
        Ok(())
    }
}

register_ui_command!(PrintPreview);
