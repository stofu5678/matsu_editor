//! `F_SETVIEWLEFT` (the macro `SetViewLeft(column)`): scrolls so that a column is at the left edge
//! of the view.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) fails when the argument is
//! missing, empty or does not start with a digit, subtracts 1 (the first column is 1; below 0 is
//! 0) and calls `CEditView::ScrollAtH`, then `SyncScrollH` for the other pane. `ScrollAtH` keeps
//! the column between 0 and the scroll range (`CEditView_Scroll.cpp`); here the range ends where
//! the widest row of the last paint ends at the right edge (as the wheel does, `scroll_by`).
//! The caret does not move.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use super::setviewtop::position;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct SetViewLeft;

impl UiCommand for SetViewLeft {
    const ID: &'static str = "setviewleft";
    const SAKURA: &'static str = "F_SETVIEWLEFT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let column = position("SetViewLeft", args)?;
        let cell = view.metrics.cell_width;
        let widest = (view.widest_cols as f32 + 1.0) * cell;
        let max_x = (widest - view.metrics.view_width).max(0.0);
        view.scroll_x = (column as f32 * cell).min(max_x);
        cx.notify();
        Ok(())
    }
}

register_ui_command!(SetViewLeft);
