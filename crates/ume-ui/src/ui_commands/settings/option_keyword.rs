//! `F_OPTION_KEYWORD`: shows the common settings at the page 強調キーワード
//!
//! Sakura has no command for this function: it is the number of the help topic of the page
//! (`FuncID_To_HelpContextID`, `sakura_core/func/Funccode.cpp`). Like the other `F_OPTION_*` of the
//! pages, it opens the sheet of `CPropCommon::DoPropertySheet` (`sakura_core/prop/CPropCommon.cpp`) at
//! that page. The window is `crate::common_settings`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::common_settings::{self, Page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OptionKeyword;

impl UiCommand for OptionKeyword {
    const ID: &'static str = "option_keyword";
    const SAKURA: &'static str = "F_OPTION_KEYWORD";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        common_settings::run_command(view, window, cx, Some(Page::Keyword))
    }
}

register_ui_command!(OptionKeyword);
