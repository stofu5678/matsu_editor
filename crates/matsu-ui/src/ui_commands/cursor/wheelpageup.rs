//! `F_WHEELPAGEUP`: `Command_WHEELPAGEUP`: scrolls the view up by a page per notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELPAGEUP;

#[derive(Default)]
pub struct WheelPageUp;

impl UiCommand for WheelPageUp {
    const ID: &'static str = "wheelpageup";
    const SAKURA: &'static str = "F_WHEELPAGEUP";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELPAGEUP, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelPageUp);
