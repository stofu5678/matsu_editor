//! `F_CUSTMENU_13`: custom menu 13 (カスタムメニュー13).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu13;

impl UiCommand for Custmenu13 {
    const ID: &'static str = "custmenu_13";
    const SAKURA: &'static str = "F_CUSTMENU_13";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(13, view, window, cx)
    }
}

register_ui_command!(Custmenu13);
