//! `F_GREP`: searches files for a pattern and writes the result to a window.
//!
//! Sakura: `CViewCommander::Command_GREP` (`sakura_core/cmd/CViewCommander_Grep.cpp`) for the call
//! without arguments, which uses what the dialog left; the macro form `Grep(pattern, files,
//! folder, flags, encoding)` is in `CMacro::HandleCommand` and `CMacro::MakeGrepInfo`
//! (`sakura_core/macro/CMacro.cpp`) and always writes the result to a new window.
//!
//! Arguments of the macro: the pattern, the files (as in the dialog; `!pattern` and `#pattern` exclude
//! files and folders, which is what a recorded Grep writes), the folder, the sum of the flags (1 subfolders,
//! 4 case, 8 regular expression, 0x10 detect the encoding, 0x20 lines, 0x40 per file, 0x80 result
//! only, 0x100 to 0xff00 the number of the encoding times 0x100, 0x10000 words, 0x20000 first hit
//! of a file, 0x40000 base folder, 0x80000 folders apart, 0x400000 lines without a hit, 0x800000
//! excluded files as regular expressions; 5 if left out) and the number of the encoding.

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};

use crate::grep::settings::GrepSettings;
use crate::grep::window::remembered;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Grep;

impl UiCommand for Grep {
    const ID: &'static str = "grep";
    const SAKURA: &'static str = "F_GREP";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if args.is_empty() {
            // `HandleCommand(F_GREP)` with nothing: what the dialog left.
            let mut settings = remembered(cx);
            return match settings.check() {
                Ok(info) => {
                    view.start_grep(info, false, window, cx);
                    Ok(())
                }
                Err(error) => {
                    view.grep_dialog_error_box(&error, window, cx);
                    Err(CommandError::BadArgs(error.message))
                }
            };
        }
        let info = GrepSettings::from_macro_args(args)?.info();
        if view.start_grep(info, true, window, cx) {
            Ok(())
        } else {
            Err(CommandError::BadArgs("Grep did not start".to_owned()))
        }
    }
}

register_ui_command!(Grep);
