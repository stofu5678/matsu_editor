//! The jumps to the next and the previous DIFF mark: they follow the search option 先頭（末尾）
//! から再検索 and report in the status bar and a message box, as the jumps to bookmarks do.
//!
//! Sakura: `Command_Diff_Next` and `Command_Diff_Prev` (`sakura_core/cmd/CViewCommander_Diff.cpp`).
//! The rules of where they stop are in `ume_commands::context`.

use gpui::{Context, Window};
use ume_commands::context::DiffJump;
use ume_core::search::Direction;

use crate::search::state::{not_found_status, wrapped_message};
use crate::view::EditorView;

/// `STR_DIFF_NEXT_NOT_FOUND` / `STR_DIFF_PREV_NOT_FOUND` (`sakura_core/sakura_rc.rc`), the text of
/// the box.
pub fn diff_not_found_message(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "前方(↓) に差分が見つかりません。",
        Direction::Backward => "後方(↑) に差分が見つかりません。",
    }
}

impl EditorView {
    /// `F_DIFF_NEXT` / `F_DIFF_PREV`: moves to the next (previous) marked line. A search that went
    /// on from the other end says so in the status bar; finding nothing says so there and in a
    /// box (if the option 見つからないときにメッセージを表示 is on, as for `AlertNotFound` with
    /// `bReplaceAll = false`).
    pub fn jump_to_diff(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let options = self.search.options();
        let jump = self.run_with(cx, |c| c.jump_to_diff(direction, options.wrap));
        match jump {
            Ok(DiffJump::Found { wrapped }) => {
                self.notice = wrapped.then(|| wrapped_message(direction).to_owned());
                cx.notify();
            }
            Ok(DiffJump::NotFound) => {
                self.notice = Some(not_found_status(direction).to_owned());
                cx.notify();
                if options.notify_not_found {
                    self.info_box(diff_not_found_message(direction), window, cx);
                }
            }
            Err(e) => {
                self.notice = Some(format!("diff: {e}"));
                cx.notify();
            }
        }
    }
}
