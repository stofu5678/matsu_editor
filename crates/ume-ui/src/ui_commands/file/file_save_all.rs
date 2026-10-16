//! `F_FILESAVEALL`: saves every window quietly.
//!
//! Sakura: `Command_FILESAVEALL` (`sakura_core/cmd/CViewCommander_File.cpp`) sends
//! `F_FILESAVE_QUIET` to every editor window. That save asks for no name: a document without a
//! file is left alone, and one without changes writes nothing. In view mode the file is not
//! overwritten.

use crate::file_ops::save::save_all;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileSaveAll;

impl UiCommand for FileSaveAll {
    const ID: &'static str = "file_save_all";
    const SAKURA: &'static str = "F_FILESAVEALL";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            let _ = view;
            save_all(cx).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSaveAll);
