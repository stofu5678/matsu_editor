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

    /// Reads the first line, which is the `initialize` notification.
    pub fn read_initialize(&mut self) -> Result<Initialize, String> {
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
        Ok(Initialize {
            info,
            script: Script {
                name: field("name")?,
                source: field("source")?,
            },
        })
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
    fn a_request_is_one_ascii_line() {
        let mut c = connection("");
        let _ = c.call("InsText", vec![json!("日本語😀\n")]);
        let written = String::from_utf8(c.output.clone()).unwrap();
        assert!(written.is_ascii());
        assert_eq!(written.matches('\n').count(), 1);
    }
}
