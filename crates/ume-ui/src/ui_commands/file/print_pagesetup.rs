//! `F_PRINT_PAGESETUP`: Sakura's print page setup dialog.
//!
//! Sakura: `CViewCommander::Command_PRINT_PAGESETUP` in `sakura_core/cmd/CViewCommander_File.cpp`
//! opens `CDlgPrintSetting` (`sakura_core/dlg/CDlgPrintSetting.cpp`).
//!
//! Not like Sakura yet: the dialog is not made. The command opens the print preview, whose bar
//! has the two settings that are easy to try (line numbers and two columns), and says so. The
//! settings are Sakura's defaults (`CPrint::SettingInitialize`) and are not saved.

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
        let preview = view.open_print_preview(window, cx);
        preview.update(cx, |preview, cx| {
            preview.notice = Some(
                "印刷ページ設定のダイアログはまだ使えない。行番号と2段組は、この画面のボタンで試せる"
                    .into(),
            );
            cx.notify();
        });
        Ok(())
    }
}

register_ui_command!(PrintPagesetup);
