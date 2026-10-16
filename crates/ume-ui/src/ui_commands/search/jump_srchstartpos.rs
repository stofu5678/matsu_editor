//! `F_JUMP_SRCHSTARTPOS`: goes back to where the last search from the panel began.
//!
//! Sakura: `CViewCommander::Command_JUMP_SRCHSTARTPOS` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct JumpSrchstartpos;

impl UiCommand for JumpSrchstartpos {
    const ID: &'static str = "jump_srchstartpos";
    const SAKURA: &'static str = "F_JUMP_SRCHSTARTPOS";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = window;
        view.jump_to_search_start(cx);
        Ok(())
    }
}

register_ui_command!(JumpSrchstartpos);
