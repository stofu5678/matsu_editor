//! `F_FILESAVE_QUIET`: saves the file without asking for a name.
//!
//! Sakura: `Command_FILESAVE(false, false)` (`sakura_core/cmd/CViewCommander_File.cpp`), used by
//! "save all": a document without a file is left alone.

use crate::file_ops::save::{SaveRequest, save};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileSaveQuiet;

impl UiCommand for FileSaveQuiet {
    const ID: &'static str = "file_save_quiet";
    const SAKURA: &'static str = "F_FILESAVE_QUIET";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            let request = SaveRequest {
                ask_name: false,
                ..SaveRequest::default()
            };
            save(&view, cx, request).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSaveQuiet);
