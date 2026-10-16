//! `F_FILE_REOPEN_UNICODEBE`: reads the file again in UTF-16BE.
//!
//! Sakura: `CViewCommander::Command_FILE_REOPEN` and `CDocFileOperation::ReloadCurrentFile`
//! (`sakura_core/cmd/CViewCommander_File.cpp`, `sakura_core/doc/CDocFileOperation.cpp`). A
//! modified document asks before the changes are lost, unless the argument is non-zero
//! (`bNoConfirm`). The caret stays where it was.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::encoding::Encoding;

use crate::file_ops::args::no_confirm_arg;
use crate::file_ops::load::reopen;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct FileReopenUnicodeBe;

impl UiCommand for FileReopenUnicodeBe {
    const ID: &'static str = "file_reopen_unicodebe";
    const SAKURA: &'static str = "F_FILE_REOPEN_UNICODEBE";

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
            reopen(&view, cx, Some(Encoding::Utf16Be), no_confirm).await;
        });
        Ok(())
    }
}

register_ui_command!(FileReopenUnicodeBe);
