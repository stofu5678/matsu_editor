//! `F_BOOKMARK_PATTERN`: bookmarks every line that matches the current search pattern (the
//! 該当行マーク button of the find and replace dialogs).
//!
//! Sakura: `CViewCommander::Command_BOOKMARK_PATTERN` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`)
//! and `CBookmarkManager::MarkSearchWord`. The pattern and options are the ones of the search
//! panel, whichever kind of search they say (plain, whole word; the regular expression option is
//! not in the panel yet). Not like Sakura's macro: the arguments of `BookmarkPattern` (a pattern
//! and the option flags) are not read, as for `search_next`; the current pattern is used.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct BookmarkPattern;

impl UiCommand for BookmarkPattern {
    const ID: &'static str = "bookmark_pattern";
    const SAKURA: &'static str = "F_BOOKMARK_PATTERN";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.mark_matching_lines(cx);
        Ok(())
    }
}

register_ui_command!(BookmarkPattern);
