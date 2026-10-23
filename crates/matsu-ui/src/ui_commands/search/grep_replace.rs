//! `F_GREP_REPLACE`: searches files for a pattern, replaces it in them, and writes what was
//! replaced to a window.
//!
//! Sakura: `CViewCommander::Command_GREP_REPLACE` (`sakura_core/cmd/CViewCommander_Grep.cpp`) for
//! the call without arguments, which uses what the dialog left; the macro form
//! `GrepReplace(pattern, replacement, files, folder, flags, encoding)` is in
//! `CMacro::HandleCommand` and `CMacro::MakeGrepInfo` (`sakura_core/macro/CMacro.cpp`) and always
//! writes the result to a new window. The arguments are those of `Grep` (`grep.rs`) after the
//! pattern and the new text; the flags 0x100000 (クリップボードから貼り付け) and 0x200000
//! (バックアップ作成) are added. The pattern must not be empty. The files are changed.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, CommandError, Result};

use crate::grep::settings::GrepSettings;
use crate::grep::window::remembered;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct GrepReplace;

impl UiCommand for GrepReplace {
    const ID: &'static str = "grep_replace";
    const SAKURA: &'static str = "F_GREP_REPLACE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if args.is_empty() {
            // `HandleCommand(F_GREP_REPLACE)` with nothing: what the dialog left.
            let mut settings = remembered(cx);
            let info = view.grep_replace_info(&mut settings, true, window, cx)?;
            return if view.start_grep(info, false, window, cx) {
                Ok(())
            } else {
                Err(CommandError::BadArgs("Grep did not start".to_owned()))
            };
        }
        let mut settings = GrepSettings::from_replace_macro_args(args)?;
        let info = view.grep_replace_info(&mut settings, false, window, cx)?;
        if view.start_grep(info, true, window, cx) {
            Ok(())
        } else {
            Err(CommandError::BadArgs("Grep did not start".to_owned()))
        }
    }
}

register_ui_command!(GrepReplace);
