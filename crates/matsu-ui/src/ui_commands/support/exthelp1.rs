//! `F_EXTHELP1`: opens the external help file at the word under the caret or the selection.
//!
//! Sakura: `CViewCommander::Command_EXTHELP1` (`sakura_core/cmd/CViewCommander_Support.cpp`). The
//! file is the one of the document type, else the one of the common settings (the page 支援); with
//! none the function asks whether to set it. See [`crate::support::help`] for what is opened and
//! how.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Exthelp1;

impl UiCommand for Exthelp1 {
    const ID: &'static str = "exthelp1";
    const SAKURA: &'static str = "F_EXTHELP1";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.ext_help1(window, cx);
        Ok(())
    }
}

register_ui_command!(Exthelp1);
