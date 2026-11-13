//! `F_ONELINER_SETTINGS` (「ワンライナーの設定」): shows the common settings at the page ワンライナー
//!
//! MATSU Editor's own function (ADR-0029); Sakura has nothing like it. It does what the `F_OPTION_*` of
//! the pages do (`CPropCommon::DoPropertySheet`, `sakura_core/prop/CPropCommon.cpp`): it opens the sheet at
//! one page. The window is `crate::common_settings`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::common_settings::{self, Page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnelinerSettings;

impl UiCommand for OnelinerSettings {
    const ID: &'static str = "oneliner_settings";
    const SAKURA: &'static str = "F_ONELINER_SETTINGS";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        common_settings::run_command(view, window, cx, Some(Page::OneLiner))
    }
}

register_ui_command!(OnelinerSettings);
