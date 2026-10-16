//! `F_WHEELPAGERIGHT`: `Command_WHEELPAGERIGHT`: scrolls the view right by a page per notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELPAGERIGHT;

#[derive(Default)]
pub struct WheelPageRight;

impl UiCommand for WheelPageRight {
    const ID: &'static str = "wheelpageright";
    const SAKURA: &'static str = "F_WHEELPAGERIGHT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELPAGERIGHT, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelPageRight);
