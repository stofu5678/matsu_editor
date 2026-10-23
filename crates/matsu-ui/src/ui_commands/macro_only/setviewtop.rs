//! `F_SETVIEWTOP` (the macro `SetViewTop(row)`): scrolls so that a row is at the top of the view.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) fails when the argument is
//! missing, empty or does not start with a digit (a negative number fails), subtracts 1 (the
//! first row is 1; below 0 is 0) and calls `CEditView::ScrollAtV`, then `SyncScrollV` for the
//! other pane. `ScrollAtV` keeps the top row inside the scroll range
//! ([`EditorView::scroll_top_to`]); the split panes follow by themselves (`crate::panes`). The
//! caret does not move.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, CommandError, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SetViewTop;

/// The number of `SetViewTop` and `SetViewLeft`: Sakura's `Is09` test of the first character
/// refuses what is not a plain non-negative number.
pub(super) fn position(name: &str, args: &[Arg]) -> Result<isize> {
    match args.first() {
        Some(Arg::Int(n)) if *n >= 0 => {
            // 1-based; 0 is the same as 1
            Ok(isize::try_from(n.saturating_sub(1).max(0)).unwrap_or(isize::MAX))
        }
        _ => Err(CommandError::BadArgs(format!(
            "{name} needs a number of 0 or more"
        ))),
    }
}

impl UiCommand for SetViewTop {
    const ID: &'static str = "setviewtop";
    const SAKURA: &'static str = "F_SETVIEWTOP";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let row = position("SetViewTop", args)?;
        // The scroll range comes from the number of rows: a document that is still being laid
        // out in the background has an estimate, and a row past it would be cut short.
        if !view.layout.is_complete() {
            let _ = view.layout.layout_all(view.document.read(cx).doc());
        }
        view.scroll_top_to(row, cx);
        Ok(())
    }
}

register_ui_command!(SetViewTop);
