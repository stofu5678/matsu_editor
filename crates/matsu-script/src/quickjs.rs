//! The QuickJS engine: `.js` macros, on the QuickJS that the `rquickjs` crate builds from C
//! source (MIT), so that a JavaScript macro runs with nothing installed. `.mjs` and `.cjs` stay
//! on Node.js.
//!
//! The script runs as a classic script (not a module) with `Editor`, `EditorError` and `console`
//! in the global scope, as `boot.cjs` gives them to a Node.js macro. Every function of `Editor`
//! comes from `matsu_plugin::catalog::entries()` (the basic API and every row of Sakura's macro tables) and is bound as a Rust function. A small JavaScript part
//! (`quickjs.js`) turns the answer of a Rust function into a return value or a thrown
//! `EditorError`, defines `console`, and describes an uncaught error.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use matsu_plugin::catalog::{Entry, entries};
use matsu_plugin::protocol::RpcError;
use rquickjs::{
    Array, CatchResultExt, Context, Ctx, Exception, Function, IntoJs, Object, Persistent, Runtime,
    Type, Value, context::EvalOptions,
};
use serde_json::{Number, Value as Json};

use crate::engine::Engine;
use crate::rpc::{CallError, Editor, Initialize, Invoke, LinkEditor, PluginInit, PluginLink};

pub struct QuickJs;

const BOOT: &str = include_str!("quickjs.js");

/// How deep the script may recurse, in bytes of the machine's stack. The main thread of a
/// Windows program has 1 MB, and QuickJS must stop (with a `RangeError`) before it is used up.
const MAX_STACK: usize = 512 * 1024;

/// A JSON value nests no deeper than this when it is made of a JavaScript value (a cycle in an
/// argument ends here instead of in a stack overflow).
const MAX_DEPTH: usize = 64;

/// What the boot (`quickjs.js`) gives back to Rust, kept across `Context::with` calls.
type Hooks = Persistent<Object<'static>>;

/// A runtime and its context, with the list of the promises that nobody handled.
///
/// The fields are dropped in this order, and the order matters: the objects that `hooks` keeps
/// alive must go before the runtime does (QuickJS asserts that no object is left when the runtime
/// is freed).
struct Vm {
    hooks: Rc<RefCell<Option<Hooks>>>,
    context: Context,
    runtime: Runtime,
}

impl Drop for Vm {
    /// Lets go of what the boot keeps (the tracker of the promises holds the list too) while the
    /// runtime is still there: an object that is freed after the runtime started to free itself
    /// trips QuickJS's own check, which aborts the process.
    fn drop(&mut self) {
        self.runtime.set_host_promise_rejection_tracker(None);
        drop(self.hooks.borrow_mut().take());
        self.runtime.run_gc();
    }
}

impl Vm {
    fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| format!("cannot set up QuickJS: {e}"))?;
        runtime.set_max_stack_size(MAX_STACK);
        let context = Context::full(&runtime).map_err(|e| format!("cannot set up QuickJS: {e}"))?;

        // A promise that is rejected and that nothing handles ends the macro like an uncaught
        // error, as Node.js does. The boot keeps the list (per promise); Rust only reports to it.
        let hooks: Rc<RefCell<Option<Hooks>>> = Rc::default();
        let tracker = hooks.clone();
        runtime.set_host_promise_rejection_tracker(Some(Box::new(
            move |ctx, promise, reason, is_handled| {
                let Some(saved) = tracker.borrow().clone() else {
                    return;
                };
                if let Ok(track) = saved
                    .restore(&ctx)
                    .and_then(|hooks| hooks.get::<_, Function>("track"))
                {
                    let _ = track.call::<_, ()>((promise, reason, is_handled));
                }
            },
        )));
        Ok(Self {
            runtime,
            context,
            hooks,
        })
    }

    /// Promise callbacks (an `async` function, `.then`) run after the script, before the end;
    /// then a promise that was rejected and not handled is an error. This locks the runtime, so it
    /// cannot be inside `context.with`.
    fn finish(&self) -> Result<(), String> {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(true) => {}
                Ok(false) => break,
                // A callback that threw (`queueMicrotask`): the exception is still pending.
                Err(_) => {
                    return self.context.with(|ctx| {
                        let exception = ctx.catch();
                        Err(match restore_hooks(&ctx, &self.hooks) {
                            Ok(hooks) => describe_value(&hooks, exception),
                            Err(message) => message,
                        })
                    });
                }
            }
        }
        self.context.with(|ctx| {
            let hooks = restore_hooks(&ctx, &self.hooks)?;
            let unhandled: Function = hooks.get("unhandled").map_err(|e| e.to_string())?;
            let reasons: Array = unhandled.call(()).map_err(|e| e.to_string())?;
            match reasons.get::<Value>(0) {
                Ok(reason) if !reasons.is_empty() => Err(describe_value(&hooks, reason)),
                _ => Ok(()),
            }
        })
    }
}

impl Engine for QuickJs {
    fn run(&self, init: &Initialize, editor: Rc<RefCell<dyn Editor>>) -> Result<(), String> {
        let vm = Vm::new()?;
        vm.context
            .with(|ctx| start(&ctx, init, editor, &vm.hooks))?;
        vm.finish()
    }

    /// A plugin: one runtime for as long as the host lives, so that what a handler leaves in the
    /// global scope is there for the next one. Each `plugin/invoke` runs the text of its handler
    /// as a classic script, as a macro does.
    fn serve(&self, init: &PluginInit, link: Rc<RefCell<dyn PluginLink>>) -> Result<(), String> {
        let vm = Vm::new()?;
        let editor: Rc<RefCell<dyn Editor>> = Rc::new(RefCell::new(LinkEditor(link.clone())));
        vm.context.with(|ctx| -> Result<(), String> {
            let hooks = install(&ctx, &init.info, Some(&init.plugin), editor)
                .map_err(|e| format!("cannot set up QuickJS: {e}"))?;
            *vm.hooks.borrow_mut() = Some(Persistent::save(&ctx, hooks));
            Ok(())
        })?;
        loop {
            let invoke = link.borrow_mut().next_invoke()?;
            let Some(invoke) = invoke else {
                return Ok(());
            };
            let outcome = vm
                .context
                .with(|ctx| run_handler(&ctx, &invoke, &vm.hooks))
                .and_then(|()| vm.finish());
            let answer = match outcome {
                Ok(()) => Ok(serde_json::json!({"ok": true})),
                Err(message) => Err(RpcError::new(HANDLER_FAILED, message)),
            };
            link.borrow_mut().respond(&invoke.id, answer)?;
        }
    }
}

/// The code of the error answer to an invoke whose handler failed.
pub const HANDLER_FAILED: i64 = -32000;

/// Runs one handler: tells the boot the jack and its arguments, then evaluates the text.
fn run_handler(
    ctx: &Ctx<'_>,
    invoke: &Invoke,
    saved: &RefCell<Option<Hooks>>,
) -> Result<(), String> {
    let hooks = restore_hooks(ctx, saved)?;
    let begin: Function = hooks.get("begin").map_err(|e| e.to_string())?;
    let args = json_to_js(ctx, &invoke.args).map_err(|e| e.to_string())?;
    begin
        .call::<_, ()>((invoke.jack.as_str(), args))
        .map_err(|e| e.to_string())?;
    let options = named(&invoke.name.replace('\0', "?"));
    let result = ctx.eval_with_options::<Value, _>(invoke.source.as_str(), options);
    match result.catch(ctx) {
        Ok(_) => Ok(()),
        Err(rquickjs::CaughtError::Exception(exception)) => {
            Err(describe_value(&hooks, exception.into_value()))
        }
        Err(rquickjs::CaughtError::Value(value)) => Err(describe_value(&hooks, value)),
        Err(rquickjs::CaughtError::Error(error)) => Err(error.to_string()),
    }
}

fn restore_hooks<'js>(
    ctx: &Ctx<'js>,
    saved: &RefCell<Option<Hooks>>,
) -> Result<Object<'js>, String> {
    let saved = saved.borrow().clone().ok_or("the boot is gone")?;
    saved.restore(ctx).map_err(|e| e.to_string())
}

/// Binds the API and runs the script. The error is the text for the standard error.
fn start(
    ctx: &Ctx<'_>,
    init: &Initialize,
    editor: Rc<RefCell<dyn Editor>>,
    saved: &RefCell<Option<Hooks>>,
) -> Result<(), String> {
    let hooks = install(ctx, &init.info, None, editor)
        .map_err(|e| format!("cannot set up QuickJS: {e}"))?;
    *saved.borrow_mut() = Some(Persistent::save(ctx, hooks.clone()));

    // The name is what the stack shows (`name:3:5`). A NUL cannot be in a C string.
    let options = named(&init.script.name.replace('\0', "?"));
    let result = ctx.eval_with_options::<Value, _>(init.script.source.as_str(), options);
    match result.catch(ctx) {
        Ok(_) => Ok(()),
        Err(rquickjs::CaughtError::Exception(exception)) => {
            Err(describe_value(&hooks, exception.into_value()))
        }
        Err(rquickjs::CaughtError::Value(value)) => Err(describe_value(&hooks, value)),
        Err(rquickjs::CaughtError::Error(error)) => Err(error.to_string()),
    }
}

/// The text for the standard error: what `describe` in `quickjs.js` makes of the value.
fn describe_value<'js>(hooks: &Object<'js>, value: Value<'js>) -> String {
    hooks
        .get::<_, Function>("describe")
        .and_then(|describe| describe.call::<_, String>((value,)))
        .unwrap_or_else(|e| format!("an error that cannot be shown: {e}"))
}

fn named(filename: &str) -> EvalOptions {
    let mut options = EvalOptions::default();
    options.filename = Some(filename.to_string());
    options
}

/// Binds the macro API and returns the object of the boot (`describe`, `track`, `unhandled`).
fn install<'js>(
    ctx: &Ctx<'js>,
    info: &Json,
    plugin: Option<&Json>,
    editor: Rc<RefCell<dyn Editor>>,
) -> rquickjs::Result<Object<'js>> {
    // The script's output goes through Rust and is flushed: the requests are written to the same
    // pipe by Rust and have to stay in order with it.
    let write = Function::new(ctx.clone(), |fd: i32, text: String| {
        // A closed pipe means the editor has gone; there is nobody to read it.
        let _ = if fd == 2 {
            let mut out = std::io::stderr().lock();
            out.write_all(text.as_bytes()).and_then(|()| out.flush())
        } else {
            let mut out = std::io::stdout().lock();
            out.write_all(text.as_bytes()).and_then(|()| out.flush())
        };
    })?;

    let raw = Object::new(ctx.clone())?;
    let mut names = Vec::with_capacity(entries().len());
    for func in entries() {
        names.push(func.name);
        raw.set(func.name, bind(ctx, func.name, Some(func), editor.clone())?)?;
    }
    let info = json_to_js(ctx, info)?;
    // The `Plugin` object: the calls the editor answers itself (the definition file, the options).
    let plugin = match plugin {
        None => Value::new_null(ctx.clone()),
        Some(plugin) => {
            let object = Object::new(ctx.clone())?;
            let plugin_raw = Object::new(ctx.clone())?;
            for name in PLUGIN_CALLS {
                plugin_raw.set(
                    name.trim_start_matches("Plugin."),
                    bind(ctx, name, None, editor.clone())?,
                )?;
            }
            object.set("raw", plugin_raw)?;
            object.set(
                "names",
                PLUGIN_CALLS
                    .iter()
                    .map(|n| n.trim_start_matches("Plugin."))
                    .collect::<Vec<_>>(),
            )?;
            object.set("id", json_to_js(ctx, &plugin["id"])?)?;
            object.set("dir", json_to_js(ctx, &plugin["dir"])?)?;
            object.into_value()
        }
    };
    let boot: Function = ctx.eval_with_options(BOOT, named("matsu-script:boot"))?;
    boot.call((raw, names, info, write, plugin))
}

/// The calls of the `Plugin` object (Sakura's `CPluginIfObj`: `help/plugin/Text/implementation02.html`).
/// They go to the editor under these names; the editor reads the definition file and the options
/// of the plugin.
pub const PLUGIN_CALLS: &[&str] = &[
    "Plugin.GetPluginDir",
    "Plugin.GetDef",
    "Plugin.GetOption",
    "Plugin.SetOption",
    "Plugin.GetString",
    "Plugin.GetCommandNo",
];

/// One function of the macro API (or a call of the `Plugin` object, which has no entry) as a Rust
/// function for JavaScript. It takes the array of the arguments and answers `[true, result]` or
/// `[false, code, message]`.
fn bind<'js>(
    ctx: &Ctx<'js>,
    name: &'static str,
    func: Option<&'static Entry>,
    editor: Rc<RefCell<dyn Editor>>,
) -> rquickjs::Result<Function<'js>> {
    Function::new(ctx.clone(), move |ctx: Ctx<'js>, args: Array<'js>| {
        let failed = |error: RpcError| -> rquickjs::Result<Array<'js>> {
            let answer = Array::new(ctx.clone())?;
            answer.set(0, false)?;
            answer.set(1, error.code)?;
            answer.set(2, error.message)?;
            Ok(answer)
        };
        let mut params = Vec::with_capacity(args.len());
        for (index, arg) in args.iter::<Value>().enumerate() {
            match js_to_json(&arg?, 0) {
                Ok(value) => params.push(value),
                Err(got) => {
                    let (param, ty) = func
                        .and_then(|func| func.params.get(index))
                        .map_or(("?", "a value"), |p| (p.name, p.ty.name()));
                    return failed(RpcError::invalid_params(format!(
                        "{name}: argument \"{param}\" must be {ty}, got {got}"
                    )));
                }
            }
        }
        let answer = editor.borrow_mut().call(name, params);
        match answer {
            Ok(result) => {
                let answer = Array::new(ctx.clone())?;
                answer.set(0, true)?;
                answer.set(1, json_to_js(&ctx, &result)?)?;
                Ok(answer)
            }
            Err(CallError::Rpc(error)) => failed(error),
            // The connection is gone: an ordinary error that ends the macro.
            Err(CallError::Closed(message)) => Err(Exception::throw_message(&ctx, &message)),
        }
    })
}

/// A JavaScript value as JSON, or what the value is when it has no JSON form (a function, a
/// symbol, a string with a lone surrogate...). `undefined` inside an array or an object is
/// `null`, as `JSON.stringify` has it for an array.
/// A float with a whole value becomes an integer, because the script cannot tell `2` from `2.0`.
fn js_to_json(value: &Value<'_>, depth: usize) -> Result<Json, String> {
    if depth > MAX_DEPTH {
        return Err("a value nested too deeply (or a cycle)".into());
    }
    Ok(match value.type_of() {
        Type::Undefined | Type::Null => Json::Null,
        Type::Bool => Json::Bool(value.as_bool().unwrap_or_default()),
        Type::Int => Json::from(value.as_int().unwrap_or_default()),
        Type::Float => float_to_json(value.as_float().unwrap_or_default()),
        Type::String => {
            let string = value.as_string().ok_or("string")?;
            // Not a lossy conversion: text with characters replaced must not reach the document.
            Json::String(
                string
                    .to_string()
                    .map_err(|_| "a string that is not valid UTF-16 (a lone surrogate)")?,
            )
        }
        Type::Array => {
            let array = value.as_array().ok_or("array")?;
            let mut items = Vec::with_capacity(array.len());
            for item in array.iter::<Value>() {
                items.push(js_to_json(&item.map_err(|e| e.to_string())?, depth + 1)?);
            }
            Json::Array(items)
        }
        // A plain object; a function, a symbol, a promise and the like have their own types.
        Type::Object => {
            let object = value.as_object().ok_or("object")?;
            let mut map = serde_json::Map::new();
            for entry in object.props::<String, Value>() {
                let (key, item) = entry.map_err(|e| e.to_string())?;
                map.insert(key, js_to_json(&item, depth + 1)?);
            }
            Json::Object(map)
        }
        other => return Err(js_type_name(other).into()),
    })
}

/// The name that `typeof` would give.
fn js_type_name(ty: Type) -> &'static str {
    match ty {
        Type::Symbol => "symbol",
        Type::BigInt => "bigint",
        Type::Function | Type::Constructor => "function",
        other => other.as_str(),
    }
}

fn float_to_json(n: f64) -> Json {
    // 2^63 is the first float above the i64 range.
    if n.fract() == 0.0 && n.abs() < 2f64.powi(63) {
        Json::from(n as i64)
    } else {
        // NaN and infinity have no JSON form: null, which the editor rejects for a number.
        Number::from_f64(n).map_or(Json::Null, Json::Number)
    }
}

/// JSON as a JavaScript value. `null` is `null`.
fn json_to_js<'js>(ctx: &Ctx<'js>, value: &Json) -> rquickjs::Result<Value<'js>> {
    Ok(match value {
        Json::Null => Value::new_null(ctx.clone()),
        Json::Bool(b) => Value::new_bool(ctx.clone(), *b),
        // An integer beyond 2^53 loses its low digits, as it does in any JavaScript.
        Json::Number(n) => match n.as_i64() {
            Some(i) => i.into_js(ctx)?,
            None => n.as_f64().unwrap_or_default().into_js(ctx)?,
        },
        Json::String(s) => s.as_str().into_js(ctx)?,
        Json::Array(items) => {
            let array = Array::new(ctx.clone())?;
            for (i, item) in items.iter().enumerate() {
                array.set(i, json_to_js(ctx, item)?)?;
            }
            array.into_value()
        }
        Json::Object(map) => {
            let object = Object::new(ctx.clone())?;
            for (key, item) in map {
                object.set(key.as_str(), json_to_js(ctx, item)?)?;
            }
            object.into_value()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_context<R>(f: impl FnOnce(&Ctx<'_>) -> R) -> R {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| f(&ctx))
    }

    fn to_json(code: &str) -> Result<Json, String> {
        with_context(|ctx| {
            let value: Value = ctx.eval(code).unwrap();
            js_to_json(&value, 0)
        })
    }

    #[test]
    fn javascript_values_become_json() {
        assert_eq!(to_json("null"), Ok(Json::Null));
        assert_eq!(to_json("undefined"), Ok(Json::Null));
        assert_eq!(to_json("true"), Ok(Json::Bool(true)));
        assert_eq!(to_json("4 / 2"), Ok(Json::from(2)));
        assert_eq!(to_json("2 ** 40"), Ok(Json::from(1_i64 << 40)));
        assert_eq!(to_json("0.5"), Ok(Json::from(0.5)));
        assert_eq!(to_json("NaN"), Ok(Json::Null));
        assert_eq!(to_json("'日本語😀'"), Ok(Json::from("日本語😀")));
        assert_eq!(
            to_json("[1, 'a', [null]]"),
            Ok(serde_json::json!([1, "a", [null]]))
        );
        assert_eq!(
            to_json("({a: 1, b: {c: 'd'}})"),
            Ok(serde_json::json!({"a": 1, "b": {"c": "d"}}))
        );
    }

    #[test]
    fn what_has_no_json_form_is_named() {
        assert_eq!(to_json("(() => 1)"), Err("function".into()));
        assert_eq!(to_json("Symbol('s')"), Err("symbol".into()));
        assert_eq!(to_json("1n"), Err("bigint".into()));
        assert!(
            to_json("'a\\ud800'")
                .unwrap_err()
                .contains("lone surrogate")
        );
        assert!(
            to_json("(() => { const a = []; a.push(a); return a; })()")
                .unwrap_err()
                .contains("cycle")
        );
    }

    #[test]
    fn json_comes_back_as_javascript() {
        with_context(|ctx| {
            let json = serde_json::json!({"n": 3, "big": 1_i64 << 40, "f": 1.5, "s": "あ😀", "a": [null, true]});
            let value = json_to_js(ctx, &json).unwrap();
            assert_eq!(js_to_json(&value, 0), Ok(json));
        });
    }
}
