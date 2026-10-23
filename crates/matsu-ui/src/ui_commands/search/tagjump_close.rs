//! `F_TAGJUMP_CLOSE`: jumps like `F_TAGJUMP`, then closes the window it came from.
//!
//! Sakura: `Command_TAGJUMP(true)` (`sakura_core/cmd/CViewCommander_TagJump.cpp`); the flow is in
//! `file_ops::tag_jump`.

use crate::file_ops::start;
use crate::file_ops::tag_jump::tag_jump;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TagjumpClose;

impl UiCommand for TagjumpClose {
    const ID: &'static str = "tagjump_close";
    const SAKURA: &'static str = "F_TAGJUMP_CLOSE";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = view;
        start(window, cx, async move |view, cx| {
            tag_jump(&view, cx, true).await;
        });
        Ok(())
    }
}

register_ui_command!(TagjumpClose);
