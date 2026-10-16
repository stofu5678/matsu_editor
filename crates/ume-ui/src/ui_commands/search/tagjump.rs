//! `F_TAGJUMP`: jumps to the file and place that the caret line names.
//!
//! Sakura: `Command_TAGJUMP` (`sakura_core/cmd/CViewCommander_TagJump.cpp`); the flow is in
//! `file_ops::tag_jump`.

use crate::file_ops::start;
use crate::file_ops::tag_jump::tag_jump;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Tagjump;

impl UiCommand for Tagjump {
    const ID: &'static str = "tagjump";
    const SAKURA: &'static str = "F_TAGJUMP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            tag_jump(&view, cx, false).await;
        });
        Ok(())
    }
}

register_ui_command!(Tagjump);
