//! `F_REDRAW`: draws the view again (再描画).
//!
//! Sakura: `Command_REDRAW` (`sakura_core/cmd/CViewCommander_Window.cpp`) calls `RedrawAll` on
//! the view.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Redraw;

impl UiCommand for Redraw {
    const ID: &'static str = "redraw";
    const SAKURA: &'static str = "F_REDRAW";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        window.refresh();
        cx.notify();
        Ok(())
    }
}

register_ui_command!(Redraw);
