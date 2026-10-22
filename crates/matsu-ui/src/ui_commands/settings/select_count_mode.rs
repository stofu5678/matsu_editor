//! `F_SELECT_COUNT_MODE`: changes how the status bar counts the selection, for this window.
//!
//! Sakura: `CViewCommander::Command_SELECT_COUNT_MODE` (`sakura_core/cmd/CViewCommander_Settings.cpp`).
//! The number is 0 (toggle between characters and bytes; a missing number is 0), 1 (characters)
//! or 2 (bytes); another number changes nothing. The setting of the common settings is not
//! written. The menu check is `IsFuncChecked` (`sakura_core/func/Funccode.cpp`), here
//! `crate::menu_state`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::ui_commands::macro_only::int_arg;
use crate::view::EditorView;

#[derive(Default)]
pub struct SelectCountMode;

impl UiCommand for SelectCountMode {
    const ID: &'static str = "select_count_mode";
    const SAKURA: &'static str = "F_SELECT_COUNT_MODE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let code = i32::try_from(int_arg("SelectCountMode", args, 0, 0)?).unwrap_or(-1);
        let setting = crate::status_options::current(cx).select_count_by_byte;
        view.prefs.select_count = view.prefs.select_count.after_code(code, setting);
        cx.notify();
        Ok(())
    }
}

register_ui_command!(SelectCountMode);
