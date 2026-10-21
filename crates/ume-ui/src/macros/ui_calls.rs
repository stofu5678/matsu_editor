//! The calls of a script to the functions that only the window can run.
//!
//! The bridge (`ume_plugin::bridge`) runs a call on the core commands. A function that needs the
//! window is a UI command: the ones only macros call (`TraceOut`, `StatusMsg`, `PutFile`,
//! `InsFile`, `FileOpen`, `SetViewTop`, `WinMaximize` and the others, see
//! `ui_commands/macro_only`) and the ones of the menus and keys (Grep, the type list, the window
//! list, DIFF, the file functions and so on). The bridge does not know those, so the window runs
//! them here, the way [`EditorView::run_function`] runs one for a key macro, and answers the
//! script.
//!
//! A script can call every function that has a macro name (`ume_commands::macro_names`, which is
//! Sakura's macro table in `sakura_core/macro/CSMacroMgr.cpp`: what Sakura refuses from macros
//! has no name) and that a UI command implements, of the kinds `Command` and `MacroOnly`.
//!
//! A command that opens a dialog or a panel, or that works in the background, is answered when
//! it is over (`wait.rs`), and the clock of the script is held meanwhile, as for a message box.
//!
//! Sakura: `CMacro::HandleCommand` in `sakura_core/macro/CMacro.cpp` runs a command with the
//! arguments of the macro and, for a command that fails, stops the macro.

use gpui::{Context, Window};
use ume_commands::macro_names;
use ume_commands::registry::Registry;
use ume_commands::sakura::{FUNCTIONS, FunctionKind};
use ume_plugin::Value;
use ume_plugin::bridge::{self, UNAVAILABLE};
use ume_plugin::protocol::{Request, RpcError, parse_request, response_error, response_ok};

use super::EditorView;
use super::wait::Wait;
use crate::ui_command::UiRegistry;

/// The UI commands that take the arguments of Sakura's macro function but do not read them yet
/// (they use the settings of the window: the bookmark pattern, the page overlap). A script that
/// gives arguments to them gets an error instead of a silent run with other values.
const IGNORES_ARGUMENTS: &[&str] = &[
    "F_BOOKMARK_PATTERN",
    "F_1PageUp",
    "F_1PageDown",
    "F_1PageUp_Sel",
    "F_1PageDown_Sel",
    "F_HalfPageUp",
    "F_HalfPageDown",
    "F_HalfPageUp_Sel",
    "F_HalfPageDown_Sel",
    "F_1PageUp_BOX",
    "F_1PageDown_BOX",
    "F_HalfPageUp_BOX",
    "F_HalfPageDown_BOX",
];

impl EditorView {
    /// If `text` is a call of a macro-only function that a UI command implements, runs it and
    /// sends the answer to the script, and returns true. Anything else is left to the bridge.
    pub(super) fn run_ui_call(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(Request { id, method, params }) = parse_request(text) else {
            return false;
        };
        let Some(row) = macro_names::by_name(&method) else {
            return false;
        };
        let callable = FUNCTIONS.iter().any(|f| {
            f.code == row.code && matches!(f.kind, FunctionKind::Command | FunctionKind::MacroOnly)
        });
        if row.result.is_some()
            || !callable
            || Registry::global().get_by_code(row.code).is_some()
            || UiRegistry::global().get_by_code(row.code).is_none()
        {
            return false;
        }
        // A command that does not read its arguments yet must not run as if it had none: the
        // search would use the pattern of an earlier search instead of the one the script gave.
        if !params.is_empty() && IGNORES_ARGUMENTS.contains(&row.function) {
            if let (Some(id), Some(running)) = (id, &self.macros.running) {
                running.session.send(response_error(
                    &id,
                    &RpcError::new(
                        UNAVAILABLE,
                        format!(
                            "{} ({}) does not use its arguments yet; call it without them",
                            row.name, row.function
                        ),
                    ),
                ));
            }
            return true;
        }
        let outcome = self.run_ui_function(row, &params, window, cx);
        // A notification gets no answer, and does not wait: the script goes on.
        let Some(id) = id else {
            return true;
        };
        let wait = match outcome {
            Ok(wait) => wait,
            Err(error) => {
                if let Some(running) = &self.macros.running {
                    running.session.send(response_error(&id, &error));
                }
                return true;
            }
        };
        let Some(running) = &self.macros.running else {
            return true;
        };
        match wait {
            None => running.session.send(response_ok(&id, Value::Null)),
            Some(wait) => {
                // The script is answered when the dialog is closed or the work is done; its
                // clock stops meanwhile. A script that has ended or been stopped is not answered
                // (the wait is dropped with it).
                running.session.hold();
                let serial = self.macros.serial;
                self.begin_wait(wait, window, cx, move |this, _, _| {
                    if this.macros.serial == serial
                        && let Some(running) = &this.macros.running
                    {
                        running.session.send(response_ok(&id, Value::Null));
                        running.session.release();
                    }
                });
            }
        }
        true
    }

    fn run_ui_function(
        &mut self,
        row: &ume_commands::sakura::macro_functions::MacroFunction,
        params: &[Value],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Wait>, RpcError> {
        let args = bridge::convert_arguments(row, params)?;
        // An IME composition that the user started ends first, as for a key.
        self.interrupt_composition(cx);
        self.notice = None;
        let (ok, wait) = self.run_macro_function(row.code, &args, window, cx);
        if ok {
            return Ok(wait);
        }
        // The command said why in the notice (a mistake in the arguments is the usual reason).
        let why = self.notice.clone().unwrap_or_default();
        Err(RpcError::new(
            if why.contains("bad arguments") {
                ume_plugin::protocol::INVALID_PARAMS
            } else {
                UNAVAILABLE
            },
            format!("{} ({}) failed: {why}", row.name, row.function),
        ))
    }
}
