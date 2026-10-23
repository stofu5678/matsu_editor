//! `F_FILE_REOPEN_SJIS`: reads the file again in Shift_JIS.
//!
//! Sakura: `CViewCommander::Command_FILE_REOPEN` and `CDocFileOperation::ReloadCurrentFile`
//! (`sakura_core/cmd/CViewCommander_File.cpp`, `sakura_core/doc/CDocFileOperation.cpp`). A
//! modified document asks before the changes are lost, unless the argument is non-zero
//! (`bNoConfirm`). The caret stays where it was.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::encoding::Encoding;

use crate::file_ops::args::no_confirm_arg;
use crate::file_ops::load::reopen;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct FileReopenSjis;

impl UiCommand for FileReopenSjis {
    const ID: &'static str = "file_reopen_sjis";
    const SAKURA: &'static str = "F_FILE_REOPEN_SJIS";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let no_confirm = no_confirm_arg(args);
        let _ = view;
        start(window, cx, async move |view, cx| {
            reopen(&view, cx, Some(Encoding::ShiftJis), no_confirm).await;
        });
        Ok(())
    }
}

register_ui_command!(FileReopenSjis);
