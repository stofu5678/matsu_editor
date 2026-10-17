//! `F_MENU_RBUTTON`: the right-click menu (右クリックメニュー), custom menu 0.
//!
//! Sakura: `CViewCommander::Command_MENU_RBUTTON` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct MenuRbutton;

impl UiCommand for MenuRbutton {
    const ID: &'static str = "menu_rbutton";
    const SAKURA: &'static str = "F_MENU_RBUTTON";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(0, view, window, cx)
    }
}

register_ui_command!(MenuRbutton);
