//! The macro side of the JSON-RPC protocol (docs/poc/json-rpc.md): read the `initialize`
//! notification, send requests and wait for their responses. It does what `boot.cjs` and
//! `boot.py` do in their languages.

use std::io::{self, BufRead, Write};

use matsu_plugin::protocol::{self, RS, RpcError};
use serde_json::{Value, json};

/// Why a call did not return a result.
#[derive(Debug)]
pub enum CallError {
    /// The editor answered with an error object.
    Rpc(RpcError),
    /// The editor closed the connection, or it cannot be written to.
    Closed(String),
}

/// What a script can ask of the editor. The engines call this; `Connection` implements it.
pub trait Editor {
    fn call(&mut self, method: &str, params: Vec<Value>) -> Result<Value, CallError>;
}

/// The script that the editor handed over in `initialize` (`params.script`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The name that errors show: the path of the file.
    pub name: String,
    pub source: String,
}

/// What `initialize` told the host.
#[derive(Debug)]
pub struct Initialize {
    /// `params` without `script`: what the script sees as `Editor.info`.
    pub info: Value,
    pub script: Script,
}

/// How the editor started the host: a macro that runs once, or a plugin that waits for calls
/// (`params.kind` of `initialize`, ADR-0026).
#[derive(Debug)]
pub enum Start {
    Macro(Initialize),
    Plugin(PluginInit),
}

/// What `initialize` told a plugin host.
#[derive(Debug)]
pub struct PluginInit {
    /// `params` without `plugin`: what the plugin sees as `Editor.info`.
    pub info: Value,
    /// `params.plugin`: `{"id": ..., "dir": ...}` of the plugin.
    pub plugin: Value,
}

/// A call of a handler (`plugin/invoke`): the editor reads the file of the handler and sends its
/// text, so that a path in any character set works (as for a macro).
#[derive(Debug, Clone, PartialEq)]
pub struct Invoke {
    /// The id to answer under.
    pub id: Value,
    pub jack: String,
    /// The name that errors show: the path of the file.
    pub name: String,
    pub source: String,
    /// What the jack hands to the handler (`Plugin.args`).
    pub args: Value,
}

/// The connection of a plugin host: the calls from the editor, and what a script asks of the
/// editor (`Editor`).
pub trait PluginLink: Editor {
    /// The next `plugin/invoke`, or `None` when the editor closed the input or sent
    /// `plugin/shutdown`. Other messages are skipped.
    fn next_invoke(&mut self) -> Result<Option<Invoke>, String>;

    /// Answers an invoke: `Ok` when the handler ended, the error with the text of what went wrong
    /// otherwise.
    fn respond(&mut self, id: &Value, outcome: Result<Value, RpcError>) -> Result<(), String>;
}

pub struct Connection<R: BufRead, W: Write> {
    input: R,
    output: W,
    next_id: i64,
}

impl<R: BufRead, W: Write> Connection<R, W> {
    pub fn new(input: R, output: W) -> Self {
        Self {
            input,
            output,
            next_id: 1,
        }
    }

    /// One line from the editor without its line break, or `None` at the end of the input.
    /// Bytes that are not UTF-8 become U+FFFD (the editor sends ASCII only).
    fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut buffer = Vec::new();
        if self.input.read_until(b'\n', &mut buffer)? == 0 {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&buffer);
        Ok(Some(protocol::trim_line_end(&text).to_string()))
    }

    /// Reads the first line, which is the `initialize` notification, for a macro.
    pub fn read_initialize(&mut self) -> Result<Initialize, String> {
        match self.read_start()? {
            Start::Macro(init) => Ok(init),
            Start::Plugin(_) => Err("initialize is for a plugin, not for a macro".into()),
        }
    }

    /// Reads the first line, which is the `initialize` notification. A plugin says so with
    /// `params.kind` and brings no script.
    pub fn read_start(&mut self) -> Result<Start, String> {
        let line = self
            .read_line()
            .map_err(|e| format!("cannot read initialize: {e}"))?
            .ok_or("the editor closed the connection before it sent initialize")?;
        let message: Value =
            serde_json::from_str(&line).map_err(|e| format!("initialize is not JSON: {e}"))?;
        if message["method"] != "initialize" {
            return Err("the first message must be initialize".into());
        }
        let mut info = message["params"].clone();
        if info["kind"] == "plugin" {
            let plugin = info
                .as_object_mut()
                .and_then(|params| params.remove("plugin"))
                .ok_or("initialize of a plugin carries no plugin (params.plugin)")?;
            return Ok(Start::Plugin(PluginInit { info, plugin }));
        }
        let script = info
            .as_object_mut()
            .and_then(|params| params.remove("script"))
            .ok_or("initialize carries no script (params.script)")?;
        let field = |key: &str| {
            script[key]
                .as_str()
                .map(str::to_string)
                .ok_or(format!("params.script.{key} must be a string"))
        };
        Ok(Start::Macro(Initialize {
            info,
            script: Script {
                name: field("name")?,
                source: field("source")?,
            },
        }))
    }
}

impl<R: BufRead, W: Write> PluginLink for Connection<R, W> {
    fn next_invoke(&mut self) -> Result<Option<Invoke>, String> {
        loop {
            let Some(text) = self
                .read_line()
                .map_err(|e| format!("cannot read from the editor: {e}"))?
            else {
                return Ok(None);
            };
            let Ok(message) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            match message["method"].as_str() {
                Some("plugin/shutdown") => return Ok(None),
                Some("plugin/invoke") => {
                    let params = &message["params"];
                    let field = |key: &str| params[key].as_str().unwrap_or_default().to_owned();
                    return Ok(Some(Invoke {
                        id: message["id"].clone(),
                        jack: field("jack"),
                        name: field("name"),
                        source: field("source"),
                        args: params.get("args").cloned().unwrap_or(Value::Null),
                    }));
                }
                _ => {}
            }
        }
    }

    fn respond(&mut self, id: &Value, outcome: Result<Value, RpcError>) -> Result<(), String> {
        let message = match outcome {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(error) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": error.code, "message": error.message},
            }),
        };
        let line = format!("{RS}{}\n", protocol::ascii_json(&message));
        self.output
            .write_all(line.as_bytes())
            .and_then(|()| self.output.flush())
            .map_err(|e| format!("cannot write to the editor: {e}"))
    }
}

/// An [`Editor`] that is a plugin link: the engines bind the macro API to an `Editor`.
pub struct LinkEditor(pub std::rc::Rc<std::cell::RefCell<dyn PluginLink>>);

impl Editor for LinkEditor {
    fn call(&mut self, method: &str, params: Vec<Value>) -> Result<Value, CallError> {
        self.0.borrow_mut().call(method, params)
    }
}

impl<R: BufRead, W: Write> Editor for Connection<R, W> {
    /// Sends a request and waits for its response. Messages that are not the answer (a
    /// notification, or an answer to something that is no longer waited for) are skipped.
    fn call(&mut self, method: &str, params: Vec<Value>) -> Result<Value, CallError> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        // RS first, so that the editor splits the line there when the script has printed text
        // without a line break just before. The user's output is written and flushed by the
        // script's own `print`, so it is already ahead of this line.
        let line = format!("{RS}{}\n", protocol::ascii_json(&request));
        self.output
            .write_all(line.as_bytes())
            .and_then(|()| self.output.flush())
            .map_err(|e| CallError::Closed(format!("cannot write to the editor: {e}")))?;
        loop {
            let text = match self.read_line() {
                Ok(Some(text)) => text,
                Ok(None) => {
                    return Err(CallError::Closed(format!(
                        "The editor closed the connection before it answered {method}"
                    )));
                }
                Err(e) => {
                    return Err(CallError::Closed(format!(
                        "cannot read from the editor: {e}"
                    )));
                }
            };
            if text.is_empty() {
                continue;
            }
            let Ok(message) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            if message["id"] != id {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(CallError::Rpc(RpcError::new(
                    error["code"].as_i64().unwrap_or(protocol::INTERNAL_ERROR),
                    error["message"].as_str().unwrap_or_default(),
                )));
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(input: &str) -> Connection<&[u8], Vec<u8>> {
        Connection::new(input.as_bytes(), Vec::new())
    }

    #[test]
    fn reads_the_script_out_of_initialize() {
        let mut c = connection(concat!(
            r#"{"jsonrpc":"2.0","method":"initialize","params":{"apiVersion":1,"#,
            r#""script":{"name":"C:\\\u3042.lua","source":"print(1)\r\n"}}}"#,
            "\n"
        ));
        let init = c.read_initialize().unwrap();
        assert_eq!(init.script.name, "C:\\あ.lua");
        assert_eq!(init.script.source, "print(1)\r\n");
        assert_eq!(init.info, json!({"apiVersion": 1}));
    }

    #[test]
    fn initialize_without_a_script_is_refused() {
        let mut c = connection("{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{}}\n");
        assert!(c.read_initialize().is_err());
        assert!(connection("").read_initialize().is_err());
        assert!(
            connection("{\"method\":\"other\"}\n")
                .read_initialize()
                .is_err()
        );
    }

    #[test]
    fn a_call_skips_other_messages_and_reads_the_result() {
        let mut c = connection(concat!(
            "\r\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"note\"}\r\n",
            "{\"jsonrpc\":\"2.0\",\"id\":99,\"result\":1}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\"\\u3042\"}\n",
        ));
        assert_eq!(c.call("GetLineStr", vec![json!(1)]).unwrap(), json!("あ"));
        let written = String::from_utf8(c.output.clone()).unwrap();
        assert!(
            written.starts_with(RS) && written.ends_with('\n'),
            "{written:?}"
        );
        let sent: Value = serde_json::from_str(&written[RS.len_utf8()..]).unwrap();
        assert_eq!(sent["id"], 1);
        assert_eq!(sent["method"], "GetLineStr");
    }

    #[test]
    fn an_error_response_and_a_closed_input_are_told_apart() {
        let mut c = connection(
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32602,\"message\":\"bad\"}}\n",
        );
        match c.call("Jump", vec![]) {
            Err(CallError::Rpc(e)) => assert_eq!((e.code, e.message.as_str()), (-32602, "bad")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(c.call("Up", vec![]), Err(CallError::Closed(_))));
    }

    #[test]
    fn a_plugin_starts_without_a_script_and_a_macro_is_not_a_plugin() {
        let mut c = connection(concat!(
            r#"{"jsonrpc":"2.0","method":"initialize","params":{"apiVersion":1,"kind":"plugin","#,
            r#""plugin":{"id":"a.b","dir":"C:\\p"}}}"#,
            "\n"
        ));
        match c.read_start().unwrap() {
            Start::Plugin(init) => {
                assert_eq!(init.plugin["id"], "a.b");
                assert_eq!(init.info, json!({"apiVersion": 1, "kind": "plugin"}));
            }
            other => panic!("{other:?}"),
        }
        let mut c = connection(
            "{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{\"kind\":\"plugin\"}}\n",
        );
        assert!(c.read_start().is_err(), "a plugin without params.plugin");
        let mut c = connection(
            "{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{\"kind\":\"plugin\",\"plugin\":{}}}\n",
        );
        assert!(
            c.read_initialize().is_err(),
            "a macro start refuses a plugin"
        );
    }

    #[test]
    fn invokes_are_read_one_after_the_other_and_shutdown_ends() {
        let mut c = connection(concat!(
            "{\"method\":\"note\"}\n",
            "garbage\n",
            "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"plugin/invoke\",\"params\":{\"jack\":\"Command\",",
            "\"name\":\"C:\\\\あ.js\",\"source\":\"1\",\"args\":{\"n\":1}}}\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"plugin/shutdown\"}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"plugin/invoke\",\"params\":{}}\n",
        ));
        let invoke = c.next_invoke().unwrap().unwrap();
        assert_eq!(invoke.id, json!(7));
        assert_eq!(invoke.jack, "Command");
        assert_eq!(invoke.name, "C:\\あ.js");
        assert_eq!(invoke.args, json!({"n": 1}));
        assert_eq!(c.next_invoke().unwrap(), None);
        // The end of the input ends it too.
        assert_eq!(connection("").next_invoke().unwrap(), None);
    }

    #[test]
    fn an_answer_to_an_invoke_is_one_ascii_line_with_the_rs() {
        let mut c = connection("");
        c.respond(&json!(7), Ok(json!({"ok": true}))).unwrap();
        c.respond(&json!(8), Err(RpcError::new(-32000, "あ")))
            .unwrap();
        let written = String::from_utf8(c.output.clone()).unwrap();
        assert!(written.is_ascii());
        let lines: Vec<_> = written.lines().collect();
        assert_eq!(lines.len(), 2);
        let first: Value = serde_json::from_str(&lines[0][RS.len_utf8()..]).unwrap();
        assert_eq!(first["id"], 7);
        assert_eq!(first["result"]["ok"], true);
        let second: Value = serde_json::from_str(&lines[1][RS.len_utf8()..]).unwrap();
        assert_eq!(second["error"]["message"], "あ");
    }

    #[test]
    fn a_request_is_one_ascii_line() {
        let mut c = connection("");
        let _ = c.call("InsText", vec![json!("日本語😀\n")]);
        let written = String::from_utf8(c.output.clone()).unwrap();
        assert!(written.is_ascii());
        assert_eq!(written.matches('\n').count(), 1);
    }
}
