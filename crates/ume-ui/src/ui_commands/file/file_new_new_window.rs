//! `F_FILENEW_NEWWINDOW`: a new untitled document in a new window.
//!
//! Sakura: `Command_FILENEW_NEWWINDOW` (`sakura_core/cmd/CViewCommander_File.cpp`) differs from
//! `F_FILENEW` in what it does with tabs: it passes `bNewWindow`, which gives the editor a group
//! of its own (`CControlTray::OpenNewEditor`), so the document gets a new window even in tab mode.

use crate::document::SharedDocument;
use crate::tabs::flows::open_document;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileNewNewWindow;

impl UiCommand for FileNewNewWindow {
    const ID: &'static str = "file_new_new_window";
    const SAKURA: &'static str = "F_FILENEW_NEWWINDOW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let from = window.window_handle();
        view.defer_in_window(window, cx, move |window, app| {
            let opened =
                open_document(app, Some(window), from, true, SharedDocument::empty(), None);
            crate::macros::run_opened_in_new(app, opened.ok());
        })
    }
}

register_ui_command!(FileNewNewWindow);
