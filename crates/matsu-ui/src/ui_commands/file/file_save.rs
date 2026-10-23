//! `F_FILESAVE`: saves the file; a document without a file asks for a name.
//!
//! Sakura: `Command_FILESAVE` (`sakura_core/cmd/CViewCommander_File.cpp`). Nothing is written if
//! the document is unmodified.

use crate::file_ops::save::{SaveRequest, save};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileSave;

impl UiCommand for FileSave {
    const ID: &'static str = "file_save";
    const SAKURA: &'static str = "F_FILESAVE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            save(&view, cx, SaveRequest::default()).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSave);
