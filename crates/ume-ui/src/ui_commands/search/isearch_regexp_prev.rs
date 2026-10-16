//! `F_ISEARCH_REGEXP_PREV`: starts the incremental search backward, with the pattern as a regular
//! expression (正規表現後方インクリメンタルサーチ).
//!
//! Sakura: `CViewCommander::HandleCommand` calls `ProcessCommand_isearch`, which runs
//! `ISearchEnter(SEARCH_REGEXP, SEARCH_BACKWARD)` (`sakura_core/view/CEditView_Cmdisrch.cpp`).
//! When the mode is already on in the same mode, this is the repeated command: the next match in
//! this direction. See [`crate::search::isearch`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};
use ume_core::search::Direction;

use crate::search::ISearchMode;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct IsearchRegexpPrev;

impl UiCommand for IsearchRegexpPrev {
    const ID: &'static str = "isearch_regexp_prev";
    const SAKURA: &'static str = "F_ISEARCH_REGEXP_PREV";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.isearch_enter(ISearchMode::Regexp, Direction::Backward, cx);
        Ok(())
    }
}

register_ui_command!(IsearchRegexpPrev);
