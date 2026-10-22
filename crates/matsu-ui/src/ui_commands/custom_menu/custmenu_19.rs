//! `F_CUSTMENU_19`: custom menu 19 (カスタムメニュー19).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu19;

impl UiCommand for Custmenu19 {
    const ID: &'static str = "custmenu_19";
    const SAKURA: &'static str = "F_CUSTMENU_19";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(19, view, window, cx)
    }
}

register_ui_command!(Custmenu19);
