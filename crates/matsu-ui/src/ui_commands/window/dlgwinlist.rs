//! `F_DLGWINLIST`: shows the dialog of the editor windows (ウィンドウ一覧表示...).
//!
//! Sakura: `Command_DLGWINLIST` (`sakura_core/cmd/CViewCommander_Window.cpp`) asks the tray
//! process to show `CDlgWindowList` (`sakura_core/dlg/CDlgWindowList.cpp`): a list of the
//! windows with check boxes and the buttons 上書き保存, ウィンドウを閉じる and OK. The dialog is
//! `crate::window_ops::list`.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::list;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Dlgwinlist;

impl UiCommand for Dlgwinlist {
    const ID: &'static str = "dlgwinlist";
    const SAKURA: &'static str = "F_DLGWINLIST";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        list::open(view, true, window, cx);
        Ok(())
    }
}

register_ui_command!(Dlgwinlist);
