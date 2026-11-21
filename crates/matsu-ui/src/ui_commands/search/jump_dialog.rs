//! `F_JUMP_DIALOG`: opens the go-to-line panel (指定行へジャンプ).
//!
//! Sakura: `CViewCommander::Command_JUMP_DIALOG` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`). Lines are
//! counted by the unit that the panel has chosen (改行単位 or 折り返し単位, `m_bLineNumIsCRLF_ForJump`).

use crate::search::PanelMode;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct JumpDialog;

impl UiCommand for JumpDialog {
    const ID: &'static str = "jump_dialog";
    const SAKURA: &'static str = "F_JUMP_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_search_panel(PanelMode::Jump, window, cx);
        Ok(())
    }
}

register_ui_command!(JumpDialog);
