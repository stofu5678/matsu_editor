//! `F_AI_SETTINGS` (「生成 AI の設定...」): shows the common settings (ADR-0035, 1). MATSU Editor's own function;
//! Sakura has nothing like it. It does what `F_ONELINER_SETTINGS` does: it opens the sheet at one page
//! (`CPropCommon::DoPropertySheet`, `sakura_core/prop/CPropCommon.cpp`). The page 「生成 AI」 is a later step of
//! ADR-0035; until then the sheet opens at 「ワンライナー」.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::common_settings::{self, Page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct AiSettings;

impl UiCommand for AiSettings {
    const ID: &'static str = "ai_settings";
    const SAKURA: &'static str = "F_AI_SETTINGS";

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

register_ui_command!(AiSettings);
