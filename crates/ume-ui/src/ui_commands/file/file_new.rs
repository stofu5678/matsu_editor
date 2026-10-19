//! `F_FILENEW`: a new untitled document in a new window, or in a new tab in tab mode.
//!
//! Sakura: `CViewCommander::Command_FILENEW` (`sakura_core/cmd/CViewCommander_File.cpp`) starts a
//! new editor window with no file, in the group of this window (`CControlTray::OpenNewEditor`,
//! `sakura_core/_main/CControlTray.cpp`): with the tab bar shown and the windows grouped that is
//! a new tab here.

use crate::document::SharedDocument;
use crate::tabs::flows::open_document;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileNew;

impl UiCommand for FileNew {
    const ID: &'static str = "file_new";
    const SAKURA: &'static str = "F_FILENEW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let from = window.window_handle();
        view.defer_in_window(window, cx, move |window, app| {
            let opened = open_document(
                app,
                Some(window),
                from,
                false,
                SharedDocument::empty(),
                None,
            );
            crate::macros::run_opened_in_new(app, opened.ok());
        })
    }
}

register_ui_command!(FileNew);
