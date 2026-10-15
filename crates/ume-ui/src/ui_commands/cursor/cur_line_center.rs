//! `F_CURLINECENTER`: scrolls so that the caret's row is in the middle of the view.
//!
//! Sakura: `CViewCommander::Command_CURLINECENTER` and `MoveViewTopLine` in
//! `sakura_core/cmd/CViewCommander_Cursor.cpp`: the top row becomes the caret's row minus half of
//! the rows of the view, and not less than the first row. The caret does not move.
//!
//! Not like Sakura yet: near the end of the document the view stops at its scroll range (the last
//! row at the bottom, plus one blank row), so the caret's row may stay below the middle. Sakura's
//! scroll bar has about the same range (`CEditView::AdjustScrollBars`: the rows plus 2), but
//! `MoveViewTopLine` does not stop there. To do with the rest of Sakura's scrolling (page up and
//! down, `F_WndScrollUp`/`Down`).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

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
