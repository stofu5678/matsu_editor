//! `F_CURLINECENTER`: scrolls so that the caret's row is in the middle of the view.
//!
//! Sakura: `CViewCommander::Command_CURLINECENTER` and `MoveViewTopLine` in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`: the top row becomes the caret's row minus half of
//! the rows of the view, and not less than the first row. The caret does not move.
//!
//! Like Sakura's `MoveViewTopLine`, the top row can be past the scroll range, so near the end of
//! the document the caret's row is still in the middle (`EditorView::move_view_top_line`).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CurLineCenter;

impl UiCommand for CurLineCenter {
    const ID: &'static str = "cur_line_center";
    const SAKURA: &'static str = "F_CURLINECENTER";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let rows = view.metrics().visible_rows();
        view.scroll_caret_row_to(rows / 2, cx);
        Ok(())
    }
}

register_ui_command!(CurLineCenter);
