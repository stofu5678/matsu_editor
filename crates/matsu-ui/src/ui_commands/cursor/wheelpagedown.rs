//! `F_WHEELPAGEDOWN`: `Command_WHEELPAGEDOWN`: scrolls the view down by a page per notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELPAGEDOWN;

#[derive(Default)]
pub struct WheelPageDown;

impl UiCommand for WheelPageDown {
    const ID: &'static str = "wheelpagedown";
    const SAKURA: &'static str = "F_WHEELPAGEDOWN";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELPAGEDOWN, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelPageDown);
