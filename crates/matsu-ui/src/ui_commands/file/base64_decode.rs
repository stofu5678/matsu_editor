//! `F_BASE64DECODE`: decodes the selected text as Base64 and saves the bytes to a file.
//!
//! Sakura: `Command_BASE64DECODE` (`sakura_core/cmd/CViewCommander_Convert.cpp`). Without a
//! selection, or if the text is not Base64 (`CDecode_Base64Decode`, ported in
//! `matsu_commands::convert::code::base64_decode`), nothing happens. Otherwise the save dialog asks
//! for the file, which starts with no name, and the bytes are written to it. A rectangular
//! selection does nothing yet.

use crate::file_ops::save_decoded::{save_bytes_as, selected_text};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_commands::convert::code::base64_decode;

#[derive(Default)]
pub struct Base64Decode;

impl UiCommand for Base64Decode {
    const ID: &'static str = "base64_decode";
    const SAKURA: &'static str = "F_BASE64DECODE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let Some(data) = selected_text(view, cx).and_then(|text| base64_decode(&text)) else {
            return Ok(());
        };
        save_bytes_as(view, String::new(), data, window, cx);
        Ok(())
    }
}

register_ui_command!(Base64Decode);
