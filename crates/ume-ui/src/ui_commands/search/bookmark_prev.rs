//! `F_BOOKMARK_PREV`: moves to the previous bookmarked line (Shift+F2).
//!
//! Sakura: `CViewCommander::Command_BOOKMARK_PREV` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`).
//! See `bookmark_next` for the rules.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::search::Direction;

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct BookmarkPrev;

impl UiCommand for BookmarkPrev {
    const ID: &'static str = "bookmark_prev";
    const SAKURA: &'static str = "F_BOOKMARK_PREV";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.jump_to_bookmark(Direction::Backward, window, cx);
        Ok(())
    }
}

register_ui_command!(BookmarkPrev);
