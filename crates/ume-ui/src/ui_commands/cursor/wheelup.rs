//! `F_WHEELUP`: `Command_WHEELUP`: scrolls the view up by the rows of one notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELUP;

#[derive(Default)]
pub struct WheelUp;

impl UiCommand for WheelUp {
    const ID: &'static str = "wheelup";
    const SAKURA: &'static str = "F_WHEELUP";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELUP, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelUp);
