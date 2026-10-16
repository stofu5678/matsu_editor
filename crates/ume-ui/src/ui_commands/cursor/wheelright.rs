//! `F_WHEELRIGHT`: `Command_WHEELRIGHT`: scrolls the view right by the columns of one notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELRIGHT;

#[derive(Default)]
pub struct WheelRight;

impl UiCommand for WheelRight {
    const ID: &'static str = "wheelright";
    const SAKURA: &'static str = "F_WHEELRIGHT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELRIGHT, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelRight);
