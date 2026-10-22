//! `F_FILESAVEAS`: saves under the given name, without a dialog.
//!
//! Sakura: `Command_FILESAVEAS` (`sakura_core/cmd/CViewCommander_File.cpp`) takes the file name
//! and the line ending (argument 3). The encoding (argument 2) is not its business: the macro
//! handler of `F_FILESAVEAS` (`CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp`) sets
//! it on the document before the command runs, when it is a valid code and differs from the
//! document's (`SetDocumentEncoding`). The BOM it sets then is the default of the encoding the
//! document had (`CCodeTypeName( GetDocumentEncoding() ).IsBomDefOn()`), not of the new one: a
//! UTF-16 file saved as UTF-8 gets a BOM, a Shift_JIS file saved as UTF-16 gets none. The real
//! Sakura Editor does this (checked with the oracle cases `save-utf16le-to-utf8` and
//! `save-sjis-to-utf16le`), so this does too.
//!
//! A file with mixed line endings is saved as it is, with no question (`CCodeChecker::OnCheckSave`
//! in `sakura_core/env/CCodeChecker.cpp` asks when `m_bChkEnterAtEnd` is on, but the real Sakura
//! Editor v2.4.2 did not ask here and wrote every line with its own ending: the oracle case
//! `save-mixed-eol-keeps`; why it does not ask has not been found in the source). A macro cannot
//! answer a question.

use gpui::{Context, Window};
use ume_commands::command::CommandError;
use ume_commands::command::{Arg, Result};

use crate::file_ops::args::{encoding_arg, eol_arg, path_arg};
use crate::file_ops::save::{SaveRequest, save};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct FileSaveAs;

impl UiCommand for FileSaveAs {
    const ID: &'static str = "file_save_as";
    const SAKURA: &'static str = "F_FILESAVEAS";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let path = path_arg(args, 0)?
            .ok_or_else(|| CommandError::BadArgs("a file name is needed".to_owned()))?;
        let encoding = encoding_arg(args, 1)?;
        let eol = eol_arg(args, 2)?;
        // `CCodeTypeName( GetDocumentEncoding() ).IsBomDefOn()`: of the encoding before the change.
        let bom = encoding.map(|_| view.document().read(cx).file().encoding.bom_default_on());
        start(window, cx, async move |view, cx| {
            let request = SaveRequest {
                path: Some(path),
                encoding,
                bom,
                eol,
                ask_mixed_eol: false,
                ask_unmappable: false,
                ..SaveRequest::default()
            };
            save(&view, cx, request).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSaveAs);
