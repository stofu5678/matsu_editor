//! `F_ISEARCH_MIGEMO_NEXT`: starts the incremental search forward, with the pattern made by
//! Migemo (MIGEMO前方インクリメンタルサーチ).
//!
//! Sakura: `CViewCommander::HandleCommand` calls `ProcessCommand_isearch`, which runs
//! `ISearchEnter(SEARCH_MIGEMO, SEARCH_FORWARD)` (`sakura_core/view/CEditView_Cmdisrch.cpp`).
//! When the library or the dictionary is not there, the mode does not begin and the status bar
//! says why. When the mode is already on in the same mode, this is the repeated command: the
//! next match in this direction. See [`crate::search::isearch`] and [`crate::search::migemo`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};
use matsu_core::search::Direction;

use crate::search::ISearchMode;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct IsearchMigemoNext;

impl UiCommand for IsearchMigemoNext {
    const ID: &'static str = "isearch_migemo_next";
    const SAKURA: &'static str = "F_ISEARCH_MIGEMO_NEXT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.isearch_enter(ISearchMode::Migemo, Direction::Forward, cx);
        Ok(())
    }
}

register_ui_command!(IsearchMigemoNext);
