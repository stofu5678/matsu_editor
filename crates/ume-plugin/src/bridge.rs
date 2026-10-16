//! The bridge from a macro's calls to the editor's commands (ADR-0007, ADR-0013 section 4).
//!
//! A macro calls Sakura's macro names (`Down`, `InsText`, `GetSelectedString`). The bridge looks
//! the name up in Sakura's macro tables (`ume_commands::sakura::macro_functions`, generated from
//! `sakura_core/macro/CSMacroMgr.cpp`) and then:
//!
//! 1. **A command** (the name is in `m_MacroFuncInfoCommandArr`): if a command is registered for
//!    the function code, the bridge converts the JSON arguments to `Arg`s and runs the command
//!    through the registry. The command is one undo block, as when a key runs it.
//! 2. **A function that returns a value** (`m_MacroFuncInfoArr`): answered from the document.
//! 3. A name that Sakura has but no command or answer exists for yet gets the error
//!    [`NOT_IMPLEMENTED`] that names the function, so the macro author sees what is missing.
//!    A name that Sakura does not have gets `-32601`.
//!
//! # How this relates to `api::MacroDocument`
//!
//! `MacroDocument` and `api::FUNCS` (the first host, 14 functions) are kept. They stay the way to
//! run the macro API on a plain text model, which the host tests use, and they are the fallback
//! of the bridge: when a name is in `FUNCS` and no command is registered for it yet (`InsText`,
//! `AddTail`, `MoveCursor`, `Jump` today), the bridge runs the `FUNCS` code on
//! [`ContextDocument`], which implements `MacroDocument` over the `CommandContext`. A command
//! that is registered wins over `FUNCS`, so adding a command later changes the path without any
//! change here. The value functions of `FUNCS` (`GetLineStr`, `GetSelectedString`, ...) have no
//! commands and always use it.
//!
//! The editor makes one `CommandContext` for the whole macro run, calls
//! [`handle_message`] for every message the macro sends, and calls `finish` on the context when
//! the macro ends to learn what changed.

use serde_json::Value;
use ume_commands::command::{Arg, CommandError};
use ume_commands::context::CommandContext;
use ume_commands::macro_names;
use ume_commands::registry::Registry;
use ume_commands::sakura::macro_functions::{ArgType, MacroFunction};
use ume_core::buffer::Pos;

use crate::api::{self, MacroDocument};
use crate::protocol::{INTERNAL_ERROR, INVALID_PARAMS, METHOD_NOT_FOUND, RpcError};

/// The error code for a function that Sakura has but this editor does not run yet. It is in the
/// range that JSON-RPC 2.0 leaves to the server (-32000 to -32099).
pub const NOT_IMPLEMENTED: i64 = -32001;

/// The error code for a function that needs something this run of the editor does not have, for
/// example the clipboard when the macro runs without a window. Also in the server's range.
pub const UNAVAILABLE: i64 = -32002;

/// Handles one message line from the macro and returns the response line, or `None` for a
/// notification.
pub fn handle_message(
    registry: &Registry,
    cx: &mut CommandContext<'_>,
    text: &str,
) -> Option<String> {
    api::respond(text, |method, params| call(registry, cx, method, params))
}

/// Runs one call by its Sakura macro name. See the module documentation.
pub fn call(
    registry: &Registry,
    cx: &mut CommandContext<'_>,
    method: &str,
    params: &[Value],
) -> Result<Value, RpcError> {
    let row = macro_names::by_name(method)
        .ok_or_else(|| RpcError::new(METHOD_NOT_FOUND, format!("unknown method: {method}")))?;
    let args = convert_arguments(row, params)?;
    let in_funcs = api::FUNCS.iter().any(|f| f.name == row.name);
    if row.result.is_none()
        && let Some(command) = registry.get_by_code(row.code)
    {
        command.run(cx, &args).map_err(command_error)?;
        return Ok(Value::Null);
    }
    if in_funcs {
        // A command of the basic API is one undo block, like a registered command.
        let is_command = row.result.is_none();
        if is_command {
            cx.begin_block();
        }
        let result = run_basic_api(cx, row.name, params);
        if is_command {
            cx.end_block();
        }
        return result;
    }
    Err(RpcError::new(
        NOT_IMPLEMENTED,
        format!(
            "{} ({}) is not supported yet: {}",
            row.name,
            row.function,
            if row.result.is_none() {
                "no command is registered for it"
            } else {
                "the editor cannot answer it yet"
            }
        ),
    ))
}

/// Checks the JSON arguments against the types in Sakura's table and converts them. A macro may
/// leave out the last arguments (Sakura passes a null pointer for those); the command decides
/// whether it can do without them.
fn convert_arguments(row: &MacroFunction, params: &[Value]) -> Result<Vec<Arg>, RpcError> {
    if params.len() > row.args.len() {
        return Err(RpcError::invalid_params(format!(
            "{}: expected at most {} argument(s), got {}",
            row.name,
            row.args.len(),
            params.len()
        )));
    }
    row.args
        .iter()
        .zip(params)
        .enumerate()
        .map(|(i, (ty, value))| {
            let arg = match ty {
                // Only what fits an i64: a bigger number must not turn into something else.
                ArgType::Int => value.as_i64().map(Arg::Int),
                ArgType::Str => value.as_str().map(|s| Arg::Text(s.to_owned())),
            };
            arg.ok_or_else(|| {
                RpcError::invalid_params(format!(
                    "{}: argument {} (\"{}\") must be {}",
                    row.name,
                    i + 1,
                    value,
                    match ty {
                        ArgType::Int => "an integer",
                        ArgType::Str => "a string",
                    }
                ))
            })
        })
        .collect()
}

fn command_error(error: CommandError) -> RpcError {
    match error {
        CommandError::BadArgs(message) => RpcError::new(INVALID_PARAMS, message),
        CommandError::Unknown(what) => {
            RpcError::new(NOT_IMPLEMENTED, format!("no such command: {what}"))
        }
        CommandError::Buffer(e) => RpcError::new(INTERNAL_ERROR, e.to_string()),
        e @ CommandError::Unavailable(_) => RpcError::new(UNAVAILABLE, e.to_string()),
    }
}

/// Runs a function of `api::FUNCS` on the context.
fn run_basic_api(
    cx: &mut CommandContext<'_>,
    name: &str,
    params: &[Value],
) -> Result<Value, RpcError> {
    let mut doc = ContextDocument::new(cx);
    let result = api::call(&mut doc, name, params);
    match doc.error {
        // A failed edit or move: the answer of the function is not to be trusted.
        Some(error) => Err(command_error(error)),
        None => result,
    }
}

/// `MacroDocument` over a `CommandContext`, for the functions of `api::FUNCS` that have no
/// command. Positions are the document's `Pos` (line, byte offset in the line). The trait has no
/// way to fail, so the first error is kept and the call fails with it afterwards.
pub struct ContextDocument<'a, 'b> {
    cx: &'a mut CommandContext<'b>,
    error: Option<CommandError>,
}

impl<'a, 'b> ContextDocument<'a, 'b> {
    pub fn new(cx: &'a mut CommandContext<'b>) -> Self {
        Self { cx, error: None }
    }

    fn keep<T: Default>(&mut self, result: Result<T, impl Into<CommandError>>) -> T {
        match result {
            Ok(value) => value,
            Err(e) => {
                self.error.get_or_insert(e.into());
                T::default()
            }
        }
    }
}

impl MacroDocument for ContextDocument<'_, '_> {
    fn line_count(&self) -> usize {
        // The trait promises 1 or more; an empty document may report 0 lines.
        self.cx.doc().line_count().max(1)
    }

    fn line_len(&self, row: usize) -> usize {
        self.cx
            .doc()
            .line_len(row)
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn line_text(&self, row: usize) -> String {
        self.cx
            .doc()
            .line(row)
            .ok()
            .flatten()
            .map(|line| line.text)
            .unwrap_or_default()
    }

    fn cursor(&self) -> (usize, usize) {
        let caret = self.cx.caret();
        (caret.line, caret.col)
    }

    fn has_selection(&self) -> bool {
        self.cx.selection().is_some()
    }

    fn selected_text(&self) -> String {
        let Some((start, end)) = self.cx.selection() else {
            return String::new();
        };
        let text = self.cx.doc().text_between(start, end).unwrap_or_default();
        // Every line ending the document knows becomes "\n" (like the lines of `GetLineStr`).
        text.replace("\r\n", "\n")
            .replace(['\r', '\u{85}', '\u{2028}', '\u{2029}'], "\n")
    }

    fn move_cursor(&mut self, pos: (usize, usize), extend: bool) {
        let result = self.cx.move_caret(Pos::new(pos.0, pos.1), extend);
        self.keep(result);
    }

    fn move_vertical(&mut self, delta: isize) {
        let result = self.cx.move_vertical(delta, false);
        self.keep(result);
    }

    fn select_all(&mut self) {
        let result = self.cx.select_all();
        self.keep(result);
    }

    fn insert(&mut self, text: &str) {
        let result = self.cx.insert_text(text);
        self.keep(result.map(|_| ()));
    }

    fn path(&self) -> Option<String> {
        self.cx.path().map(|p| p.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests;
