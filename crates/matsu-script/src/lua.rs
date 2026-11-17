//! The Lua engine: Lua 5.4 (`mlua`, with the C source built in), running the user's script with
//! `Editor` and `EditorError` in the global scope, as the JavaScript and Python boots do.
//!
//! Every function of `Editor` comes from `matsu_plugin::catalog::entries()` (the basic API and every row of Sakura's macro tables) and is bound as a Rust
//! function. A small Lua part (`BOOT`) only turns the answer of a Rust function into a return
//! value or into a thrown `EditorError`, because a Rust function cannot throw a table.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

use matsu_plugin::catalog::{Entry, entries};
use matsu_plugin::protocol::RpcError;
use mlua::{Function, MultiValue, Table, Value as LuaValue, Variadic};
use serde_json::{Number, Value};

use crate::engine::Engine;
use crate::plugin::{self, serve_each};
use crate::rpc::{CallError, Editor, Initialize, Invoke, LinkEditor, PluginInit, PluginLink};

pub struct Lua;

/// Makes `Editor` out of the Rust functions of `raw`:
///
/// - a function answers `true, result` or `false, code, message, where`;
/// - an error becomes `error(EditorError, 0)`, an object with `code`, `message`, `where` (the
///   place of the call, as Lua's own errors show it) and a `__tostring`, so that an uncaught one
///   ends the macro with `name:3: EditorError -32602: ...` just like a Lua error.
const BOOT: &str = r#"
local raw, names, info = ...
local EditorError = {name = "EditorError"}
EditorError.__index = EditorError
EditorError.__tostring = function(e)
  return e.where .. " EditorError " .. e.code .. ": " .. e.message
end
local Editor = {info = info}
for _, name in ipairs(names) do
  local call = raw[name]
  Editor[name] = function(...)
    local ok, a, b, c = call(...)
    if ok then
      return a
    end
    error(setmetatable({code = a, message = b, where = c}, EditorError), 0)
  end
end
Editor.info = info
_G.Editor = Editor
_G.EditorError = EditorError
"#;

/// Makes the objects of a plugin out of the Rust functions of `raw` (one for each full name,
/// `Plugin.GetDef`), and returns `Plugin`; `members` lists the names of each object:
///
/// - `Plugin` has the calls the editor answers, and `id`, `dir`, `jack` and `args` (the handler's
///   arguments, set before each call);
/// - `Outline` has the calls of the jack `Outline`;
/// - `Complement` has `GetCurrentWord` and `GetOption`, which read `Plugin.args`, and `AddList`.
///
/// Every function answers like those of `Editor` (see `BOOT`), so an error is an `EditorError`.
const PLUGIN_BOOT: &str = r#"
local EditorError, raw, members, identity = ...
local function object(prefix)
  local o = {}
  for _, name in ipairs(members[prefix]) do
    local call = raw[prefix .. "." .. name]
    o[name] = function(...)
      local ok, a, b, c = call(...)
      if ok then
        return a
      end
      error(setmetatable({code = a, message = b, where = c}, EditorError), 0)
    end
  end
  return o
end
local Plugin = object("Plugin")
Plugin.id = identity.id
Plugin.dir = identity.dir
Plugin.jack = ""
local Outline = object("Outline")
local Complement = object("Complement")
-- 0x01: ignore the case of letters; 0x02: the words of the document are a source.
function Complement.GetCurrentWord()
  local args = Plugin.args
  if type(args) == "table" and type(args.word) == "string" then
    return args.word
  end
  return ""
end
function Complement.GetOption()
  local args = Plugin.args
  if type(args) == "table" and math.type(args.option) == "integer" then
    return args.option
  end
  return 0
end
_G.Plugin = Plugin
_G.Outline = Outline
_G.Complement = Complement
return Plugin
"#;

impl Engine for Lua {
    /// A plugin: one Lua state for each handler file, for as long as the host lives, so that the
    /// globals a handler leaves are there for the next call of the same file and no other file sees
    /// them (ADR-0026).
    fn serve(&self, init: &PluginInit, link: Rc<RefCell<dyn PluginLink>>) -> Result<(), String> {
        let editor: Rc<RefCell<dyn Editor>> = Rc::new(RefCell::new(LinkEditor(link.clone())));
        let mut states: HashMap<String, PluginState> = HashMap::new();
        serve_each(&link, |invoke| {
            if !states.contains_key(&invoke.name) {
                let state = PluginState::new(init, &editor)
                    .map_err(|e| format!("cannot set up Lua: {e}"))?;
                states.insert(invoke.name.clone(), state);
            }
            states[&invoke.name].run(invoke).map_err(|e| e.to_string())
        })
    }

    fn run(&self, init: &Initialize, editor: Rc<RefCell<dyn Editor>>) -> Result<(), String> {
        let lua = mlua::Lua::new();
        install(&lua, &init.info, &editor).map_err(|e| format!("cannot set up Lua: {e}"))?;
        let main = lua
            .load(prepare_source(&init.script.source))
            .set_name(format!("@{}", init.script.name))
            .into_function()
            .map_err(|e| e.to_string())?;
        main.call::<()>(()).map_err(|e| e.to_string())
    }
}

/// The standalone `lua` skips a first line that starts with `#` (a shebang); `load` does not.
/// The line stays, as a comment, so that the line numbers are the file's.
fn prepare_source(source: &str) -> String {
    match source.strip_prefix('#') {
        Some(rest) => format!("--{rest}"),
        None => source.to_string(),
    }
}

/// The error that the handler's `os.exit` raises.
#[derive(Debug)]
struct HandlerExit;

impl std::fmt::Display for HandlerExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("os.exit")
    }
}

impl std::error::Error for HandlerExit {}

/// The Lua state of one handler file of a plugin.
struct PluginState {
    lua: mlua::Lua,
    /// The `Plugin` object: `jack` and `args` are set before each call.
    plugin: Table,
}

impl PluginState {
    fn new(init: &PluginInit, editor: &Rc<RefCell<dyn Editor>>) -> mlua::Result<Self> {
        let lua = mlua::Lua::new();
        install(&lua, &init.info, editor)?;
        // `os.exit` would end the whole host (and the state of every plugin file with it): in a
        // handler it ends the handler only, with no failure, as `sys.exit()` does in Python.
        // It raises an error of its own type, which the end of the chunk recognizes (a handler may
        // catch it with `pcall`, and then it simply goes on, as `except SystemExit` does).
        let exit = lua.create_function(|_, _: Variadic<LuaValue>| -> mlua::Result<()> {
            Err(mlua::Error::external(HandlerExit))
        })?;
        let os: Table = lua.globals().get("os")?;
        os.set("exit", exit)?;
        let raw = lua.create_table()?;
        let members = lua.create_table()?;
        for object in ["Plugin", "Outline", "Complement"] {
            members.set(object, lua.create_table()?)?;
        }
        for name in plugin::object_calls() {
            raw.set(name, bind(&lua, name, None, editor.clone())?)?;
            let (object, member) = plugin::split(name);
            let list: Table = members.get(object)?;
            list.push(member)?;
        }
        let identity = json_to_lua(&lua, &init.plugin)?;
        let error: LuaValue = lua.globals().get("EditorError")?;
        let plugin = lua
            .load(PLUGIN_BOOT)
            .set_name("=matsu-script plugin boot")
            .call((error, raw, members, identity))?;
        Ok(Self { lua, plugin })
    }

    /// Runs the text of a handler in the globals of this state.
    fn run(&self, invoke: &Invoke) -> mlua::Result<()> {
        self.plugin.set("jack", invoke.jack.as_str())?;
        self.plugin
            .set("args", json_to_lua(&self.lua, &invoke.args)?)?;
        // The `Indent` object of the jack `SmartIndent` (Sakura's `CSmartIndentIfObj`): `GetChar`
        // answers the typed char.
        let indent = if invoke.jack == "SmartIndent" {
            let typed = plugin::typed_char(&invoke.args).to_owned();
            let object = self.lua.create_table()?;
            object.set(
                "GetChar",
                self.lua.create_function(move |_, ()| Ok(typed.clone()))?,
            )?;
            LuaValue::Table(object)
        } else {
            LuaValue::Nil
        };
        self.lua.globals().set("Indent", indent)?;
        let result = self
            .lua
            .load(prepare_source(&invoke.source))
            .set_name(format!("@{}", invoke.name))
            .into_function()?
            .call::<()>(());
        match result {
            Err(error) if error.downcast_ref::<HandlerExit>().is_some() => Ok(()),
            other => other,
        }
    }
}

fn install(lua: &mlua::Lua, info: &Value, editor: &Rc<RefCell<dyn Editor>>) -> mlua::Result<()> {
    // The script's `io.write` goes to the C stdout, which must not hold text back: the
    // requests are written to the same pipe by Rust and have to stay in order with it.
    lua.load("io.stdout:setvbuf('no'); io.stderr:setvbuf('no')")
        .exec()?;

    // `print` writes through Rust and flushes, for the same reason. It is Lua's `print`
    // otherwise: every argument through `tostring` (so `__tostring` works), tab-separated.
    let tostring: Function = lua.globals().get("tostring")?;
    let print = lua.create_function(move |_, args: Variadic<LuaValue>| {
        let mut line = Vec::new();
        for (i, arg) in args.into_iter().enumerate() {
            if i > 0 {
                line.push(b'\t');
            }
            let text: mlua::LuaString = tostring.call(arg)?;
            line.extend_from_slice(&text.as_bytes());
        }
        line.push(b'\n');
        // A closed pipe means the editor has gone; there is nobody to read it.
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(&line).and_then(|()| out.flush());
        Ok(())
    })?;
    lua.globals().set("print", print)?;

    let raw = lua.create_table()?;
    let mut names = Vec::with_capacity(entries().len());
    for func in entries() {
        names.push(func.name);
        raw.set(func.name, bind(lua, func.name, Some(func), editor.clone())?)?;
    }
    let info = json_to_lua(lua, info)?;
    lua.load(BOOT)
        .set_name("=matsu-script boot")
        .call::<()>((raw, names, info))
}

/// One function of the macro API (or a call of a plugin object, which has no entry) as a Rust
/// function for Lua.
fn bind(
    lua: &mlua::Lua,
    name: &'static str,
    func: Option<&'static Entry>,
    editor: Rc<RefCell<dyn Editor>>,
) -> mlua::Result<Function> {
    lua.create_function(move |lua, args: Variadic<LuaValue>| {
        let failed = |error: RpcError| -> mlua::Result<MultiValue> {
            Ok(MultiValue::from_iter([
                LuaValue::Boolean(false),
                LuaValue::Integer(error.code),
                LuaValue::String(lua.create_string(&error.message)?),
                LuaValue::String(lua.create_string(caller_location(lua))?),
            ]))
        };
        // `Editor.Jump(2, nil)` means `Editor.Jump(2)`.
        let mut args = args.into_iter().collect::<Vec<_>>();
        while matches!(args.last(), Some(LuaValue::Nil)) {
            args.pop();
        }
        let mut params = Vec::with_capacity(args.len());
        for (index, arg) in args.iter().enumerate() {
            // `Complement.AddList(2)` adds "2", as `String(word)` does in JavaScript.
            let converted = match arg {
                LuaValue::Integer(_) | LuaValue::Number(_) | LuaValue::Boolean(_)
                    if name == "Complement.AddList" =>
                {
                    arg.to_string()
                        .map(Value::String)
                        .map_err(|e| e.to_string())
                }
                _ => lua_to_json(arg),
            };
            match converted {
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
            Ok(result) => Ok(MultiValue::from_iter([
                LuaValue::Boolean(true),
                json_to_lua(lua, &result)?,
            ])),
            Err(CallError::Rpc(error)) => failed(error),
            // The connection is gone: an ordinary Lua error that ends the macro.
            Err(CallError::Closed(message)) => Err(mlua::Error::runtime(message)),
        }
    })
}

/// Where the script called an `Editor` function, like the front of a Lua error message
/// (`name:3:`). Level 0 is the Rust function and 1 the wrapper in `BOOT`; the script is the
/// next Lua function (`pcall(Editor.Jump, ...)` puts the C function `pcall` in between).
fn caller_location(lua: &mlua::Lua) -> String {
    (2..)
        .map_while(|level| {
            lua.inspect_stack(level, |frame| {
                let source = frame.source();
                if source.what == "C" {
                    return None;
                }
                let name = match source.source.as_deref() {
                    Some(s) if s.starts_with(['@', '=']) => s[1..].to_string(),
                    _ => source.short_src.map(|s| s.to_string()).unwrap_or_default(),
                };
                Some(match frame.current_line() {
                    Some(line) => format!("{name}:{line}:"),
                    None => format!("{name}:"),
                })
            })
        })
        .find_map(|location| location)
        .unwrap_or_default()
}

/// A Lua value as JSON, or what the value is when it has no JSON form (a table, a function, a
/// string that is not UTF-8...).
/// A float with a whole value becomes an integer, because Lua writes `2.0` for what the script
/// meant as 2 (the result of a division, for one).
fn lua_to_json(value: &LuaValue) -> Result<Value, String> {
    Ok(match value {
        LuaValue::Nil => Value::Null,
        LuaValue::Boolean(b) => Value::Bool(*b),
        LuaValue::Integer(i) => Value::from(*i),
        LuaValue::Number(n) => float_to_json(*n),
        // Not `to_string_lossy`: text with bytes replaced must not reach the document.
        LuaValue::String(s) => match s.to_str() {
            Ok(text) => Value::String(text.to_string()),
            Err(_) => return Err("a string that is not valid UTF-8".into()),
        },
        other => return Err(other.type_name().into()),
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

/// JSON as a Lua value. `null` is `nil`, so a `null` inside an array leaves a hole.
fn json_to_lua(lua: &mlua::Lua, value: &Value) -> mlua::Result<LuaValue> {
    Ok(match value {
        Value::Null => LuaValue::Nil,
        Value::Bool(b) => LuaValue::Boolean(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => LuaValue::Integer(i),
            None => LuaValue::Number(n.as_f64().unwrap_or_default()),
        },
        Value::String(s) => LuaValue::String(lua.create_string(s)?),
        Value::Array(items) => {
            let table: Table = lua.create_table()?;
            for (i, item) in items.iter().enumerate() {
                table.raw_set(i + 1, json_to_lua(lua, item)?)?;
            }
            LuaValue::Table(table)
        }
        Value::Object(map) => {
            let table: Table = lua.create_table()?;
            for (key, item) in map {
                table.raw_set(key.as_str(), json_to_lua(lua, item)?)?;
            }
            LuaValue::Table(table)
        }
    })
}
