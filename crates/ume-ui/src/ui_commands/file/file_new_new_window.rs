//! `F_FILENEW_NEWWINDOW`: a new untitled document in a new window.
//!
//! Sakura: `Command_FILENEW_NEWWINDOW` (`sakura_core/cmd/CViewCommander_File.cpp`) differs from
//! `F_FILENEW` only in what the window does with tabs, which we do not have.

use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileNewNewWindow;

impl UiCommand for FileNewNewWindow {
    const ID: &'static str = "file_new_new_window";
    const SAKURA: &'static str = "F_FILENEW_NEWWINDOW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = (view, window);
        open_editor_window(cx, SharedDocument::empty(), None).ok();
        Ok(())
    }
}

register_ui_command!(FileNewNewWindow);
