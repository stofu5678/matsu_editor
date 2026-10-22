//! `F_WINCLOSE`: closes the window, after asking about unsaved changes.
//!
//! Sakura: `CViewCommander::Command_WINCLOSE` (`sakura_core/cmd/CViewCommander_Window.cpp`) and
//! `CEditDoc::OnFileClose` for the question.

use crate::file_ops::close::confirm_and_close_window;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct WinClose;

impl UiCommand for WinClose {
    const ID: &'static str = "win_close";
    const SAKURA: &'static str = "F_WINCLOSE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            confirm_and_close_window(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(WinClose);
