//! `F_CUSTMENU_17`: custom menu 17 (カスタムメニュー17).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu17;

impl UiCommand for Custmenu17 {
    const ID: &'static str = "custmenu_17";
    const SAKURA: &'static str = "F_CUSTMENU_17";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(17, view, window, cx)
    }
}

register_ui_command!(Custmenu17);
