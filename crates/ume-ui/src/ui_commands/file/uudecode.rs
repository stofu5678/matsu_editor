//! `F_UUDECODE`: decodes the selected text as uuencoded data and saves the bytes to a file.
//!
//! Sakura: `Command_UUDECODE` (`sakura_core/cmd/CViewCommander_Convert.cpp`). Without a
//! selection, or if the text is not uuencoded (`CDecode_UuDecode`, ported in
//! `ume_commands::convert::code::uudecode`), nothing happens. Otherwise the save dialog asks for
//! the file, starting with the name of the `begin` line, and the bytes are written to it. A
//! rectangular selection does nothing yet.

use crate::file_ops::save_decoded::{save_bytes_as, selected_text};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_commands::convert::code::uudecode;

#[derive(Default)]
pub struct Uudecode;

impl UiCommand for Uudecode {
    const ID: &'static str = "uudecode";
    const SAKURA: &'static str = "F_UUDECODE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let Some(decoded) = selected_text(view, cx).and_then(|text| uudecode(&text)) else {
            return Ok(());
        };
        save_bytes_as(view, decoded.file_name, decoded.data, window, cx);
        Ok(())
    }
}

register_ui_command!(Uudecode);
