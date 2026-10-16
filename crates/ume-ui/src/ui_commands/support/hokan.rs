//! `F_HOKAN`: input completion (入力補完). Takes the word before the caret and shows the words of
//! the document that start with it; with one candidate it completes at once, with none it only says so.
//!
//! Sakura: `CViewCommander::Command_HOKAN` (`sakura_core/cmd/CViewCommander_Support.cpp`).
//! See [`crate::support::completion`] for the list and its keys.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Hokan;

impl UiCommand for Hokan {
    const ID: &'static str = "hokan";
    const SAKURA: &'static str = "F_HOKAN";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.start_completion(cx);
        Ok(())
    }
}

register_ui_command!(Hokan);
