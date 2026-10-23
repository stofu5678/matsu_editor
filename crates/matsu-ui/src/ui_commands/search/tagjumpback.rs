//! `F_TAGJUMPBACK`: goes back to the window and place of the last tag jump.
//!
//! Sakura: `Command_TAGJUMPBACK` (`sakura_core/cmd/CViewCommander_TagJump.cpp`); the flow is in
//! `file_ops::tag_jump`.

use crate::file_ops::start;
use crate::file_ops::tag_jump::tag_jump_back;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TagjumpBack;

impl UiCommand for TagjumpBack {
    const ID: &'static str = "tagjumpback";
    const SAKURA: &'static str = "F_TAGJUMPBACK";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            tag_jump_back(&view, cx);
        });
        Ok(())
    }
}

register_ui_command!(TagjumpBack);
