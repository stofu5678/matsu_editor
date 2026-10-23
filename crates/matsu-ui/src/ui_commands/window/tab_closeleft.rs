//! `F_TAB_CLOSELEFT`: closes the tabs to the left of this one (左をすべて閉じる).
//!
//! Sakura: `CViewCommander::Command_TAB_CLOSELEFT` (`sakura_core/cmd/CViewCommander_Window.cpp`)
//! asks every window of the group on the left of this one to close with `RequestCloseEditor`
//! when the tab bar is shown: each document with changes asks "save?" and "cancel" stops it.

use crate::file_ops::start;
use crate::tabs::flows;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TabCloseleft;

impl UiCommand for TabCloseleft {
    const ID: &'static str = "tab_closeleft";
    const SAKURA: &'static str = "F_TAB_CLOSELEFT";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let current = window.window_handle();
        start(window, cx, async move |_view, cx| {
            flows::close_side(cx, current, true).await;
        });
        Ok(())
    }
}

register_ui_command!(TabCloseleft);
