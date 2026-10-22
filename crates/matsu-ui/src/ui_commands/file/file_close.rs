//! `F_FILECLOSE`: closes the file and leaves an untitled document (閉じて(無題)).
//!
//! Sakura: `CDocFileOperation::FileClose` (`sakura_core/doc/CDocFileOperation.cpp`): asks to save
//! a modified document first; cancelling keeps everything.

use crate::file_ops::close::close_file;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileClose;

impl UiCommand for FileClose {
    const ID: &'static str = "file_close";
    const SAKURA: &'static str = "F_FILECLOSE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            close_file(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(FileClose);
