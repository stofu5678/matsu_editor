//! `F_FILECLOSE_OPEN`: closes the file and opens another in the same window (閉じて開く).
//!
//! Sakura: `CDocFileOperation::FileCloseOpen` (`sakura_core/doc/CDocFileOperation.cpp`): asks to
//! save a modified document, shows the open dialog, and keeps the file if the dialog is cancelled.

use crate::file_ops::close::close_and_open;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileCloseOpen;

impl UiCommand for FileCloseOpen {
    const ID: &'static str = "file_close_open";
    const SAKURA: &'static str = "F_FILECLOSE_OPEN";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            close_and_open(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(FileCloseOpen);
