//! `F_PRINT_PAGESETUP`: Sakura's print page setup dialog.
//!
//! Sakura: `CViewCommander::Command_PRINT_PAGESETUP` in `sakura_core/cmd/CViewCommander_File.cpp`
//! calls `CEditWnd::OnPrintPageSetting`, which shows `CDlgPrintSetting`
//! (`sakura_core/dlg/CDlgPrintSetting.cpp`). The dialog works with or without the print preview:
//! it does not open the preview, and a preview that is open lays the document out again with what
//! OK keeps. The dialog and the settings are in `crate::print_setting`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct PrintPagesetup;

impl UiCommand for PrintPagesetup {
    const ID: &'static str = "print_pagesetup";
    const SAKURA: &'static str = "F_PRINT_PAGESETUP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.print_page_setup(window, cx);
        Ok(())
    }
}

register_ui_command!(PrintPagesetup);
