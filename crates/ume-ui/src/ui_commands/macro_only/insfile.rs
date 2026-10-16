//! `F_INSFILE` (the macro `InsFile(file, code, option)`): inserts a file at the caret.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the file
//! name is missing and reads the code and the option as numbers (a missing one is 0, so the code
//! is Shift_JIS); `CViewCommander::Command_INSFILE` (`sakura_core/cmd/CViewCommander_File.cpp`)
//! returns without a message when the name is empty, reads the file in the given encoding (99,
//! `CODE_AUTODETECT`: the encoding the file had when it was last opened, else that of the
//! document; a number that is not an encoding: Shift_JIS), and inserts it line by line with
//! `Command_INSTEXT`, so the line endings of the file are kept as they are and a selection is
//! replaced. When there was a selection, the inserted text is selected afterwards. The option is
//! unused. A file that cannot be opened or read is shown to the user and the macro goes on.
//!
//! The history of the files that were opened (the "last opened encoding" of code 99) does not
//! exist here yet, so code 99 is the encoding of the document.

use std::path::Path;

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::buffer::Pos;
use ume_core::encoding::Encoding;
use ume_core::file::{OpenOptions, open_file};

use super::putfile::encoding_of;
use super::{int_arg, text_arg};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct InsFile;

impl UiCommand for InsFile {
    const ID: &'static str = "insfile";
    const SAKURA: &'static str = "F_INSFILE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let name = text_arg("InsFile", args, 0)?;
        let code = int_arg("InsFile", args, 1, 0)?;
        // the option is not used (`[[maybe_unused]] int nFlgOpt`)
        int_arg("InsFile", args, 2, 0)?;
        if name.is_empty() {
            return Ok(());
        }
        let document_encoding = view.document().read(cx).file().encoding;
        let encoding = encoding_of(code, document_encoding).unwrap_or(Encoding::ShiftJis);
        let text = match read_text(Path::new(name), encoding) {
            Ok(text) => text,
            Err(message) => {
                view.macro_failed(message, window, cx);
                return Ok(());
            }
        };
        // `bBeforeTextSelected` and `ptFrom`
        let from = view.cursor().selection().map(|(from, _)| from);
        if text.is_empty() {
            return Ok(());
        }
        view.run_with(cx, |command_cx| {
            command_cx.begin_block();
            let result = command_cx.insert_text(&text).and_then(|_| match from {
                Some(from) => {
                    let end = command_cx.caret();
                    command_cx.select_range(from, end)
                }
                None => Ok(()),
            });
            command_cx.end_block();
            result
        })
    }
}

/// The text of the file read in `encoding` (the BOM is dropped), with its line endings.
fn read_text(path: &Path, encoding: Encoding) -> std::result::Result<String, String> {
    let options = OpenOptions {
        encoding: Some(encoding),
        ..OpenOptions::default()
    };
    // `STR_GREP_ERR_FILEOPEN` and `STR_ERR_DLGEDITVWCMDNW12`
    let failed = |error: &dyn std::fmt::Display| {
        format!("ファイルを開けませんでした。\n{}\n{error}", path.display())
    };
    let file = open_file(path, &options).map_err(|error| failed(&error))?;
    let end = file.document.end_pos().map_err(|error| failed(&error))?;
    file.document
        .text_between(Pos::new(0, 0), end)
        .map_err(|error| failed(&error))
}

register_ui_command!(InsFile);
