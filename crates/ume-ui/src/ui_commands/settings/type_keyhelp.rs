//! `F_TYPE_KEYHELP`: shows the type settings of the type of this document at the page キーワードヘルプ
//!
//! Sakura has no command for this function: it is the number of the help topic of the page
//! (`FuncID_To_HelpContextID`, `sakura_core/func/Funccode.cpp`). Like `F_OPTION_TYPE`
//! (`Command_OPTION_TYPE`, `sakura_core/cmd/CViewCommander_Settings.cpp`), it opens the sheet for the
//! type of the document, at that page. The window is `crate::type_settings`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::type_settings::{self, page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct TypeKeyhelp;

impl UiCommand for TypeKeyhelp {
    const ID: &'static str = "type_keyhelp";
    const SAKURA: &'static str = "F_TYPE_KEYHELP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        type_settings::run_command_at(view, window, cx, Some(page::KEYHELP))
    }
}

register_ui_command!(TypeKeyhelp);
