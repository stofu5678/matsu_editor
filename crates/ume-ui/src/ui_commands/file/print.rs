//! `F_PRINT`: opens the print preview and then the print dialog of the OS.
//!
//! Sakura: `CViewCommander::Command_PRINT` in `sakura_core/cmd/CViewCommander_File.cpp` opens the
//! preview (`Command_PRINT_PREVIEW`) and prints from it (`CPrintPreview::OnPrint`). The preview
//! closes after printing and stays when the dialog is cancelled. The printing itself is in
//! `ume-platform` (ADR-0011).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Print;

impl UiCommand for Print {
    const ID: &'static str = "print";
    const SAKURA: &'static str = "F_PRINT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.print(window, cx);
        Ok(())
    }
}

register_ui_command!(Print);
