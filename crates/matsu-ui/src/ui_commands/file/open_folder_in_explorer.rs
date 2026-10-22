//! `F_OPEN_FOLDER_IN_EXPLORER`: shows the file in the file manager of the OS.
//!
//! Sakura: `Command_OPEN_FOLDER_IN_EXPLORER` (`sakura_core/cmd/CViewCommander_File.cpp`) runs
//! `explorer.exe /select,"<file>"`; a document without a file beeps. GPUI's `reveal_path` does
//! the same on each OS: Explorer with the file selected, Finder, and on Linux the file manager
//! through the desktop portal, else the folder with `xdg-open`.

use crate::file_ops::external::document_file;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct OpenFolderInExplorer;

impl UiCommand for OpenFolderInExplorer {
    const ID: &'static str = "open_folder_in_explorer";
    const SAKURA: &'static str = "F_OPEN_FOLDER_IN_EXPLORER";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if let Some(path) = document_file(view, cx) {
            cx.reveal_path(&path);
        }
        Ok(())
    }
}

register_ui_command!(OpenFolderInExplorer);
