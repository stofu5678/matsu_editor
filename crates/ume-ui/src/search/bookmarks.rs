//! The bookmark functions that need the search state or the window: the jumps (which follow the
//! option 先頭（末尾）から再検索 and report in the status bar and a message box) and the marking of
//! the lines that match the current pattern.
//!
//! Sakura: `Command_BOOKMARK_NEXT`, `Command_BOOKMARK_PREV`, `Command_BOOKMARK_PATTERN`
//! (`sakura_core/cmd/CViewCommander_Bookmark.cpp`) and `CBookmarkManager::MarkSearchWord`
//! (`sakura_core/docplus/CBookmarkManager.cpp`). The rules of setting and following bookmarks are
//! in `ume_commands::context` and `ume_core::bookmarks`.

use gpui::{Context, Window};
use ume_commands::context::BookmarkJump;
use ume_core::search::{Direction, find_in_line};

use super::state::{not_found_status, wrapped_message};
use crate::view::EditorView;

/// `STR_BOOKMARK_NEXT_NOT_FOUND` / `STR_BOOKMARK_PREV_NOT_FOUND` (`sakura_core/sakura_rc.rc`),
/// the text of the box.
pub fn bookmark_not_found_message(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "前方(↓) にブックマークが見つかりません。",
        Direction::Backward => "後方(↑) にブックマークが見つかりません。",
    }
}

impl EditorView {
    /// `F_BOOKMARK_NEXT` / `F_BOOKMARK_PREV`: moves to the next (previous) bookmarked line. A
    /// search that went on from the other end says so in the status bar; finding nothing says so
    /// there and in a box (if the option 見つからないときにメッセージを表示 is on, as for
    /// `AlertNotFound` with `bReplaceAll = false`).
    pub fn jump_to_bookmark(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let options = self.search.options();
        let jump = self.run_with(cx, |c| c.jump_to_bookmark(direction, options.wrap));
        match jump {
            Ok(BookmarkJump::Found { wrapped }) => {
                self.notice = wrapped.then(|| wrapped_message(direction).to_owned());
                cx.notify();
            }
            Ok(BookmarkJump::NotFound) => {
                self.notice = Some(not_found_status(direction).to_owned());
                cx.notify();
                if options.notify_not_found {
                    self.info_box(bookmark_not_found_message(direction), window, cx);
                }
            }
            Err(e) => {
                self.notice = Some(format!("bookmark: {e}"));
                cx.notify();
            }
        }
    }

    /// `F_BOOKMARK_PATTERN`: bookmarks every line that has a match of the current pattern (and
    /// keeps the bookmarks that are there). With no pattern nothing happens.
    pub fn mark_matching_lines(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.report_pattern_error(window, cx) {
            return;
        }
        let Some(matcher) = self.search.matcher().cloned() else {
            return;
        };
        let lines: Result<Vec<usize>, String> = {
            let doc = self.document.read(cx).doc();
            let mut lines = Vec::new();
            let mut failure = None;
            for i in 0..doc.line_count() {
                // Sakura matches the line with its line ending (`CBookmarkManager::MarkSearchWord`
                // gets `GetDocLineStrWithEOL`), so `\n` or `\s$` marks a line too.
                let hit = find_in_line(doc, &*matcher, i, 0, false).map_err(|e| e.to_string());
                match hit {
                    Ok(Some(_)) => lines.push(i),
                    Ok(None) => {}
                    Err(e) => {
                        failure = Some(e);
                        break;
                    }
                }
            }
            failure.map_or(Ok(lines), Err)
        };
        match lines {
            Ok(lines) => {
                self.run_with(cx, |c| c.add_bookmarks(lines));
            }
            Err(e) => {
                self.notice = Some(format!("検索できなかった: {e}"));
                cx.notify();
            }
        }
    }
}
