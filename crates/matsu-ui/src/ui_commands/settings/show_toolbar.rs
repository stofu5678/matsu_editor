//! `F_SHOWTOOLBAR`: `Command_SHOWTOOLBAR`: shows the toolbar, or hides it, in every window.
//!
//! Sakura flips `m_bDispTOOLBAR` of the common settings, lays the bar out, and tells all the editors
//! (`MYWM_BAR_CHANGE_NOTIFY`, `MYBCN_TOOLBAR`) (`sakura_core/cmd/CViewCommander_Settings.cpp`). Here
//! the setting is `bDispTOOLBAR` of `[Common]`; the windows watch it (`crate::bars`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ShowToolbar;

impl UiCommand for ShowToolbar {
    const ID: &'static str = "show_toolbar";
    const SAKURA: &'static str = "F_SHOWTOOLBAR";

    fn run(
        &self,
        _: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::bars::update(cx, |bars| bars.toolbar = !bars.toolbar);
        Ok(())
    }
}

register_ui_command!(ShowToolbar);
