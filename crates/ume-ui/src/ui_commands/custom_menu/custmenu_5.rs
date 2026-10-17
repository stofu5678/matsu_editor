//! `F_CUSTMENU_5`: custom menu 5 (カスタムメニュー5).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu5;

impl UiCommand for Custmenu5 {
    const ID: &'static str = "custmenu_5";
    const SAKURA: &'static str = "F_CUSTMENU_5";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(5, view, window, cx)
    }
}

register_ui_command!(Custmenu5);
