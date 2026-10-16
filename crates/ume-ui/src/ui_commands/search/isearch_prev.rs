//! `F_ISEARCH_PREV`: starts the incremental search backward (後方インクリメンタルサーチ).
//!
//! Sakura: `CViewCommander::HandleCommand` calls `ProcessCommand_isearch`, which runs
//! `ISearchEnter(SEARCH_NORMAL, SEARCH_BACKWARD)` (`sakura_core/view/CEditView_Cmdisrch.cpp`).
//! When the mode is already on in the same mode, this is the repeated command: the next match in
//! this direction. See [`crate::search::isearch`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::search::Direction;

use crate::search::ISearchMode;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct IsearchPrev;

impl UiCommand for IsearchPrev {
    const ID: &'static str = "isearch_prev";
    const SAKURA: &'static str = "F_ISEARCH_PREV";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.isearch_enter(ISearchMode::Normal, Direction::Backward, cx);
        Ok(())
    }
}

register_ui_command!(IsearchPrev);
