//! `F_CUSTMENU_18`: custom menu 18 (カスタムメニュー18).
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu18;

impl UiCommand for Custmenu18 {
    const ID: &'static str = "custmenu_18";
    const SAKURA: &'static str = "F_CUSTMENU_18";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(18, view, window, cx)
    }
}

register_ui_command!(Custmenu18);
