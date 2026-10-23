//! `F_WHEELPAGELEFT`: `Command_WHEELPAGELEFT`: scrolls the view left by a page per notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELPAGELEFT;

#[derive(Default)]
pub struct WheelPageLeft;

impl UiCommand for WheelPageLeft {
    const ID: &'static str = "wheelpageleft";
    const SAKURA: &'static str = "F_WHEELPAGELEFT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELPAGELEFT, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelPageLeft);
