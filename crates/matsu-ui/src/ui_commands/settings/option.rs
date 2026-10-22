//! `F_OPTION`: shows the common settings (共通設定...) at the page used last
//!
//! Sakura: `CPropCommon::DoPropertySheet` (`sakura_core/prop/CPropCommon.cpp`): the sheet opens at `m_nPageNum`, the page used last. The
//! window is `crate::common_settings`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::common_settings;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Option;

impl UiCommand for Option {
    const ID: &'static str = "option";
    const SAKURA: &'static str = "F_OPTION";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        common_settings::run_command(view, window, cx, None)
    }
}

register_ui_command!(Option);
