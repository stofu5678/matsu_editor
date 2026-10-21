//! `F_AUTOSCROLL`: starts the auto scroll at the pointer, or ends it (`Command_AUTOSCROLL`).
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The middle button runs the same function;
//! the rules are in `view/autoscroll.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Autoscroll;

impl UiCommand for Autoscroll {
    const ID: &'static str = "autoscroll";
    const SAKURA: &'static str = "F_AUTOSCROLL";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.toggle_autoscroll(window, cx);
        Ok(())
    }
}

register_ui_command!(Autoscroll);
