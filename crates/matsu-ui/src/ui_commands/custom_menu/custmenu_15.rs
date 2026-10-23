//! `F_CUSTMENU_15`: custom menu 15 (カスタムメニュー15).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu15;

impl UiCommand for Custmenu15 {
    const ID: &'static str = "custmenu_15";
    const SAKURA: &'static str = "F_CUSTMENU_15";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(15, view, window, cx)
    }
}

register_ui_command!(Custmenu15);
