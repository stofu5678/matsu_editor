//! `F_EXITALLEDITORS`: closes every window and quits (編集の全終了).
//!
//! Sakura closes the editor windows and, for `F_EXITALL`, the tray process too. This program has
//! no tray process, so both are the same: every window with unsaved changes asks, and a
//! cancel stops the whole thing.

use crate::file_ops::close::exit_all;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct ExitAllEditors;

impl UiCommand for ExitAllEditors {
    const ID: &'static str = "exit_all_editors";
    const SAKURA: &'static str = "F_EXITALLEDITORS";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            let _ = view;
            exit_all(cx).await;
        });
        Ok(())
    }
}

register_ui_command!(ExitAllEditors);
