//! `F_INS_FOLDER_USED_RECENTLY`: inserts the recent folders, one on each line (最近使ったフォルダー挿入).
//!
//! Sakura: `CViewCommander::Command_INS_FOLDER_USED_RECENTLY` (`sakura_core/cmd/CViewCommander_Insert.cpp`)
//! writes each path of the folder history (newest first, as many as the folder menu would show)
//! followed by the line ending of the document, and inserts the text at the caret
//! (`Command_INSTEXT`). The history is the application's, so this is a command of the window.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct InsFolderUsedRecently;

impl UiCommand for InsFolderUsedRecently {
    const ID: &'static str = "ins_folder_used_recently";
    const SAKURA: &'static str = "F_INS_FOLDER_USED_RECENTLY";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let eol = view.document().read(cx).new_line().as_str();
        let text = crate::history::current(cx)
            .map(|history| history.folders_text(eol))
            .unwrap_or_default();
        if text.is_empty() {
            return Ok(());
        }
        // One undo step.
        view.run_with(cx, |command_cx| {
            command_cx.begin_block();
            let result = command_cx.insert_text(&text);
            command_cx.end_block();
            result
        })
        .map(|_| ())
    }
}

register_ui_command!(InsFolderUsedRecently);
