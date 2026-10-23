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
use matsu_commands::command::{Arg, Result};
use matsu_commands::context::CommandContext;
use matsu_core::buffer::Document;
use matsu_core::encoding::Encoding;
use matsu_core::file::{OpenOptions, TextFile, open_file};

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
        let file = match open(Path::new(name), encoding) {
            Ok(file) => file,
            Err(message) => {
                view.macro_failed(message, window, cx);
                return Ok(());
            }
        };
        // `bBeforeTextSelected` and `ptFrom`
        let from = view.cursor().selection().map(|(from, _)| from);
        let doc = file.document;
        if doc.line_count() == 0 {
            return Ok(());
        }
        view.run_with(cx, |command_cx| {
            command_cx.begin_block();
            let result = insert_lines(command_cx, &doc).and_then(|()| match from {
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

/// How much text one insertion takes. Sakura inserts line by line; a few lines at a time is the
/// same and does not make a text of the whole file (CLAUDE.md: the document is read by ranges).
pub(super) const CHUNK_BYTES: usize = 64 * 1024;

/// Inserts the lines of `doc` at the caret, with their line endings as they are.
fn insert_lines(cx: &mut CommandContext<'_>, doc: &Document) -> Result {
    let mut chunk = String::new();
    for index in 0..doc.line_count() {
        let Some(line) = doc.line(index)? else { break };
        chunk.push_str(&line.text);
        if let Some(eol) = line.eol {
            chunk.push_str(eol.as_str());
        }
        // A box selection takes the first line of a text only (`insert_into_box`), so a text of
        // several lines would lose the rest: with a box, each line is inserted by itself, as
        // Sakura does for every file.
        if chunk.len() >= CHUNK_BYTES || cx.cursor().is_box() {
            cx.insert_text(&chunk)?;
            chunk.clear();
        }
    }
    if !chunk.is_empty() {
        cx.insert_text(&chunk)?;
    }
    Ok(())
}

/// Opens the file in `encoding` (the BOM is dropped). The error is the text to show.
fn open(path: &Path, encoding: Encoding) -> std::result::Result<TextFile, String> {
    let options = OpenOptions {
        encoding: Some(encoding),
        ..OpenOptions::default()
    };
    // `STR_GREP_ERR_FILEOPEN` and `STR_ERR_DLGEDITVWCMDNW12`
    open_file(path, &options)
        .map_err(|error| format!("ファイルを開けませんでした。\n{}\n{error}", path.display()))
}

register_ui_command!(InsFile);
