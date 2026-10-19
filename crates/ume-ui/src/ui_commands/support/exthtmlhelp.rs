//! `F_EXTHTMLHELP` (the macro `ExtHtmlHelp(file, keyword)`): opens the external HTML help file at a
//! word.
//!
//! Sakura: `CViewCommander::Command_EXTHTMLHELP` (`sakura_core/cmd/CViewCommander_Support.cpp`).
//! A key or a menu gives no arguments: the file is the one of the settings and the word is the one
//! under the caret or the selection. A macro can give either or both; an empty one counts as
//! missing (`CMacro::HandleCommand` passes the two texts on). See [`crate::support::help`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Exthtmlhelp;

/// The text of argument `i`, if the macro gave one.
fn text(args: &[Arg], i: usize) -> Option<&str> {
    match args.get(i) {
        Some(Arg::Text(text)) => Some(text),
        _ => None,
    }
}

impl UiCommand for Exthtmlhelp {
    const ID: &'static str = "exthtmlhelp";
    const SAKURA: &'static str = "F_EXTHTMLHELP";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.ext_html_help(text(args, 0), text(args, 1), window, cx);
        Ok(())
    }
}

register_ui_command!(Exthtmlhelp);
