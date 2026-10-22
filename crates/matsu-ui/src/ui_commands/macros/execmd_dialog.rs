//! `F_EXECMD_DIALOG`: opens the dialog of 外部コマンド実行 and runs what it asks for.
//!
//! Sakura: `CViewCommander::Command_EXECCOMMAND_DIALOG` (`sakura_core/cmd/CViewCommander_Macro.cpp`),
//! `CDlgExec` (`sakura_core/dlg/CDlgExec.cpp`). The running is `F_EXECMD` (`execmd.rs`); the flow
//! is in `crate::exec`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::exec::flow::run_dialog;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ExecmdDialog;

impl UiCommand for ExecmdDialog {
    const ID: &'static str = "execmd_dialog";
    const SAKURA: &'static str = "F_EXECMD_DIALOG";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        start(window, cx, async move |view, cx| {
            run_dialog(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(ExecmdDialog);
