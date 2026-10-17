//! `F_OPTION_TYPE`: shows the type settings (タイプ別設定...) of the type of this document.
//!
//! Sakura: `Command_OPTION_TYPE` (`sakura_core/cmd/CViewCommander_Settings.cpp`) opens
//! `CPropTypes::DoPropertySheet` for the type of the document. The window is `crate::type_settings`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::type_settings;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OptionType;

impl UiCommand for OptionType {
    const ID: &'static str = "option_type";
    const SAKURA: &'static str = "F_OPTION_TYPE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        type_settings::run_command(view, window, cx)
    }
}

register_ui_command!(OptionType);
