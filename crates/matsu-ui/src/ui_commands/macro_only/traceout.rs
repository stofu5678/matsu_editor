//! `F_TRACEOUT` (the macro `TraceOut(text, option)`): writes to the output of macros.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the text
//! is missing and passes the option (default 0); `CViewCommander::Command_TRACEOUT`
//! (`sakura_core/cmd/CViewCommander_Window.cpp`) writes the text to the output window
//! (`CShareData::TraceOutString`) and ends it with CR LF unless the option has 0x02. Option 0x01
//! runs `ExpandParameter` on the text first (a buffer of 2047 characters). The output window is
//! an editor window in Sakura; here it is the output panel under the text of this window
//! (`crate::macros`), which `TraceOut` opens (without starting a new run, so it adds to what is
//! there).

use gpui::{Context, Window};
use matsu_commands::command::{Arg, CommandError, Result};
use matsu_plugin::Value;
use matsu_plugin::functions;
use matsu_plugin::host::Editor;

use super::{int_arg, text_arg};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

/// `ExpandParameter` of the option.
const EXPAND: i64 = 0x01;
/// No line break after the text.
const NO_LINE_BREAK: i64 = 0x02;

#[derive(Default)]
pub struct TraceOut;

impl UiCommand for TraceOut {
    const ID: &'static str = "traceout";
    const SAKURA: &'static str = "F_TRACEOUT";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let text = text_arg("TraceOut", args, 0)?;
        let option = int_arg("TraceOut", args, 1, 0)?;
        let text = if option & EXPAND != 0 {
            expand(view, text, cx)?
        } else {
            text.to_owned()
        };
        view.macros.output.trace(&text, option & NO_LINE_BREAK == 0);
        cx.notify();
        Ok(())
    }
}

/// `CSakuraEnvironment::ExpandParameter` on `text`.
fn expand(view: &mut EditorView, text: &str, cx: &mut Context<EditorView>) -> Result<String> {
    let mut env = view.host_env("", cx);
    let answer = view.run_with(cx, |command_cx| {
        functions::call(
            &mut Editor {
                cx: command_cx,
                env: &mut env,
            },
            "ExpandParameter",
            &[Value::from(text)],
        )
    });
    view.finish_host_env(env, cx);
    match answer {
        Some(Ok(value)) => Ok(value.as_str().unwrap_or_default().to_owned()),
        Some(Err(error)) => Err(CommandError::BadArgs(error.message)),
        None => Err(CommandError::Unavailable("ExpandParameter")),
    }
}

register_ui_command!(TraceOut);
