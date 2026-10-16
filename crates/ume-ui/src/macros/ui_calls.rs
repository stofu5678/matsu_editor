//! The calls of a script to the functions that only the window can run.
//!
//! The bridge (`ume_plugin::bridge`) runs a call on the core commands. A function that only
//! macros call and that needs the window (`TraceOut`, `StatusMsg`, `PutFile`, `InsFile`,
//! `FileOpen`, `SetViewTop`, `WinMaximize` and the others, see `ui_commands/macro_only`) is a UI
//! command; the bridge does not know those, so the window runs them here, the way
//! [`EditorView::run_function`] runs one for a key macro, and answers the script.
//!
//! Only the functions that Sakura has for macros alone are run this way (the kind `MacroOnly` of
//! `ume_commands::sakura::FUNCTIONS`). The other UI commands (the dialogs of the menus, the
//! window list and so on) are not called by scripts yet.
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
use crate::ui_command::UiRegistry;

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
        let macro_only = FUNCTIONS
            .iter()
            .any(|f| f.code == row.code && f.kind == FunctionKind::MacroOnly);
        if row.result.is_some()
            || !macro_only
            || Registry::global().get_by_code(row.code).is_some()
            || UiRegistry::global().get_by_code(row.code).is_none()
        {
            return false;
        }
        let outcome = self.run_ui_function(row, &params, window, cx);
        // A notification gets no answer.
        if let (Some(id), Some(running)) = (id, &self.macros.running) {
            running.session.send(match outcome {
                Ok(()) => response_ok(&id, Value::Null),
                Err(error) => response_error(&id, &error),
            });
        }
        true
    }

    fn run_ui_function(
        &mut self,
        row: &ume_commands::sakura::macro_functions::MacroFunction,
        params: &[Value],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), RpcError> {
        let args = bridge::convert_arguments(row, params)?;
        // An IME composition that the user started ends first, as for a key.
        self.interrupt_composition(cx);
        self.notice = None;
        if self.run_function(row.code, &args, window, cx) {
            return Ok(());
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
