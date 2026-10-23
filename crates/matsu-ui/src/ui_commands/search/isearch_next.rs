//! `F_ISEARCH_NEXT`: starts the incremental search forward (前方インクリメンタルサーチ).
//!
//! Sakura: `CViewCommander::HandleCommand` calls `ProcessCommand_isearch`, which runs
//! `ISearchEnter(SEARCH_NORMAL, SEARCH_FORWARD)` (`sakura_core/view/CEditView_Cmdisrch.cpp`).
//! When the mode is already on in the same mode, this is the repeated command: the next match in
//! this direction. See [`crate::search::isearch`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::search::ISearchMode;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct IsearchNext;

impl UiCommand for IsearchNext {
    const ID: &'static str = "isearch_next";
    const SAKURA: &'static str = "F_ISEARCH_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.isearch_enter(ISearchMode::Normal, Direction::Forward, cx);
        Ok(())
    }
}

register_ui_command!(IsearchNext);
