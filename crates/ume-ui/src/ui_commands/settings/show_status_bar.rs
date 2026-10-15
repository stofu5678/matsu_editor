//! `F_SHOWSTATUSBAR`: `Command_SHOWSTATUSBAR`: shows the status bar, or hides it. The setting is `bDispSTATUSBAR`; here it is kept for the session (see `crate::settings`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ShowStatusBar;

impl UiCommand for ShowStatusBar {
    const ID: &'static str = "show_status_bar";
    const SAKURA: &'static str = "F_SHOWSTATUSBAR";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.toggle_status_bar(cx);
        Ok(())
    }
}

register_ui_command!(ShowStatusBar);
