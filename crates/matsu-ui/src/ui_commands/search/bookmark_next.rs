//! `F_BOOKMARK_NEXT`: moves to the next bookmarked line (F2).
//!
//! Sakura: `CViewCommander::Command_BOOKMARK_NEXT` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`).
//! It is a plain move (it extends the selection while the selection is locked) and does not add
//! to the jump history. The rules are in `EditorView::jump_to_bookmark`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct BookmarkNext;

impl UiCommand for BookmarkNext {
    const ID: &'static str = "bookmark_next";
    const SAKURA: &'static str = "F_BOOKMARK_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_bookmark(Direction::Forward, window, cx);
        Ok(())
    }
}

register_ui_command!(BookmarkNext);
