//! `F_CUSTMENU_14`: custom menu 14 (カスタムメニュー14).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu14;

impl UiCommand for Custmenu14 {
    const ID: &'static str = "custmenu_14";
    const SAKURA: &'static str = "F_CUSTMENU_14";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(14, view, window, cx)
    }
}

register_ui_command!(Custmenu14);
