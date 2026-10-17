//! `F_DIRECT_TAGJUMP`: jumps to the tag that the word under the caret names, by the tags file.
//!
//! Sakura: `Command_TagJumpByTagsFileMsg(true)` (`sakura_core/cmd/CViewCommander_TagJump.cpp`); the
//! flow is in `tags`. With several hits the list asks which one.

use crate::file_ops::start;
use crate::tags::direct_tag_jump;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct DirectTagjump;

impl UiCommand for DirectTagjump {
    const ID: &'static str = "direct_tagjump";
    const SAKURA: &'static str = "F_DIRECT_TAGJUMP";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        start(window, cx, async move |view, cx| {
            direct_tag_jump(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(DirectTagjump);
