//! `F_OPTION_GREP`: shows the common settings at the page 検索
//!
//! Sakura: `CPropCommon::DoPropertySheet` (`sakura_core/prop/CPropCommon.cpp`): the sheet opens at the page 検索. The
//! window is `crate::common_settings`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::common_settings::{self, Page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OptionGrep;

impl UiCommand for OptionGrep {
    const ID: &'static str = "option_grep";
    const SAKURA: &'static str = "F_OPTION_GREP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        common_settings::run_command(view, window, cx, Some(Page::Search))
    }
}

register_ui_command!(OptionGrep);
