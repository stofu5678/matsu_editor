//! The wire format of the macro protocol (docs/poc/json-rpc.md): one JSON-RPC 2.0 message per
//! line, ASCII only, and an RS (0x1E) in front of every message that the macro writes.

use serde_json::{Map, Value, json};

/// The RS character that starts a protocol message in the macro's output.
pub const RS: char = '\u{1e}';

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

/// A JSON-RPC error object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(INVALID_PARAMS, message)
    }
}

/// A request or notification from the macro.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// `None` for a notification, which gets no response.
    pub id: Option<Value>,
    pub method: String,
    pub params: Vec<Value>,
}

/// A message that cannot be run, with the id to answer it under (`null` when unknown).
#[derive(Debug, Clone, PartialEq)]
pub struct Rejection {
    pub id: Value,
    pub error: RpcError,
}

/// One line of the macro's standard output, split at the first RS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitLine<'a> {
    /// What the user's code printed before the message (or the whole line when there is none).
    pub user: Option<&'a str>,
    /// The JSON-RPC message after the RS.
    pub message: Option<&'a str>,
}

/// Splits a line at the first RS. The text before it is user output, which a macro may have
/// written without a line break just before the request.
pub fn split_output_line(line: &str) -> SplitLine<'_> {
    match line.split_once(RS) {
        // An empty line is the user's too (`print()`).
        None => SplitLine {
            user: Some(line),
            message: None,
        },
        Some((user, message)) => SplitLine {
            user: (!user.is_empty()).then_some(user),
            message: Some(message),
        },
    }
}

/// Removes the line break (LF, or CR LF) at the end of a line that was read.
pub fn trim_line_end(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

/// Parses one message from the macro.
pub fn parse_request(text: &str) -> Result<Request, Rejection> {
    let reject = |id: Value, code: i64, message: &str| Rejection {
        id,
        error: RpcError::new(code, message),
    };
    let value: Value = serde_json::from_str(text)
        .map_err(|e| reject(Value::Null, PARSE_ERROR, &format!("parse error: {e}")))?;
    let Value::Object(mut object) = value else {
        return Err(reject(
            Value::Null,
            INVALID_REQUEST,
            "the request must be a JSON object",
        ));
    };
    // The id is echoed only when it has a shape that a response may carry.
    let id = match object.get("id") {
        None => None,
        Some(id) if id.is_string() || id.is_i64() || id.is_u64() => Some(id.clone()),
        Some(_) => {
            return Err(reject(
                Value::Null,
                INVALID_REQUEST,
                "the id must be an integer or a string",
            ));
        }
    };
    let answer_id = id.clone().unwrap_or(Value::Null);
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(reject(
            answer_id,
            INVALID_REQUEST,
            "\"jsonrpc\" must be \"2.0\"",
        ));
    }
    let Some(Value::String(method)) = object.remove("method") else {
        return Err(reject(
            answer_id,
            INVALID_REQUEST,
            "\"method\" must be a string",
        ));
    };
    let params = match object.remove("params") {
        None => Vec::new(),
        Some(Value::Array(params)) => params,
        Some(_) => {
            return Err(reject(
                answer_id,
                INVALID_PARAMS,
                "\"params\" must be an array",
            ));
        }
    };
    Ok(Request { id, method, params })
}

/// Escapes every non-ASCII character as `\uXXXX`, with surrogate pairs for U+10000 and above.
/// DEL (0x7F) is escaped too. Only JSON text goes through this, where non-ASCII characters can
/// only be inside strings.
pub fn escape_non_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut units = [0u16; 2];
    for ch in text.chars() {
        if ch.is_ascii() && ch != '\u{7f}' {
            out.push(ch);
        } else {
            for unit in ch.encode_utf16(&mut units) {
                out.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    out
}

/// A JSON value as one ASCII line (without the line break).
pub fn ascii_json(value: &Value) -> String {
    escape_non_ascii(&value.to_string())
}

/// The response to a request that succeeded.
pub fn response_ok(id: &Value, result: Value) -> String {
    ascii_json(&json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

/// The response to a request that failed.
pub fn response_error(id: &Value, error: &RpcError) -> String {
    ascii_json(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": error.code, "message": error.message},
    }))
}

/// What the editor tells the macro about itself (`initialize`, a notification).
#[derive(Debug, Clone)]
pub struct InitializeInfo {
    pub editor_name: String,
    pub editor_version: String,
    pub process_id: u32,
    /// The path of the document, or `None` when it is not a file.
    pub document_path: Option<String>,
    /// The user's script, for a host that does not open the file itself (the bundled script
    /// host: a Windows program opens a path with the ANSI code page, which cannot name every
    /// file). `None` for the interpreters that get the path as an argument.
    pub script: Option<ScriptSource>,
    /// Set when the process is a plugin host (ADR-0026): it brings no script and waits for
    /// `plugin/invoke` calls.
    pub plugin: Option<PluginIdentity>,
}

/// Which plugin a host serves, as `params.plugin` of `initialize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginIdentity {
    pub id: String,
    /// The folder of the plugin.
    pub dir: String,
}

/// The name and the text of the user's script, as `params.script` of `initialize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptSource {
    /// The name that errors show (the path of the file).
    pub name: String,
    pub source: String,
}

pub const PROTOCOL_VERSION: u32 = 1;
pub const API_VERSION: u32 = 1;

/// The `initialize` notification (spec section 3), as one ASCII line.
pub fn initialize_message(info: &InitializeInfo) -> String {
    let mut document = Map::new();
    document.insert(
        "path".into(),
        info.document_path
            .clone()
            .map_or(Value::Null, Value::String),
    );
    let mut params = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "processId": info.process_id,
        "editor": {"name": info.editor_name, "version": info.editor_version},
        "apiVersion": API_VERSION,
        "document": document,
    });
    if let Some(script) = &info.script {
        params["script"] = json!({"name": script.name, "source": script.source});
    }
    if let Some(plugin) = &info.plugin {
        params["kind"] = json!("plugin");
        params["plugin"] = json!({"id": plugin.id, "dir": plugin.dir});
    }
    ascii_json(&json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": params,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_the_first_rs() {
        let line = format!("abc{RS}{{\"x\":1}}");
        let split = split_output_line(&line);
        assert_eq!(split.user, Some("abc"));
        assert_eq!(split.message, Some("{\"x\":1}"));
        let only_message = format!("{RS}{{}}");
        let split = split_output_line(&only_message);
        assert_eq!((split.user, split.message), (None, Some("{}")));
        let split = split_output_line("plain output");
        assert_eq!((split.user, split.message), (Some("plain output"), None));
        let split = split_output_line("");
        assert_eq!((split.user, split.message), (Some(""), None));
        // A second RS belongs to the message.
        let line = format!("a{RS}b{RS}c");
        assert_eq!(split_output_line(&line).message, Some(&line[2..]));
    }

    #[test]
    fn trims_lf_and_crlf() {
        assert_eq!(trim_line_end("a\r\n"), "a");
        assert_eq!(trim_line_end("a\n"), "a");
        assert_eq!(trim_line_end("a"), "a");
        // Only one CR goes: a CR before it is part of the text.
        assert_eq!(trim_line_end("a\r\r\n"), "a\r");
    }

    #[test]
    fn escapes_non_ascii_with_surrogate_pairs() {
        assert_eq!(escape_non_ascii("abc"), "abc");
        assert_eq!(escape_non_ascii("あ"), "\\u3042");
        assert_eq!(escape_non_ascii("😀"), "\\ud83d\\ude00");
        assert_eq!(escape_non_ascii("\u{7f}"), "\\u007f");
        let line = response_ok(&json!(1), json!("日本語😀"));
        assert!(line.is_ascii());
        assert!(
            line.contains(r#""result":"\u65e5\u672c\u8a9e\ud83d\ude00""#),
            "{line}"
        );
        // The escaped line parses back to the same text.
        let back: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(back["result"], "日本語😀");
    }

    #[test]
    fn control_characters_stay_escaped_in_one_line() {
        let line = response_ok(&json!(2), json!("a\r\nb\u{1e}c"));
        assert!(!line.contains('\n') && !line.contains('\r') && !line.contains(RS));
        let back: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(back["result"], "a\r\nb\u{1e}c");
    }

    #[test]
    fn parses_a_request() {
        let request = parse_request(
            r#"{"jsonrpc":"2.0","id":7,"method":"InsText","params":["\u3042\ud83d\ude00"]}"#,
        )
        .unwrap();
        assert_eq!(request.id, Some(json!(7)));
        assert_eq!(request.method, "InsText");
        assert_eq!(request.params, vec![json!("あ😀")]);
    }

    #[test]
    fn a_missing_params_is_empty_and_a_missing_id_is_a_notification() {
        let request = parse_request(r#"{"jsonrpc":"2.0","method":"Up"}"#).unwrap();
        assert_eq!(request.id, None);
        assert!(request.params.is_empty());
    }

    #[test]
    fn malformed_input_gets_the_standard_codes() {
        let code = |text: &str| parse_request(text).unwrap_err().error.code;
        assert_eq!(code("{not json"), PARSE_ERROR);
        assert_eq!(code(""), PARSE_ERROR);
        assert_eq!(code("[1,2]"), INVALID_REQUEST);
        assert_eq!(code(r#"{"id":1,"method":"Up"}"#), INVALID_REQUEST);
        assert_eq!(
            code(r#"{"jsonrpc":"1.0","id":1,"method":"Up"}"#),
            INVALID_REQUEST
        );
        assert_eq!(
            code(r#"{"jsonrpc":"2.0","id":1,"method":5}"#),
            INVALID_REQUEST
        );
        assert_eq!(
            code(r#"{"jsonrpc":"2.0","id":1.5,"method":"Up"}"#),
            INVALID_REQUEST
        );
        assert_eq!(
            code(r#"{"jsonrpc":"2.0","id":1,"method":"Up","params":{"a":1}}"#),
            INVALID_PARAMS
        );
    }

    #[test]
    fn a_rejection_keeps_the_id_when_it_is_known() {
        let rejection = parse_request(r#"{"jsonrpc":"2.0","id":9,"method":3}"#).unwrap_err();
        assert_eq!(rejection.id, json!(9));
        let rejection = parse_request("oops").unwrap_err();
        assert_eq!(rejection.id, Value::Null);
    }

    #[test]
    fn initialize_is_one_ascii_line() {
        let line = initialize_message(&InitializeInfo {
            editor_name: "MATSU Editor".into(),
            editor_version: "0.0.0".into(),
            process_id: 1234,
            document_path: Some("C:\\work\\あ.txt".into()),
            script: Some(ScriptSource {
                name: "C:\\work\\絵文字😀.lua".into(),
                source: "print('あ')\n".into(),
            }),
            plugin: None,
        });
        assert!(line.is_ascii() && !line.contains('\n'));
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["method"], "initialize");
        assert!(value.get("id").is_none());
        assert_eq!(value["params"]["protocolVersion"], 1);
        assert_eq!(value["params"]["document"]["path"], "C:\\work\\あ.txt");
        assert_eq!(value["params"]["script"]["name"], "C:\\work\\絵文字😀.lua");
        assert_eq!(value["params"]["script"]["source"], "print('あ')\n");
        let none = initialize_message(&InitializeInfo {
            editor_name: "MATSU Editor".into(),
            editor_version: "0.0.0".into(),
            process_id: 1,
            document_path: None,
            script: None,
            plugin: None,
        });
        assert!(none.contains(r#""path":null"#));
        assert!(!none.contains("script") && !none.contains("plugin"));
        let plugin = initialize_message(&InitializeInfo {
            editor_name: "MATSU Editor".into(),
            editor_version: "0.0.0".into(),
            process_id: 1,
            document_path: None,
            script: None,
            plugin: Some(PluginIdentity {
                id: "jp.example.あ".into(),
                dir: "C:\\plugins\\あ".into(),
            }),
        });
        assert!(plugin.is_ascii());
        let value: Value = serde_json::from_str(&plugin).unwrap();
        assert_eq!(value["params"]["kind"], "plugin");
        assert_eq!(value["params"]["plugin"]["id"], "jp.example.あ");
        assert_eq!(value["params"]["plugin"]["dir"], "C:\\plugins\\あ");
        assert!(value["params"].get("script").is_none());
    }
}
