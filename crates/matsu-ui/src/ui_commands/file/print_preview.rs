//! `F_PRINT_PREVIEW`: shows the print preview in place of the text.
//!
//! Sakura: `CViewCommander::Command_PRINT_PREVIEW` in `sakura_core/cmd/CViewCommander_File.cpp`.
//!
//! Like Sakura, the command turns the preview on and off (`CEditWnd::PrintPreviewModeONOFF`,
//! `CEditWnd.cpp`): a macro that calls it while the preview is open closes it. The user closes the
//! preview by its button and Esc (the keys of the text do not reach it). The command returns at
//! once, also for a macro: the macro goes on while the preview is shown, as in Sakura, where
//! `PrintPreviewModeONOFF` has no modal loop (see `macros/wait.rs`).

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
        if view.is_print_preview_open() {
            view.close_print_preview(window, cx);
        } else {
            view.open_print_preview(window, cx);
        }
        Ok(())
    }
}

register_ui_command!(PrintPreview);
