//! `F_OPTION_FNAME`: shows the common settings at the page ファイル名表示
//!
//! Sakura: `CPropCommon::DoPropertySheet` (`sakura_core/prop/CPropCommon.cpp`): the sheet opens at the page ファイル名表示. The
//! window is `crate::common_settings`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::common_settings::{self, Page};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OptionFname;

impl UiCommand for OptionFname {
    const ID: &'static str = "option_fname";
    const SAKURA: &'static str = "F_OPTION_FNAME";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        common_settings::run_command(view, window, cx, Some(Page::FileName))
    }
}

register_ui_command!(OptionFname);
