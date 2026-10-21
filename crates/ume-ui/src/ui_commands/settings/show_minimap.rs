//! `F_SHOWMINIMAP`: `Command_SHOWMINIMAP`: shows the minimap, or hides it, in every window.
//!
//! Sakura flips `m_bDispMiniMap` of the common settings, lays the minimap out, and tells all the
//! editors (`MYWM_BAR_CHANGE_NOTIFY`, `MYBCN_MINIMAP`) (`sakura_core/cmd/CViewCommander_Settings.cpp`).
//! Here the setting is `bDispMiniMap` of `[Common]`; the windows watch it (`crate::minimap`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ShowMiniMap;

impl UiCommand for ShowMiniMap {
    const ID: &'static str = "show_minimap";
    const SAKURA: &'static str = "F_SHOWMINIMAP";

    fn run(
        &self,
        _: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        crate::minimap::toggle(cx);
        Ok(())
    }
}

register_ui_command!(ShowMiniMap);
