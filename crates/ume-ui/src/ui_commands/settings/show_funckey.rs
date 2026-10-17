//! `F_SHOWFUNCKEY`: `Command_SHOWFUNCKEY`: shows the function key bar, or hides it, in every window.
//!
//! Sakura flips `m_bDispFUNCKEYWND` of the common settings, lays the bar out, and tells all the
//! editors (`MYWM_BAR_CHANGE_NOTIFY`, `MYBCN_FUNCKEY`) (`sakura_core/cmd/CViewCommander_Settings.cpp`).
//! Here the setting is `bDispFUNCKEYWND` of `[Common]`; the windows watch it (`crate::bars`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ShowFunckey;

impl UiCommand for ShowFunckey {
    const ID: &'static str = "show_funckey";
    const SAKURA: &'static str = "F_SHOWFUNCKEY";

    fn run(
        &self,
        _: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::bars::update(cx, |bars| bars.funckeys = !bars.funckeys);
        Ok(())
    }
}

register_ui_command!(ShowFunckey);
