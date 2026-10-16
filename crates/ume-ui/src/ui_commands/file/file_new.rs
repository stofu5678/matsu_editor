//! `F_FILENEW`: a new untitled document in a new window.
//!
//! Sakura: `CViewCommander::Command_FILENEW` (`sakura_core/cmd/CViewCommander_File.cpp`) starts a
//! new editor window with no file.

use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileNew;

impl UiCommand for FileNew {
    const ID: &'static str = "file_new";
    const SAKURA: &'static str = "F_FILENEW";

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

register_ui_command!(FileNew);
