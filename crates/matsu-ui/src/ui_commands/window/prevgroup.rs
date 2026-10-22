//! `F_PREVGROUP`: brings the previous group (window) to the front (前のグループ).
//!
//! Sakura: `CViewCommander::Command_PREVGROUP` calls `CTabWnd::PrevGroup`
//! (`sakura_core/window/CTabWnd.cpp`); see `F_NEXTGROUP`.

use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Prevgroup;

impl UiCommand for Prevgroup {
    const ID: &'static str = "prevgroup";
    const SAKURA: &'static str = "F_PREVGROUP";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        flows::go_to_group(cx, window.window_handle(), false);
        Ok(())
    }
}

register_ui_command!(Prevgroup);
