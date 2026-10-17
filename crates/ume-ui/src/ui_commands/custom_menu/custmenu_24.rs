//! `F_CUSTMENU_24`: custom menu 24 (カスタムメニュー24), which is also the tab menu.
//!
//! Sakura: `CViewCommander::Command_CUSTMENU` (`sakura_core/cmd/CViewCommander_CustMenu.cpp`). See
//! [`crate::custom_menu`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Custmenu24;

impl UiCommand for Custmenu24 {
    const ID: &'static str = "custmenu_24";
    const SAKURA: &'static str = "F_CUSTMENU_24";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::custom_menu::run(24, view, window, cx)
    }
}

register_ui_command!(Custmenu24);
