//! `F_OPTION_STATUSBAR`: shows the common settings at the page ステータスバー
//!
//! Sakura: `CPropCommon::DoPropertySheet` (`sakura_core/prop/CPropCommon.cpp`): the sheet opens at the page ステータスバー. The
//! window is `crate::common_settings`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::common_settings::{self, Page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OptionStatusbar;

impl UiCommand for OptionStatusbar {
    const ID: &'static str = "option_statusbar";
    const SAKURA: &'static str = "F_OPTION_STATUSBAR";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        common_settings::run_command(view, window, cx, Some(Page::StatusBar))
    }
}

register_ui_command!(OptionStatusbar);
