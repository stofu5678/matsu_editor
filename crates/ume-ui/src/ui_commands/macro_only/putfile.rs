//! `F_PUTFILE` (the macro `PutFile(file, code, option)`): writes the document, or the selected
//! text, to a file without making it the file of the document (作業中ファイルの一時出力).
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the file
//! name is missing and reads the code and the option as numbers (a missing one is 0, so the code
//! is Shift_JIS); `CViewCommander::Command_PUTFILE` (`sakura_core/cmd/CViewCommander_File.cpp`)
//! returns without a message when the name is empty, takes the encoding of the document for code
//! 99 (`CODE_AUTODETECT`), and writes a BOM only when the encoding has one and the document has
//! one. Option 0x01 writes the selected text; without a selection no bytes are written (so no
//! file is made, and one that exists stays; with a BOM the BOM alone is written). Without it the
//! whole document is written, with its line endings as they are, and characters that the
//! encoding cannot write become `?`. A failure is shown to the user and the macro goes on (the
//! macro ignores the result).
//!
//! The file is replaced as a whole (`ume_core::file::write_bytes`, ADR-0005): a failed write
//! leaves the old file. Sakura truncates it and writes into it.

use std::path::PathBuf;

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};
use ume_core::encoding::{AUTODETECT_CODE, Encoding};
use ume_core::file::{FileError, Preserve, SaveOptions, no_progress, write_bytes};

use super::{int_arg, text_arg};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

/// The option that writes the selected text.
const SELECTION: i64 = 0x01;

#[derive(Default)]
pub struct PutFile;

/// The encoding of a code argument: Sakura's number, or the document's for 99.
pub(super) fn encoding_of(code: i64, document: Encoding) -> Option<Encoding> {
    if code == i64::from(AUTODETECT_CODE) {
        return Some(document);
    }
    i32::try_from(code)
        .ok()
        .and_then(Encoding::from_sakura_code)
}

impl UiCommand for PutFile {
    const ID: &'static str = "putfile";
    const SAKURA: &'static str = "F_PUTFILE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let name = text_arg("PutFile", args, 0)?;
        let code = int_arg("PutFile", args, 1, 0)?;
        let option = int_arg("PutFile", args, 2, 0)?;
        if name.is_empty() {
            return Ok(());
        }
        let (document_encoding, document_bom) = {
            let file = view.document().read(cx).file();
            (file.encoding, file.bom)
        };
        let encoding = encoding_of(code, document_encoding).ok_or_else(|| {
            CommandError::BadArgs(format!("PutFile: {code} is not an encoding number"))
        })?;
        // `UseBom`: only the encodings that have a BOM, and then the BOM of the document.
        let bom = encoding.can_have_bom() && document_bom;
        let path = PathBuf::from(name);
        let written = if option & SELECTION != 0 {
            put_selection(view, &path, encoding, bom, cx)
        } else {
            put_document(view, path.clone(), encoding, bom, cx)
        };
        if let Err(message) = written {
            // Sakura shows a warning and the macro goes on.
            view.macro_failed(message, window, cx);
        }
        Ok(())
    }
}

/// The error of a write, in the words of Sakura's `STR_ERR_DLGEDITVWCMDNW11`.
fn write_error(path: &std::path::Path, error: &dyn std::fmt::Display) -> String {
    format!(
        "ファイルの書き込みに失敗しました。\n{}\n{error}",
        path.display()
    )
}

/// `Command_PUTFILE` with the whole document.
fn put_document(
    view: &mut EditorView,
    path: PathBuf,
    encoding: Encoding,
    bom: bool,
    cx: &mut Context<EditorView>,
) -> std::result::Result<(), String> {
    let mut options = SaveOptions::for_encoding(encoding);
    options.bom = bom;
    // the characters that the encoding cannot write become `?`
    options.allow_unmappable = true;
    options.preserve = Preserve::default();
    let shown = path.clone();
    view.document()
        .update(cx, |shared, _| {
            shared.put_file(path, &options, &mut no_progress)
        })
        .map(|_| ())
        .map_err(|error: FileError| write_error(&shown, &error))
}

/// `Command_PUTFILE` with the selected text (`GetSelectedDataSimple`: its line endings as they
/// are; a box is its rows, each with CR LF).
fn put_selection(
    view: &mut EditorView,
    path: &std::path::Path,
    encoding: Encoding,
    bom: bool,
    cx: &mut Context<EditorView>,
) -> std::result::Result<(), String> {
    let text = if view.cursor().is_box() {
        view.run_with(cx, |command_cx| command_cx.box_text())
            .ok()
            .flatten()
    } else {
        view.cursor()
            .selection()
            .and_then(|(from, to)| view.document().read(cx).doc().text_between(from, to).ok())
    }
    .unwrap_or_default();
    let mut bytes = Vec::new();
    if bom {
        bytes.extend_from_slice(encoding.bom());
    }
    // A character that the encoding cannot write is written the way Sakura writes it; the
    // report of them is not needed here.
    let _ = encoding.codec().encode(&text, &mut bytes);
    if bytes.is_empty() {
        return Ok(());
    }
    write_bytes(path, &bytes, Preserve::default()).map_err(|error| write_error(path, &error))
}

register_ui_command!(PutFile);
