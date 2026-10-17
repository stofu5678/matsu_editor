//! `F_CUSTMENU_4`: custom menu 4 (カスタムメニュー4).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu4;

impl UiCommand for Custmenu4 {
    const ID: &'static str = "custmenu_4";
    const SAKURA: &'static str = "F_CUSTMENU_4";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(4, view, window, cx)
    }
}

register_ui_command!(Custmenu4);
