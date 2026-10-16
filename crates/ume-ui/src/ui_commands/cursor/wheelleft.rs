//! `F_WHEELLEFT`: `Command_WHEELLEFT`: scrolls the view left by the columns of one notch.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Cursor.cpp`. The argument is `zDelta` (120 per notch;
//! 0 or none means one notch). The rules are the wheel's, in `view/paging.rs`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::view::paging::F_WHEELLEFT;

#[derive(Default)]
pub struct WheelLeft;

impl UiCommand for WheelLeft {
    const ID: &'static str = "wheelleft";
    const SAKURA: &'static str = "F_WHEELLEFT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.run_wheel_function(F_WHEELLEFT, args, cx);
        Ok(())
    }
}

register_ui_command!(WheelLeft);
