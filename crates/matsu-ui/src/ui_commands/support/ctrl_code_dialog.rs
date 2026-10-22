//! `F_CTRL_CODE_DIALOG`: the control code dialog (コントロールコード入力); the chosen code is typed
//! with `F_CTRL_CODE`.
//!
//! Sakura: `CViewCommander::Command_CtrlCode_Dialog` (`sakura_core/cmd/CViewCommander_Insert.cpp`).
//! See [`crate::support::ctrl_code`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CtrlCodeDialog;

impl UiCommand for CtrlCodeDialog {
    const ID: &'static str = "ctrl_code_dialog";
    const SAKURA: &'static str = "F_CTRL_CODE_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_ctrl_code_dialog(window, cx);
        Ok(())
    }
}

register_ui_command!(CtrlCodeDialog);
