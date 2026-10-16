//! The jumps to the next and the previous modified line: they follow the search option 先頭（末尾）
//! から再検索 and report in the status bar and a message box, as the jumps to bookmarks do.
//!
//! Sakura: `Command_MODIFYLINE_NEXT` and `Command_MODIFYLINE_PREV`
//! (`sakura_core/cmd/CViewCommander_Cursor.cpp`). The rules of where they stop are in
//! `ume_commands::context`.

use gpui::{Context, Window};
use ume_commands::context::ModifiedLineJump;
use ume_core::search::Direction;

use super::state::{not_found_status, wrapped_message};
use crate::view::EditorView;

/// `STR_MODLINE_NEXT_NOT_FOUND` / `STR_MODLINE_PREV_NOT_FOUND` (`sakura_core/sakura_rc.rc`), the
/// text of the box.
pub fn modified_line_not_found_message(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "前方(↓) に変更行が見つかりません。",
        Direction::Backward => "後方(↑) に変更行が見つかりません。",
    }
}

impl EditorView {
    /// `F_MODIFYLINE_NEXT` / `F_MODIFYLINE_PREV` and the `_SEL` forms (`extend`): moves to the
    /// next (previous) edge of a block of modified lines. A search that went on from the other end
    /// says so in the status bar; finding nothing says so there and in a box (if the option
    /// 見つからないときにメッセージを表示 is on, as for `AlertNotFound` with `bReplaceAll = false`).
    pub fn jump_to_modified_line(
        &mut self,
        direction: Direction,
        extend: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let options = self.search.options();
        let jump = self.run_with(cx, |c| {
            c.jump_to_modified_line(direction, extend, options.wrap)
        });
        match jump {
            Ok(ModifiedLineJump::Found { wrapped }) => {
                self.notice = wrapped.then(|| wrapped_message(direction).to_owned());
                cx.notify();
            }
            Ok(ModifiedLineJump::NotFound) => {
                self.notice = Some(not_found_status(direction).to_owned());
                cx.notify();
                if options.notify_not_found {
                    self.info_box(modified_line_not_found_message(direction), window, cx);
                }
            }
            // a document without lines: Sakura returns without a word
            Ok(ModifiedLineJump::Empty) => {}
            Err(e) => {
                self.notice = Some(format!("modified line: {e}"));
                cx.notify();
            }
        }
    }
}
