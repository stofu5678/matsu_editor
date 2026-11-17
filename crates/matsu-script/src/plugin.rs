//! What the engines share when they serve a plugin (ADR-0026): the loop that answers each
//! `plugin/invoke`, the names of the calls that the editor answers for the plugin objects, and
//! the calls that an engine whose loop runs inside its language uses to reach the connection.

use std::cell::RefCell;
use std::rc::Rc;

use matsu_plugin::protocol::RpcError;
use serde_json::{Value, json};

use crate::rpc::{Invoke, PluginLink};

/// The code of the error answer to an invoke whose handler failed.
pub const HANDLER_FAILED: i64 = -32000;

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
    "Plugin.AddCommand",
];

/// The calls of the `Outline` object (Sakura's `COutlineIfObj`, `sakura_core/plugin/COutlineIfObj.h`;
/// `help/plugin/Text/implementation02.html`). The editor answers them, and only while an `Outline`
/// plug runs.
pub const OUTLINE_CALLS: &[&str] = &[
    "Outline.AddFuncInfo",
    "Outline.AddFuncInfo2",
    "Outline.AddFuncInfo3",
    "Outline.AddFuncInfo4",
    "Outline.SetTitle",
    "Outline.SetListType",
    "Outline.SetLabel",
];

/// The calls of the `Complement` object that the editor answers (Sakura's `CComplementIfObj`,
/// `sakura_core/plugin/CComplementIfObj.h`). `GetCurrentWord` and `GetOption` need nothing from the
/// editor: the word and the option come with the invoke (`Plugin.args`).
pub const COMPLEMENT_CALLS: &[&str] = &["Complement.AddList"];

/// Every call of the plugin objects that goes to the editor.
pub fn object_calls() -> impl Iterator<Item = &'static str> {
    PLUGIN_CALLS
        .iter()
        .chain(OUTLINE_CALLS)
        .chain(COMPLEMENT_CALLS)
        .copied()
}

/// The object and the member of a call name (`Plugin.GetDef` is `("Plugin", "GetDef")`).
pub fn split(name: &str) -> (&str, &str) {
    name.split_once('.').unwrap_or_default()
}

/// The char that the `Indent` object answers for the jack `SmartIndent`: the typed char, `"\r"` for
/// the Enter key as in Sakura (`CSmartIndentIfObj`). Empty when `args` has none.
pub fn typed_char(args: &Value) -> &str {
    args["char"].as_str().unwrap_or_default()
}

/// Answers every `plugin/invoke` until the editor ends the connection: `handle` runs one handler
/// and returns the text of its error, if it failed. A handler that failed does not stop the loop.
pub fn serve_each(
    link: &Rc<RefCell<dyn PluginLink>>,
    mut handle: impl FnMut(&Invoke) -> Result<(), String>,
) -> Result<(), String> {
    loop {
        let invoke = link.borrow_mut().next_invoke()?;
        let Some(invoke) = invoke else {
            return Ok(());
        };
        let answer = answer_of(handle(&invoke));
        link.borrow_mut().respond(&invoke.id, answer)?;
    }
}

/// An invoke as the object that `plugin/next` answers (the fields are moved, not copied: the
/// source of a handler can be large).
fn invoke_value(invoke: Invoke) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("id".into(), invoke.id);
    object.insert("jack".into(), Value::String(invoke.jack));
    object.insert("name".into(), Value::String(invoke.name));
    object.insert("source".into(), Value::String(invoke.source));
    object.insert("args".into(), invoke.args);
    Value::Object(object)
}

fn answer_of(outcome: Result<(), String>) -> Result<Value, RpcError> {
    match outcome {
        Ok(()) => Ok(json!({"ok": true})),
        Err(message) => Err(RpcError::new(HANDLER_FAILED, message)),
    }
}

/// The calls that are not for the editor but for the connection. An engine whose loop runs in its
/// own language (MicroPython and mruby, whose namespaces for the handler files are easiest to keep
/// there) asks the host for the next invoke and answers it through these. They have a slash, so no
/// function of the macro API has the name.
pub mod reserved {
    /// `plugin/identity`: `{"id": ..., "dir": ...}` of the plugin.
    pub const IDENTITY: &str = "plugin/identity";
    /// `plugin/next`: the next invoke as `{"id", "jack", "name", "source", "args"}`, or null when
    /// the editor ended the conversation.
    pub const NEXT: &str = "plugin/next";
    /// `plugin/respond`: `[id, error]` answers the invoke; `error` is null when the handler ended.
    pub const RESPOND: &str = "plugin/respond";
}

/// What an engine whose loop is in its language needs from the connection.
pub struct Reserved {
    link: Rc<RefCell<dyn PluginLink>>,
    identity: Value,
}

impl Reserved {
    /// `plugin` is `params.plugin` of `initialize`; the answer to `plugin/identity` is that with
    /// `members` added: the names of the calls of each object (`Plugin`, `Outline`, `Complement`).
    pub fn new(link: Rc<RefCell<dyn PluginLink>>, plugin: &Value) -> Self {
        let mut identity = plugin.clone();
        let mut members = serde_json::Map::new();
        for name in object_calls() {
            let (object, member) = split(name);
            if let Some(list) = members
                .entry(object)
                .or_insert_with(|| json!([]))
                .as_array_mut()
            {
                list.push(json!(member));
            }
        }
        if let Some(object) = identity.as_object_mut() {
            object.insert("members".into(), Value::Object(members));
        }
        Self { link, identity }
    }

    /// Answers a reserved call, or `None` when `name` is a call for the editor.
    /// The `Err` is the text of a failure that ends the host (the connection is gone).
    pub fn answer(&self, name: &str, params: &[Value]) -> Option<Result<Value, String>> {
        Some(match name {
            reserved::IDENTITY => Ok(self.identity.clone()),
            reserved::NEXT => self
                .link
                .borrow_mut()
                .next_invoke()
                .map(|next| next.map_or(Value::Null, invoke_value)),
            reserved::RESPOND => {
                let id = params.first().cloned().unwrap_or(Value::Null);
                let outcome = match params.get(1).and_then(Value::as_str) {
                    None => Ok(()),
                    Some(message) => Err(message.to_owned()),
                };
                self.link
                    .borrow_mut()
                    .respond(&id, answer_of(outcome))
                    .map(|()| Value::Null)
            }
            _ => return None,
        })
    }
}
