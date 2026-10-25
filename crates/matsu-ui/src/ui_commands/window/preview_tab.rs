//! `F_PREVIEW_TAB`: shows the document in a preview tab (ADR-0024). MATSU Editor's own function;
//! Sakura has nothing like it. The window does the work (`tabs/preview_tab.rs`) after the command
//! ends, because it changes the tabs.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct PreviewTab;

impl UiCommand for PreviewTab {
    const ID: &'static str = "preview_tab";
    const SAKURA: &'static str = "F_PREVIEW_TAB";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.with_workspace(window, cx, |workspace, window, cx| {
            workspace.open_preview(window, cx);
        })
    }
}

register_ui_command!(PreviewTab);
