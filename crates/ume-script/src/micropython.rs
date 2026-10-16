//! The MicroPython engine (`.upy`): MicroPython built into `ume-script` (crate
//! `ume-micropython-sys`), running the user's script with `Editor` and `EditorError` in its
//! globals, as `boot.py` does for CPython.
//!
//! The macro API reaches the script through one native function, `_ume_call(name, args)`. A
//! small Python part (`BOOT`) makes `Editor` out of the names in `ume_plugin::catalog::entries()` (so no
//! function is written by hand) and turns the answer into a return value or an `EditorError`.
//! The arguments are written as JSON by the C glue, a result that is a string comes back as it
//! is, and any other result is parsed by the `json` module of MicroPython (`json.dumps` and
//! `json.loads` take quadratic time on a string of megabytes, which a selection can be).
//!
//! What differs from CPython: this is MicroPython (its language level, its library), a trailing
//! `None` argument is left out (like Lua's trailing `nil`), the traceback is MicroPython's, and
//! `import` finds only the built-in modules (`sys`, `json`, `math`, `struct`, `collections`,
//! `array`, `binascii`, `heapq`, `io`, `errno`, `gc`).

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::{Number, Value};
use ume_micropython_sys::{CallOutcome, Host, Source};
use ume_plugin::catalog::entries;
use ume_plugin::protocol::RpcError;

use crate::engine::Engine;
use crate::rpc::{CallError, Editor, Initialize};

pub struct MicroPython;

/// Runs before the user's script, in the same globals. It leaves `Editor`, `EditorError` and
/// the helpers `_json`, `_bind` and `_ume_call` there (the underscore names are not for the
/// script).
const BOOT: &str = r#"import json as _json


class EditorError(Exception):
    """The editor answered a call with an error (a JSON-RPC error object)."""

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message


class _Editor:
    pass


def _bind(name):
    def call(*args):
        args = list(args)
        while args and args[-1] is None:
            args.pop()
        answer = _ume_call(name, args)
        if type(answer) is str:
            return answer
        if len(answer) == 1:
            return _json.loads(answer[0])
        raise EditorError(answer[0], answer[1])

    return call


def _install():
    names, info = _ume_setup()
    editor = _Editor()
    for name in _json.loads(names):
        setattr(editor, name, _bind(name))
    editor.info = _json.loads(info)
    return editor


Editor = _install()
del _install, _Editor, _ume_setup
"#;

impl Engine for MicroPython {
    fn run(&self, init: &Initialize, editor: Rc<RefCell<dyn Editor>>) -> Result<(), String> {
        let names: Vec<&str> = entries().iter().map(|func| func.name).collect();
        let names_json = Value::from(names).to_string();
        let info_json = init.info.to_string();
        let mut bridge = Bridge { editor };
        ume_micropython_sys::run(
            &mut bridge,
            &names_json,
            &info_json,
            Source {
                name: "<ume-boot>",
                text: BOOT,
            },
            Source {
                name: &init.script.name,
                text: &init.script.source,
            },
        )
    }
}

/// The calls of the script, forwarded to the editor.
struct Bridge {
    editor: Rc<RefCell<dyn Editor>>,
}

impl Host for Bridge {
    fn call(&mut self, name: &[u8], args: &[u8]) -> CallOutcome {
        let invalid = |message: String| {
            let error = RpcError::invalid_params(message);
            CallOutcome::Error {
                code: error.code,
                message: error.message,
            }
        };
        let Ok(name) = std::str::from_utf8(name) else {
            return invalid("the name of the function is not valid UTF-8".into());
        };
        let params = match parse_arguments(args) {
            Ok(params) => params,
            Err(why) => return invalid(format!("{name}: {why}")),
        };
        let answer = self.editor.borrow_mut().call(name, params);
        match answer {
            Ok(Value::String(text)) => CallOutcome::Text(text),
            Ok(result) => CallOutcome::Ok(result.to_string()),
            Err(CallError::Rpc(error)) => CallOutcome::Error {
                code: error.code,
                message: error.message,
            },
            Err(CallError::Closed(message)) => CallOutcome::Closed(message),
        }
    }
}

/// The arguments that `json.dumps` produced, as the values of the request. A float with a whole
/// value becomes an integer, because Python writes `2.0` for what the script meant as 2 (the
/// result of a division, for one).
fn parse_arguments(args: &[u8]) -> Result<Vec<Value>, String> {
    // Not lossy: text with bytes replaced must not reach the document (a string with a lone
    // surrogate is such a case).
    let text = std::str::from_utf8(args)
        .map_err(|_| "an argument is a string that is not valid UTF-8".to_string())?;
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Array(mut items)) => {
            items.iter_mut().for_each(whole_floats_to_integers);
            Ok(items)
        }
        _ => Err("an argument cannot be sent (it has no JSON form)".into()),
    }
}

fn whole_floats_to_integers(value: &mut Value) {
    match value {
        Value::Number(n) if n.is_f64() => {
            let f = n.as_f64().unwrap_or_default();
            // 2^63 is the first float above the i64 range.
            if f.fract() == 0.0 && f.abs() < 2f64.powi(63) {
                *value = Value::Number(Number::from(f as i64));
            }
        }
        Value::Array(items) => items.iter_mut().for_each(whole_floats_to_integers),
        Value::Object(map) => map.values_mut().for_each(whole_floats_to_integers),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn arguments_come_from_a_json_array() {
        assert_eq!(
            parse_arguments(br#"[1, "a", 2.0, 2.5, null, [3.0]]"#).unwrap(),
            [
                json!(1),
                json!("a"),
                json!(2),
                json!(2.5),
                json!(null),
                json!([3])
            ]
        );
        assert_eq!(parse_arguments(b"[]").unwrap(), Vec::<Value>::new());
    }

    #[test]
    fn what_has_no_json_form_is_refused() {
        // MicroPython writes `nan` and the repr of an object.
        assert!(parse_arguments(b"[nan]").is_err());
        assert!(parse_arguments(b"[<object 1>]").is_err());
        assert!(parse_arguments(b"{}").is_err());
        // A string with a lone surrogate (U+D800 as UTF-8 bytes).
        assert!(parse_arguments(b"[\"\xed\xa0\x80\"]").is_err());
    }
}
