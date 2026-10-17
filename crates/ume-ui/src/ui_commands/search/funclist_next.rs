//! `F_FUNCLIST_NEXT`: `Command_FUNCLIST_NEXT` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`) moves to the next line of a
//! function that the last outline analysis found. It is a plain move and follows the search option
//! 先頭（末尾）から再検索. The rules are in `ume_commands::context`.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::search::Direction;

#[derive(Default)]
pub struct FuncListNext;

impl UiCommand for FuncListNext {
    const ID: &'static str = "funclist_next";
    const SAKURA: &'static str = "F_FUNCLIST_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let _ = args;
        view.jump_to_funclist_mark(Direction::Forward, window, cx);
        Ok(())
    }
}

register_ui_command!(FuncListNext);
