//! `F_CURLINETOP`: `Command_CURLINETOP`: the caret's row goes to the top of the view.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`; the rules are in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CurLineTop;

impl UiCommand for CurLineTop {
    const ID: &'static str = "cur_line_top";
    const SAKURA: &'static str = "F_CURLINETOP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.scroll_caret_row_to(0, cx);
        Ok(())
    }
}

register_ui_command!(CurLineTop);
