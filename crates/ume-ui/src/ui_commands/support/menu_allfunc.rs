//! `F_MENU_ALLFUNC`: the command list (コマンド一覧), a menu of every function by group; choosing
//! one runs it.
//!
//! Sakura: `CViewCommander::Command_MENU_ALLFUNC` (`sakura_core/cmd/CViewCommander_Support.cpp`).
//! See [`crate::support::command_list`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct MenuAllfunc;

impl UiCommand for MenuAllfunc {
    const ID: &'static str = "menu_allfunc";
    const SAKURA: &'static str = "F_MENU_ALLFUNC";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.open_command_list(window, cx);
        Ok(())
    }
}

register_ui_command!(MenuAllfunc);
