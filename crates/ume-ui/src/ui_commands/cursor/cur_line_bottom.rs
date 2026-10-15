//! `F_CURLINEBOTTOM`: `Command_CURLINEBOTTOM`: the caret's row goes to the bottom of the view.
//!
//! Sakura sets the top row to the caret's row minus the rows of the view, which puts the caret's
//! row just below the view. This one puts it on the last row that is fully in view.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CurLineBottom;

impl UiCommand for CurLineBottom {
    const ID: &'static str = "cur_line_bottom";
    const SAKURA: &'static str = "F_CURLINEBOTTOM";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let rows = view.metrics().visible_rows();
        view.scroll_caret_row_to(rows - 1, cx);
        Ok(())
    }
}

register_ui_command!(CurLineBottom);
