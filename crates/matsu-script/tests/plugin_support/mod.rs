//! What the tests of the plugin host (ADR-0026) share: the built `matsu-script` program serves a
//! plugin under the editor's own host code (`matsu_plugin::plugin_host`), against a plain text
//! model. Each test crate uses a part of it.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use matsu_plugin::api;
use matsu_plugin::model::TextModel;
use matsu_plugin::plugin_host::{HostSettings, PluginApi, PluginHost, Report, respond};
use matsu_plugin::protocol::RpcError;
use matsu_plugin::session::{Engine, Language, Stream};
use serde_json::{Value, json};

pub const PATIENT: Duration = Duration::from_secs(30);

/// What the editor answers to the `Plugin` object in these tests.
#[derive(Default)]
pub struct Api {
    pub options: BTreeMap<(String, String), String>,
    pub calls: Vec<String>,
    /// The candidates of `Complement.AddList`.
    pub added: Vec<String>,
}

impl PluginApi for Api {
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError> {
        self.calls.push(name.to_owned());
        let text = |i: usize| {
            params
                .get(i)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        match name {
            "Plugin.GetPluginDir" => Ok(json!("/plugins/テスト")),
            "Plugin.GetDef" => Ok(json!(format!("def:{}:{}", text(0), text(1)))),
            "Plugin.GetOption" => Ok(json!(
                self.options
                    .get(&(text(0), text(1)))
                    .cloned()
                    .unwrap_or_default()
            )),
            "Plugin.SetOption" => {
                self.options.insert((text(0), text(1)), text(2));
                Ok(json!(true))
            }
            "Plugin.GetString" => Ok(json!(format!("string {}", params[0]))),
            // The `Outline` object: the editor takes the calls as they are.
            n if n.starts_with("Outline.") => Ok(json!(null)),
            // Sakura's `AddKouhoUnique`: the size of the list, or -1 for a word it has.
            "Complement.AddList" => {
                let word = text(0);
                if word.is_empty() {
                    return Err(RpcError::invalid_params("empty"));
                }
                if self.added.contains(&word) {
                    return Ok(json!(-1));
                }
                self.added.push(word);
                Ok(json!(self.added.len()))
            }
            other => Err(RpcError::new(-32601, format!("no such call: {other}"))),
        }
    }
}

pub fn plugin_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, code) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, code).unwrap();
    }
    dir
}

/// A host of the bundled `engine` for the plugin in `dir`.
pub fn host_of(engine: Engine, dir: &Path, resident: bool) -> PluginHost {
    let mut settings =
        HostSettings::new("jp.example.test", dir, Language::Bundled(engine), resident);
    settings.interpreter = env!("CARGO_BIN_EXE_matsu-script").into();
    PluginHost::new(settings)
}

pub struct World {
    pub doc: TextModel,
    pub api: Api,
}

impl World {
    pub fn new(text: &str) -> Self {
        Self {
            doc: TextModel::new(text, Some("/work/テスト.txt".into())),
            api: Api::default(),
        }
    }

    pub fn call(&mut self, host: &mut PluginHost, handler: &str, timeout: Duration) -> Report {
        self.call_jack(host, "Command", handler, &json!({"n": 1}), timeout)
    }

    /// A call of the jack `jack` with `args`.
    pub fn call_jack(
        &mut self,
        host: &mut PluginHost,
        jack: &str,
        handler: &str,
        args: &Value,
        timeout: Duration,
    ) -> Report {
        let doc = &mut self.doc;
        let api_state = &mut self.api;
        host.run_blocking(jack, handler, args, timeout, |text| {
            respond(text, api_state, |method, params| {
                api::call(doc, method, params)
            })
        })
        .expect("the call begins")
    }
}

pub fn stdout(report: &Report) -> Vec<&str> {
    report.lines(Stream::Stdout)
}
