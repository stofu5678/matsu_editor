//! `F_TAGS_MAKE`: makes a tags file by running ctags (タグファイルの作成).
//!
//! Sakura: `Command_TagsMake` (`sakura_core/cmd/CViewCommander_TagJump.cpp`); the flow is in `tags`.
//! ctags is not part of this program: the one the user installed is run.

use crate::file_ops::start;
use crate::tags::make_tags;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TagsMake;

impl UiCommand for TagsMake {
    const ID: &'static str = "tags_make";
    const SAKURA: &'static str = "F_TAGS_MAKE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        start(window, cx, async move |view, cx| {
            make_tags(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(TagsMake);
