//! `F_TYPE_HELPER`: shows the type settings of the type of this document at the page 支援
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
pub struct TypeHelper;

impl UiCommand for TypeHelper {
    const ID: &'static str = "type_helper";
    const SAKURA: &'static str = "F_TYPE_HELPER";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        type_settings::run_command_at(view, window, cx, Some(page::HELPER))
    }
}

register_ui_command!(TypeHelper);
