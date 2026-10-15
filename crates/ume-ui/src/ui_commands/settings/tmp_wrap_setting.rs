//! `F_TMPWRAPSETTING`: Wrap at the column of the document type (`WRAP_SETTING_WIDTH`), for this view only.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::settings::WrapMethod;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct TmpWrapSetting;

impl UiCommand for TmpWrapSetting {
    const ID: &'static str = "tmp_wrap_setting";
    const SAKURA: &'static str = "F_TMPWRAPSETTING";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.set_wrap_method(WrapMethod::SettingWidth, cx);
        Ok(())
    }
}

register_ui_command!(TmpWrapSetting);
