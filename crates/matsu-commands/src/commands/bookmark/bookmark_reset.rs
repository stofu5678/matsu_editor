//! `F_BOOKMARK_RESET`: removes every bookmark of the document (Ctrl+Shift+F2).
//!
//! Sakura: `CViewCommander::Command_BOOKMARK_RESET` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`),
//! which calls `CBookmarkManager::ResetAllBookMark` (`sakura_core/docplus/CBookmarkManager.cpp`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct BookmarkReset;

impl Command for BookmarkReset {
    const ID: &'static str = "bookmark_reset";
    const SAKURA: &'static str = "F_BOOKMARK_RESET";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.clear_bookmarks();
        Ok(())
    }
}

register_command!(BookmarkReset);

#[cfg(test)]
mod tests {
    use crate::testing::Editor;

    fn marked(e: &Editor) -> Vec<usize> {
        e.model().bookmarks().iter().collect()
    }

    #[test]
    fn removes_every_bookmark() {
        let mut e = Editor::new("a\n日本\r\nc|");
        e.run("bookmark_set").run("go_file_top").run("bookmark_set");
        assert_eq!(marked(&e), [0, 2]);
        e.run("bookmark_reset");
        assert!(marked(&e).is_empty());
        assert!(e.changes().bookmarks_changed);
        // the caret stays, and there is nothing to do the second time
        e.assert_marked("|a\n日本\r\nc");
        e.run("bookmark_reset");
        assert!(marked(&e).is_empty());
    }

    #[test]
    fn works_on_an_empty_document() {
        let mut e = Editor::new("|");
        e.run("bookmark_reset");
        assert!(marked(&e).is_empty());
    }

    #[test]
    fn is_not_an_edit() {
        let mut e = Editor::new("a|");
        e.run("bookmark_set").run("bookmark_reset");
        assert!(!e.changes().text_changed());
        assert!(!e.model().is_modified());
    }
}
