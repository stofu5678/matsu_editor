//! The mruby engine: mruby 4.0 (`ume-mruby-sys`, with the C source built in), running the
//! user's script with `Editor` and `EditorError` as top-level constants, as the Ruby boot of
//! `ume-plugin` does for the user's CRuby.
//!
//! Every function of `Editor` comes from `ume_plugin::api::FUNCS` and is bound as a native
//! method by name. A small Ruby part (`mruby_boot.rb`) defines `EditorError` and the text of an
//! error for the standard error.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::{Map, Number, Value};
use ume_mruby_sys::{Failure, Host, Script, Value as Mrb};
use ume_plugin::api::{FUNCS, Func};
use ume_plugin::protocol::RpcError;

use crate::engine::Engine;
use crate::rpc::{CallError, Editor, Initialize};

pub struct Mruby;

const BOOT: &str = include_str!("mruby_boot.rb");

impl Engine for Mruby {
    fn run(&self, init: &Initialize, editor: Rc<RefCell<dyn Editor>>) -> Result<(), String> {
        let names: Vec<&str> = FUNCS.iter().map(|f| f.name).collect();
        let info = json_to_mruby(&init.info);
        // A BOM starts a UTF-8 file but is not Ruby.
        let source = init
            .script
            .source
            .strip_prefix('\u{feff}')
            .unwrap_or(&init.script.source);
        let script = Script {
            boot: BOOT,
            name: &init.script.name,
            source,
            functions: &names,
            info: &info,
        };
        ume_mruby_sys::run(&script, &mut Bridge { editor })
    }
}

/// The functions of the macro API as the editor connection answers them.
struct Bridge {
    editor: Rc<RefCell<dyn Editor>>,
}

impl Host for Bridge {
    fn call(&mut self, function: &str, mut args: Vec<Mrb>) -> Result<Mrb, Failure> {
        let func = FUNCS
            .iter()
            .find(|f| f.name == function)
            .expect("the functions come from FUNCS");
        // `Editor.Jump(2, nil)` means `Editor.Jump(2)`.
        while matches!(args.last(), Some(Mrb::Nil)) {
            args.pop();
        }
        let mut params = Vec::with_capacity(args.len());
        for (index, arg) in args.iter().enumerate() {
            match mruby_to_json(arg) {
                Ok(value) => params.push(value),
                Err(got) => return Err(bad_argument(func, index, &got)),
            }
        }
        let answer = self.editor.borrow_mut().call(func.name, params);
        match answer {
            Ok(result) => Ok(json_to_mruby(&result)),
            Err(CallError::Rpc(RpcError { code, message, .. })) => {
                Err(Failure::Editor { code, message })
            }
            Err(CallError::Closed(message)) => Err(Failure::Fatal(message)),
        }
    }
}

fn bad_argument(func: &Func, index: usize, got: &str) -> Failure {
    let (name, ty) = func
        .params
        .get(index)
        .map_or(("?", "a value"), |p| (p.name, p.ty.name()));
    let error = RpcError::invalid_params(format!(
        "{}: argument \"{name}\" must be {ty}, got {got}",
        func.name
    ));
    Failure::Editor {
        code: error.code,
        message: error.message,
    }
}

/// A script value as JSON, or what the value is when it has no JSON form (an object, a range,
/// a string that is not UTF-8...).
/// A float with a whole value becomes an integer, because the script may have computed it as
/// one (`4 / 2.0`) and the editor wants an integer.
fn mruby_to_json(value: &Mrb) -> Result<Value, String> {
    Ok(match value {
        Mrb::Nil => Value::Null,
        Mrb::Bool(b) => Value::Bool(*b),
        Mrb::Integer(i) => Value::from(*i),
        Mrb::Float(n) => float_to_json(*n),
        // Not a lossy conversion: text with bytes replaced must not reach the document.
        Mrb::Str(bytes) => match String::from_utf8(bytes.clone()) {
            Ok(text) => Value::String(text),
            Err(_) => return Err("a string that is not valid UTF-8".into()),
        },
        Mrb::Array(items) => {
            Value::Array(items.iter().map(mruby_to_json).collect::<Result<_, _>>()?)
        }
        Mrb::Hash(pairs) => {
            let mut map = Map::new();
            for (key, item) in pairs {
                let Mrb::Str(key) = key else {
                    return Err("a hash whose keys are not strings or symbols".into());
                };
                let key = String::from_utf8(key.clone())
                    .map_err(|_| "a hash key that is not valid UTF-8".to_string())?;
                map.insert(key, mruby_to_json(item)?);
            }
            Value::Object(map)
        }
        Mrb::Other(class) => return Err(class.clone()),
    })
}

fn float_to_json(n: f64) -> Value {
    // 2^63 is the first float above the i64 range.
    if n.fract() == 0.0 && n.abs() < 2f64.powi(63) {
        Value::from(n as i64)
    } else {
        // NaN and infinity have no JSON form: null, which the editor rejects for a number.
        Number::from_f64(n).map_or(Value::Null, Value::Number)
    }
}

fn json_to_mruby(value: &Value) -> Mrb {
    match value {
        Value::Null => Mrb::Nil,
        Value::Bool(b) => Mrb::Bool(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => Mrb::Integer(i),
            None => Mrb::Float(n.as_f64().unwrap_or_default()),
        },
        Value::String(s) => Mrb::Str(s.as_bytes().to_vec()),
        Value::Array(items) => Mrb::Array(items.iter().map(json_to_mruby).collect()),
        Value::Object(map) => Mrb::Hash(
            map.iter()
                .map(|(key, item)| (Mrb::Str(key.as_bytes().to_vec()), json_to_mruby(item)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn json_and_mruby_values_round_trip() {
        let value = json!({"a": [1, 2.5, "あ😀", null, true], "b": {}});
        assert_eq!(mruby_to_json(&json_to_mruby(&value)), Ok(value));
    }

    #[test]
    fn a_whole_float_is_an_integer_and_a_bad_string_is_refused() {
        assert_eq!(mruby_to_json(&Mrb::Float(2.0)), Ok(json!(2)));
        assert_eq!(mruby_to_json(&Mrb::Float(2.5)), Ok(json!(2.5)));
        assert!(mruby_to_json(&Mrb::Str(vec![b'a', 0xff])).is_err());
        assert_eq!(
            mruby_to_json(&Mrb::Other("Range".into())),
            Err("Range".into())
        );
        assert!(mruby_to_json(&Mrb::Hash(vec![(Mrb::Integer(1), Mrb::Nil)])).is_err());
    }
}
