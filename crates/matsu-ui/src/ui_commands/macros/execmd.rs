//! `F_EXECMD` (the macro `ExecCommand(command, option, folder)`): runs an external command.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the
//! command is missing, reads the option as a number (0 when left out: nothing is taken from the
//! command) and passes the folder only when there is one; `Command_EXECCOMMAND`
//! (`sakura_core/cmd/CViewCommander_Macro.cpp`) expands `$F` and `$$`, and
//! `CEditView::ExecCmd` (`sakura_core/view/CEditView_ExecCmd.cpp`) runs it. The sum of bits:
//!
//! | bit | meaning |
//! |---|---|
//! | 0x01 | take the standard output (and the error) |
//! | 0x02 | put it in the window being edited (else in the output panel) |
//! | 0x04 | send the selection, or the document, to the standard input |
//! | 0x08 / 0x80 | the output is Unicode / UTF-8 (else Shift_JIS) |
//! | 0x10 / 0x100 | the input is Unicode / UTF-8 (else Shift_JIS) |
//! | 0x20 / 0x40 | write / do not write the command line, the date and the exit code |
//! | 0x200 | run in the folder of the third argument |
//!
//! A macro does not change what the dialog remembers (`CDlgExec`). The flow is in `crate::exec`;
//! a macro that called this waits until the command has ended.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, CommandError, Result};
use matsu_core::exec::ExecFlags;

use super::super::macro_only::{int_arg, text_arg};
use crate::exec::flow::{Request, run_command};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Execmd;

impl UiCommand for Execmd {
    const ID: &'static str = "execmd";
    const SAKURA: &'static str = "F_EXECMD";

    fn run(
        &self,
        _view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let command = text_arg("ExecCommand", args, 0)?.to_owned();
        let option = int_arg("ExecCommand", args, 1, 0)?;
        let flags = i32::try_from(option).map_err(|_| {
            CommandError::BadArgs(format!("ExecCommand: {option} is not an option"))
        })?;
        let folder = match args.get(2) {
            None => None,
            Some(Arg::Text(folder)) => Some(folder.clone()),
            Some(_) => {
                return Err(CommandError::BadArgs(
                    "ExecCommand needs the folder as text (argument 3)".to_owned(),
                ));
            }
        };
        let request = Request {
            command,
            flags: ExecFlags(flags),
            folder,
            jump: None,
        };
        start(window, cx, async move |view, cx| {
            run_command(&view, cx, request).await;
        });
        Ok(())
    }
}

register_ui_command!(Execmd);
