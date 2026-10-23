//! `F_TMPWRAPNOWRAP`: `HandleCommand( F_TEXTWRAPMETHOD, WRAP_NO_TEXT_WRAP )` (`sakura_core/cmd/CViewCommander.cpp`, `Command_TEXTWRAPMETHOD` in `CViewCommander_Settings.cpp`): do not wrap, for this view only.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::settings::WrapMethod;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct TmpWrapNoWrap;

impl UiCommand for TmpWrapNoWrap {
    const ID: &'static str = "tmp_wrap_no_wrap";
    const SAKURA: &'static str = "F_TMPWRAPNOWRAP";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.set_wrap_method(WrapMethod::None, cx);
        Ok(())
    }
}

register_ui_command!(TmpWrapNoWrap);
