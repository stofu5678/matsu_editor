//! `F_FILESAVEAS_DIALOG`: saves under a name chosen in the save dialog.
//!
//! Sakura: `Command_FILESAVEAS_DIALOG` (`sakura_core/cmd/CViewCommander_File.cpp`). Arguments, as
//! in the macro `S_FileSaveAsDialog`: the default file name (the dialog offers it, as `FileOpen`'s default name; a folder in it is where the dialog starts), the encoding number
//! (99 or none: keep the document's), the line ending (0 or none: do not change, 1 CRLF, 2 LF,
//! 3 CR). The save dialog has the encoding, the BOM and the line ending in itself, as Sakura's
//! does (`file_dialog::save_fields`); where it cannot (GPUI's dialog), a small dialog of the window
//! asks for them after the path (`file_ops::save_as_dialog`). A macro that passes the encoding or
//! the line ending skips both.

use crate::file_ops::args::{encoding_arg, eol_arg};
use crate::file_ops::save::{SaveRequest, save};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, CommandError, Result};
use matsu_config::dialog_dir::split_default_name;

#[derive(Default)]
pub struct FileSaveAsDialog;

impl UiCommand for FileSaveAsDialog {
    const ID: &'static str = "file_save_as_dialog";
    const SAKURA: &'static str = "F_FILESAVEAS_DIALOG";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let encoding = encoding_arg(args, 1)?;
        let eol = eol_arg(args, 2)?;
        // `Command_FILESAVEAS_DIALOG(fileNameDef, ...)`: an empty text is no name
        let given = match args.first() {
            None => None,
            Some(Arg::Text(text)) => {
                (!text.is_empty()).then(|| split_default_name(text, &|dir| dir.is_dir()))
            }
            Some(_) => {
                return Err(CommandError::BadArgs(
                    "FileSaveAsDialog needs a text as argument 1".to_owned(),
                ));
            }
        };
        let _ = view;
        start(window, cx, async move |view, cx| {
            let request = SaveRequest {
                dialog: true,
                given,
                encoding,
                eol,
                ..SaveRequest::default()
            };
            save(&view, cx, request).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSaveAsDialog);
