//! `F_FUNCLIST_PREV`: `Command_FUNCLIST_PREV` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`) moves to the previous line of a
//! function that the last outline analysis found. See `funclist_next`.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

#[derive(Default)]
pub struct FuncListPrev;

impl UiCommand for FuncListPrev {
    const ID: &'static str = "funclist_prev";
    const SAKURA: &'static str = "F_FUNCLIST_PREV";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = args;
        view.jump_to_funclist_mark(Direction::Backward, window, cx);
        Ok(())
    }
}

register_ui_command!(FuncListPrev);
