//! `F_TAGJUMP_KEYWORD`: asks for a keyword and jumps to a tag of the tags file (キーワードを指定してタグ
//! ジャンプ).
//!
//! Sakura: `Command_TagJumpByTagsFileKeyword` (`sakura_core/cmd/CViewCommander_TagJump.cpp`); the
//! macro `KeywordTagJump(keyword)` gives the keyword to start the dialog with
//! (`sakura_core/macro/CMacro.cpp`); the keyword may be left out. The flow is in `tags`.

use crate::file_ops::start;
use crate::tags::keyword_tag_jump;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct TagjumpKeyword;

impl UiCommand for TagjumpKeyword {
    const ID: &'static str = "tagjump_keyword";
    const SAKURA: &'static str = "F_TAGJUMP_KEYWORD";

    fn run(
        &self,
        _view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let keyword = match args.first() {
            Some(Arg::Text(text)) => Some(text.clone()),
            _ => None,
        };
        start(window, cx, async move |view, cx| {
            keyword_tag_jump(&view, cx, keyword).await;
        });
        Ok(())
    }
}

register_ui_command!(TagjumpKeyword);
