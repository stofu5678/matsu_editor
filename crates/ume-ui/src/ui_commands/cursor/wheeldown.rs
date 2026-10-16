//! `F_WHEELDOWN`: `Command_WHEELDOWN`: scrolls the view down by the rows of one notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELDOWN;

#[derive(Default)]
pub struct WheelDown;

impl UiCommand for WheelDown {
    const ID: &'static str = "wheeldown";
    const SAKURA: &'static str = "F_WHEELDOWN";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELDOWN, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelDown);
