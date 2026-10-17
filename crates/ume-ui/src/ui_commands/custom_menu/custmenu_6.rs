//! `F_CUSTMENU_6`: custom menu 6 (カスタムメニュー6).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu6;

impl UiCommand for Custmenu6 {
    const ID: &'static str = "custmenu_6";
    const SAKURA: &'static str = "F_CUSTMENU_6";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(6, view, window, cx)
    }
}

register_ui_command!(Custmenu6);
