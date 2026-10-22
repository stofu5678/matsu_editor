//! `F_FILESAVECLOSE`: saves and closes the window (保存して閉じる).
//!
//! Sakura: `sakura_core/cmd/CViewCommander.cpp`, `case F_FILESAVECLOSE`: an unmodified document is
//! only closed.

use crate::file_ops::close::save_and_close_window;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileSaveClose;

impl UiCommand for FileSaveClose {
    const ID: &'static str = "file_save_close";
    const SAKURA: &'static str = "F_FILESAVECLOSE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            save_and_close_window(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSaveClose);
