//! `F_SEARCH_BOX`: moves the focus into the search box of the toolbar.
//!
//! Sakura: `CViewCommander::Command_SEARCH_BOX` (`sakura_core/cmd/CViewCommander_Search.cpp`) calls
//! `CMainToolBar::SetFocusSearchBox`, which does nothing when the toolbar has no search box.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SearchBox;

impl UiCommand for SearchBox {
    const ID: &'static str = "search_box";
    const SAKURA: &'static str = "F_SEARCH_BOX";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if let Some(host) = view.split_host.clone() {
            // The window may be in the middle of an update that ran this function.
            window.defer(cx, move |window, app| {
                host.update(app, |workspace, cx| workspace.focus_search_box(window, cx))
                    .ok();
            });
        }
        Ok(())
    }
}

register_ui_command!(SearchBox);
