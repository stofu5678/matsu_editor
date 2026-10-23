//! `F_MSGBEEP` (the macro `MsgBeep(type)`): plays a system sound.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) maps the type (default 0) to
//! the argument of `MessageBeep`: -1 the simple beep, 0 `MB_OK`, 1 `MB_ICONERROR`, 2
//! `MB_ICONQUESTION`, 3 `MB_ICONWARNING`, 4 `MB_ICONINFORMATION`, anything else `MB_OK`. GPUI
//! plays the one alert sound of the OS (`Window::play_system_bell`), so the kinds are not told
//! apart here.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use super::int_arg;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct MsgBeep;

impl UiCommand for MsgBeep {
    const ID: &'static str = "msgbeep";
    const SAKURA: &'static str = "F_MSGBEEP";

    fn run(
        &self,
        _view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        _cx: &mut Context<EditorView>,
    ) -> Result {
        // The type only picks the sound; it must still be a number.
        int_arg("MsgBeep", args, 0, 0)?;
        window.play_system_bell();
        Ok(())
    }
}

register_ui_command!(MsgBeep);
