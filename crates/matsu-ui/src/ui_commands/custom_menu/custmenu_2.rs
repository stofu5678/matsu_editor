//! `F_CUSTMENU_2`: custom menu 2 (カスタムメニュー2).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu2;

impl UiCommand for Custmenu2 {
    const ID: &'static str = "custmenu_2";
    const SAKURA: &'static str = "F_CUSTMENU_2";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(2, view, window, cx)
    }
}

register_ui_command!(Custmenu2);
