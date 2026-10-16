//! `F_TEXTWRAPMETHOD` (the macro `TextWrapMethod(method)`): sets how the text wraps, for this
//! view only (a temporary setting).
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) passes the number (a missing
//! one is 0); `CViewCommander::Command_TEXTWRAPMETHOD` (`sakura_core/cmd/CViewCommander_Settings.cpp`)
//! does nothing when the document already wraps that way, ignores a number other than 0 (do not
//! wrap), 1 (at the column of the document type) and 2 (at the right edge of the window), and
//! otherwise changes the layout. `F_TMPWRAPNOWRAP`, `F_TMPWRAPSETTING` and `F_TMPWRAPWINDOW` call
//! it with 0, 1 and 2; the rules are in [`EditorView::set_wrap_method`].

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use super::int_arg;
use crate::settings::WrapMethod;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct TextWrapMethod;

impl UiCommand for TextWrapMethod {
    const ID: &'static str = "textwrapmethod";
    const SAKURA: &'static str = "F_TEXTWRAPMETHOD";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let method = match int_arg("TextWrapMethod", args, 0, 0)? {
            0 => WrapMethod::None,
            1 => WrapMethod::SettingWidth,
            2 => WrapMethod::WindowWidth,
            // `default: return;`
            _ => return Ok(()),
        };
        view.set_wrap_method(method, cx);
        Ok(())
    }
}

register_ui_command!(TextWrapMethod);
