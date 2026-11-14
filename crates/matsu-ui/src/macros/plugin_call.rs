//! A call of a plugin on a window (ADR-0026): the hosts that stay between two calls, the answers
//! to what a plugin asks (`Editor.*` on the document of the window, `Plugin.*` from the
//! definition file and the options), and the loop that follows a call to its end.
//!
//! The host process and its time limits are `matsu_plugin::plugin_host`; this is the part that
//! needs the window. What is here is the smallest that makes a plugin run end to end: a call
//! of the first command of a plugin from the settings page (試しに実行). The commands, menus and
//! keys (PR 4) and the events (PR 6) call [`run`] the same way.
//!
//! A plugin runs only when the user turned plugins on (`EnablePlugin`), the plugin is registered,
//! and the user enabled it with the files as they are ([`prepare_command`] checks all three).
//!
//! Not done here: a call that waits for the user (a message box, a file dialog, `Sleep`) is
//! answered with an error. Stopping a call by hand is not on a button yet; the time limit stops it.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, Context, Entity, Global};
use matsu_commands::plugin_commands::{self, CommandInfo};
use matsu_commands::registry::Registry;
use matsu_config::Profile;
use matsu_config::plugins::consent::{Consent, ConsentFile};
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::options::PluginOptions;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginDef, SearchDirs};
use matsu_plugin::bridge;
use matsu_plugin::plugin_host::{
    self, HostSettings, MAX_FAILURES, Outcome, PluginApi, PluginHost, Progress, limit_of,
};
use matsu_plugin::protocol::RpcError;
use matsu_plugin::session::{Engine, Language, Stream};
use matsu_plugin::{Value, json};

use super::output::Kind;
use crate::plugin_outline::{Collected, Ending};
use crate::view::EditorView;

mod indent;

thread_local! {
    /// The plugins that three failures in a row stopped, for the list of the settings page (which
    /// has no window to ask). Only the UI thread uses it.
    static STOPPED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

thread_local! {
    /// The failures in a row of each plugin, over all the languages of its handlers (the hosts count
    /// their own too, but the plugin is what stops: ADR-0026).
    static FAILURES: RefCell<HashMap<String, u32>> = RefCell::new(HashMap::new());
}

/// Counts the end of a call of the plugin `id`: a success starts the count over, a failure adds to
/// it, and [`MAX_FAILURES`] in a row stop the plugin (all its hosts, whatever the language). Returns
/// whether the plugin is stopped now.
pub(crate) fn note_outcome(id: &str, outcome: &Outcome) -> bool {
    FAILURES.with(|map| {
        let mut map = map.borrow_mut();
        if *outcome == Outcome::Ok {
            map.remove(id);
        } else if outcome.is_failure() {
            let count = map.entry(id.to_owned()).or_insert(0);
            *count += 1;
            if *count >= MAX_FAILURES {
                set_stopped(id, true);
            }
        }
    });
    is_stopped(id)
}

/// Whether three failures in a row stopped the plugin `id`.
pub(crate) fn is_stopped(id: &str) -> bool {
    STOPPED.with(|set| set.borrow().contains(id))
}

pub(crate) fn set_stopped(id: &str, stopped: bool) {
    STOPPED.with(|set| {
        if stopped {
            set.borrow_mut().insert(id.to_owned());
        } else {
            set.borrow_mut().remove(id);
        }
    });
}

type SharedHost = Rc<RefCell<PluginHost>>;

/// The hosts, by the ID of the plugin and the language of the handler (a plugin may have handlers in
/// more than one language, and each language has a process of its own). A host is made at the first
/// call.
#[derive(Default)]
struct Runtime {
    hosts: HashMap<(String, Language), SharedHost>,
}

impl Runtime {
    /// The hosts of one plugin.
    fn of<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a SharedHost> {
        self.hosts
            .iter()
            .filter(move |((host_id, _), _)| host_id == id)
            .map(|(_, host)| host)
    }

    /// Takes the hosts of one plugin out.
    fn take(&mut self, id: &str) -> Vec<SharedHost> {
        let keys: Vec<_> = self
            .hosts
            .keys()
            .filter(|(host_id, _)| host_id == id)
            .cloned()
            .collect();
        keys.iter()
            .filter_map(|key| self.hosts.remove(key))
            .collect()
    }
}

impl Global for Runtime {}

/// A success of the plugin starts the failure count of every host of it over: only the count of
/// the plugin stops it, so the hosts' own counts (which stop a host alone) must never get ahead of it.
fn forgive_hosts(cx: &mut App, id: &str) {
    for host in cx.default_global::<Runtime>().of(id) {
        host.borrow_mut().reset();
    }
}

/// The user enabled the plugin again: the failures start over.
pub(crate) fn reset(cx: &mut App, id: &str) {
    set_stopped(id, false);
    FAILURES.with(|map| map.borrow_mut().remove(id));
    for host in cx.default_global::<Runtime>().of(id) {
        host.borrow_mut().reset();
    }
}

/// The plugin is off or gone: its process ends.
pub(crate) fn shutdown(cx: &mut App, id: &str) {
    set_stopped(id, false);
    FAILURES.with(|map| map.borrow_mut().remove(id));
    for host in cx.default_global::<Runtime>().take(id) {
        host.borrow_mut().shutdown();
    }
}

/// The editor quits: every plugin process ends now, also one whose handler loops (nothing but a
/// kill stops it, and it would go on using the processor after the editor is gone).
pub(crate) fn shutdown_all(cx: &mut App) {
    for (_, host) in cx.default_global::<Runtime>().hosts.drain() {
        host.borrow_mut().shutdown_now();
    }
}

/// What the plugin asks of the editor besides the document (`Plugin.*`): the definition file and
/// the options of the plugin (Sakura's `CPluginIfObj`, `sakura_core/plugin/CPluginIfObj.h`).
pub(crate) struct DefApi {
    dir: PathBuf,
    main: Profile,
    def: PluginDef,
    options: PluginOptions,
    option_file: PathBuf,
    command_no: usize,
    /// The slot of the plugin in the plugin table, which `Plugin.AddCommand` needs for the code of the
    /// command it adds (`None`: the plugin is not in the table, as in a test of the object alone).
    slot: Option<usize>,
    /// What the `Outline` object is given, in a call of the jack `Outline` (`None`: no other call may
    /// use the object).
    outline: Option<Rc<RefCell<Collected>>>,
    /// The list that `Complement.AddList` fills, while the call is one of a completion jack.
    complement: Option<crate::plugin_complement::Shared>,
}

impl DefApi {
    pub(crate) fn new(dirs: &SearchDirs, folder: &str, def: PluginDef, command_no: usize) -> Self {
        let dir = dirs.plugin_dir(folder).unwrap_or_default();
        let main =
            Profile::load(&dir.join(matsu_config::plugins::DEF_FILE_NAME)).unwrap_or_default();
        Self {
            options: PluginOptions::load(&dirs.option_file(folder)),
            option_file: dirs.option_file(folder),
            dir,
            main,
            def,
            command_no,
            slot: None,
            outline: None,
            complement: None,
        }
    }

    pub(crate) fn in_slot(mut self, slot: usize) -> Self {
        self.slot = Some(slot);
        self
    }

    /// `F_PL_ADDCOMMAND` (`CPlugin::AddCommand` with `doRegister`): the plugin gets one more command,
    /// the next number after the ones it has. Its handler is a file of the folder of the plugin, so that
    /// what the user agreed to (the files of the folder) covers it.
    fn add_command(&mut self, params: &[Value]) -> Result<Value, RpcError> {
        let name = "Plugin.AddCommand";
        let handler = text_at(params, 0, name)?;
        let label = params
            .get(1)
            .and_then(Value::as_str)
            .map_or_else(|| handler.to_owned(), str::to_owned);
        let label = if label.is_empty() {
            handler.to_owned()
        } else {
            label
        };
        let icon = params
            .get(2)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let path = Path::new(handler);
        let inside = !handler.is_empty()
            && path.is_relative()
            && path
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_)));
        if !inside {
            return Err(RpcError::invalid_params(format!(
                "{name}: the handler must be a file inside the folder of the plugin: {handler}"
            )));
        }
        let Some(slot) = self.slot else {
            return Err(RpcError::new(
                plugin_host_error(),
                format!("{name}: the plugin is not registered"),
            ));
        };
        // A plugin that adds its command when a window starts (EditorStart) is called for each window,
        // and Sakura's editors each have their own table: here the same command is added once.
        if plugin_commands::commands()
            .iter()
            .any(|c| c.added && c.slot == slot && c.handler == handler && c.label == label)
        {
            return Ok(Value::Null);
        }
        let plug = plugin_commands::commands()
            .iter()
            .filter(|c| c.slot == slot)
            .map(|c| c.plug)
            .max()
            .unwrap_or(0)
            + 1;
        let Some(code) = plugin_commands::function_code(slot, plug) else {
            return Err(RpcError::new(
                plugin_host_error(),
                format!(
                    "{name}: a plugin can have {} commands",
                    plugin_commands::MAX_PLUG_CMD - 1
                ),
            ));
        };
        plugin_commands::add_command(CommandInfo {
            code,
            slot,
            plug,
            plugin_id: self.def.id.clone(),
            plugin_name: if self.def.name.is_empty() {
                self.def.id.clone()
            } else {
                self.def.name.clone()
            },
            label,
            handler: handler.to_owned(),
            icon,
            added: true,
        });
        Ok(Value::Null)
    }
}

fn text_at<'p>(params: &'p [Value], index: usize, name: &str) -> Result<&'p str, RpcError> {
    params.get(index).and_then(Value::as_str).ok_or_else(|| {
        RpcError::invalid_params(format!("{name}: argument {} must be a string", index + 1))
    })
}

impl PluginApi for DefApi {
    fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError> {
        match name {
            // `F_PL_GETPLUGINDIR`
            "Plugin.GetPluginDir" => Ok(json!(self.dir.display().to_string())),
            // `F_PL_GETDEF`: a value of `plugin.def`; nothing is an empty string.
            "Plugin.GetDef" => {
                let (section, key) = (text_at(params, 0, name)?, text_at(params, 1, name)?);
                Ok(json!(self.main.get(section, key).unwrap_or_default()))
            }
            // `F_PL_GETOPTION`: the value in the options file; if there is none, the default of
            // the option in `plugin.def`.
            "Plugin.GetOption" => {
                // Read at each call, as Sakura does: the option dialog may have written it.
                self.options = PluginOptions::load(&self.option_file);
                let (section, key) = (text_at(params, 0, name)?, text_at(params, 1, name)?);
                let value = self
                    .options
                    .get(section, key)
                    .map(str::to_owned)
                    .or_else(|| {
                        self.def
                            .options
                            .iter()
                            .find(|o| o.section == section && o.key == key)
                            .map(|o| o.default.clone())
                    });
                Ok(json!(value.unwrap_or_default()))
            }
            // `F_PL_SETOPTION`: writes the options file at once.
            "Plugin.SetOption" => {
                self.options = PluginOptions::load(&self.option_file);
                let section = text_at(params, 0, name)?;
                let key = text_at(params, 1, name)?;
                let value = match params.get(2) {
                    Some(Value::String(text)) => Cow::Borrowed(text.as_str()),
                    Some(Value::Null) | None => {
                        return Err(RpcError::invalid_params(format!(
                            "{name}: argument 3 must be a value"
                        )));
                    }
                    Some(other) => Cow::Owned(other.to_string()),
                };
                self.options
                    .set(section, key, &value)
                    .map_err(|e| RpcError::invalid_params(format!("{name}: {e}")))?;
                self.options
                    .save()
                    .map_err(|e| RpcError::new(plugin_host_error(), format!("{name}: {e}")))?;
                Ok(Value::Null)
            }
            "Plugin.AddCommand" => self.add_command(params),
            // The `Outline` object (`COutlineIfObj`): in a plug of the jack `Outline` only.
            other if other.starts_with("Outline.") => match &self.outline {
                Some(collected) => collected.borrow_mut().call(other, params),
                None => Err(crate::plugin_outline::not_in_an_outline(other)),
            },
            // `F_CM_ADDLIST` (`CComplementIfObj`): the number of candidates now, or -1 if the list
            // has the word already.
            "Complement.AddList" => {
                let Some(list) = &self.complement else {
                    return Err(RpcError::new(
                        plugin_host_error(),
                        format!("{name}: this call is not one of a completion"),
                    ));
                };
                let word = text_at(params, 0, name)?;
                if word.is_empty() {
                    return Err(RpcError::invalid_params(format!(
                        "{name}: the candidate must not be empty"
                    )));
                }
                Ok(json!(list.borrow_mut().add(word)))
            }
            // `F_PL_GETCOMMANDNO`
            "Plugin.GetCommandNo" => Ok(json!(self.command_no)),
            // `F_PL_GETSTRING`: 0 is the language; `S[n]` of `plugin.def` for 1 to 99.
            "Plugin.GetString" => {
                let number = params.first().and_then(Value::as_i64).ok_or_else(|| {
                    RpcError::invalid_params(format!("{name}: argument 1 must be a number"))
                })?;
                Ok(json!(match number {
                    0 => DEFAULT_LANGUAGE.to_owned(),
                    1..=99 => self
                        .def
                        .strings
                        .get(number as usize)
                        .cloned()
                        .unwrap_or_default(),
                    _ => String::new(),
                }))
            }
            other => Err(RpcError::new(
                matsu_plugin::protocol::METHOD_NOT_FOUND,
                format!("no such call: {other}"),
            )),
        }
    }
}

/// The error code of a failure of the editor's own side.
fn plugin_host_error() -> i64 {
    matsu_plugin::protocol::INTERNAL_ERROR
}

/// Everything a call needs, checked: the host, the handler and the answers.
pub(crate) struct Prepared {
    pub(crate) label: String,
    host: SharedHost,
    jack: &'static str,
    handler: String,
    args: Value,
    api: DefApi,
    /// The jack of an event (ADR-0026, PR 6; `None`: a command): an event runs quietly when it goes
    /// well, and the caller may wait for it ([`EditorView::plugin_event_running`]). With the folder
    /// and `handler` it is what [`Prepared::refresh`] prepares again. The result of a call of the jack
    /// `Outline` and the list of a completion are in `api` ([`prepare_outline`],
    /// [`Prepared::for_complement`]).
    event: Option<Jack>,
    /// The folder of the plugin.
    folder: String,
    /// A call of a completion jack: the session of the list it fills (the list is `api.complement`).
    complement: Option<u64>,
}

impl Prepared {
    /// The ID of the plugin.
    pub(crate) fn id(&self) -> &str {
        &self.api.def.id
    }

    /// Makes this the call of a completion jack for the word `word`: `Complement.GetCurrentWord`
    /// gives the word, `GetOption` the option, and `AddList` adds to `list`. The window shows what is
    /// added while the call runs (`session` names the list).
    pub(crate) fn for_complement(
        mut self,
        session: u64,
        word: &str,
        option: i64,
        list: crate::plugin_complement::Shared,
    ) -> Self {
        self.args = json!({"word": word, "option": option});
        self.api.complement = Some(list);
        self.complement = Some(session);
        self
    }

    /// An event call that waited for its turn is checked again right before it begins: the plugin may
    /// have been switched off, taken away or changed meanwhile, and the agreement of the user is of the
    /// files as they are at the moment the handler runs (ADR-0026). `None`: it must not run.
    pub(crate) fn refresh(self, cx: &mut App, dirs: &SearchDirs) -> Option<Prepared> {
        let jack = self.event?;
        let handler = self.handler;
        let mut fresh = prepare(cx, dirs, &self.folder, Target::Event { jack, handler }).ok()?;
        // The same plugin as the one the call was made for (the folder may hold another one now).
        if fresh.id() != self.api.def.id {
            return None;
        }
        // A call of a completion jack keeps its word, its option and its list.
        if let Some(session) = self.complement {
            fresh.args = self.args;
            fresh.api.complement = self.api.complement;
            fresh.complement = Some(session);
        }
        Some(fresh)
    }

    /// The arguments the handler gets in `Plugin.args` (the `SmartIndent` jack passes the typed char).
    pub(crate) fn with_args(mut self, args: Value) -> Self {
        self.args = args;
        self
    }

    /// The host of the plugin has a call going (on another window): `run` would be refused.
    pub(crate) fn host_busy(&self) -> bool {
        self.host.borrow().current_jack().is_some()
    }
}

/// What a call is for: a command the user chose, or a plug of an event jack.
enum Target {
    Command {
        /// Counts from 1.
        command: usize,
        /// The handler it was registered with (a command added while a plugin ran); `None`: the one in
        /// `plugin.def`.
        handler: Option<String>,
    },
    Event {
        jack: Jack,
        handler: String,
    },
}

/// Why the first command of the plugin in `folder` cannot run, or what it needs to run. `command`
/// counts from 1.
pub(crate) fn prepare_command(
    cx: &mut App,
    dirs: &SearchDirs,
    folder: &str,
    command: usize,
) -> Result<Prepared, String> {
    prepare(
        cx,
        dirs,
        folder,
        Target::Command {
            command,
            handler: None,
        },
    )
}

/// As [`prepare_command`], for the plugin command with the function code `code` (a menu, a key or a
/// toolbar button): the plugin and the handler are the ones the command was registered with, also
/// when the plugin added it while it ran (`Plugin.AddCommand`).
pub(crate) fn prepare_function(
    cx: &mut App,
    dirs: &SearchDirs,
    code: u16,
) -> Result<Prepared, String> {
    let info = plugin_commands::info(code)
        .ok_or("このプラグインコマンドは、今は使えません（プラグインが有効でないか、読み込まれていません）")?;
    let settings = crate::doc_types::with_user(cx, crate::plugin_commands::settings_of);
    let folder = settings
        .table
        .get(info.slot)
        .filter(|rec| rec.is_used() && rec.id == info.plugin_id)
        .map(|rec| rec.name.as_str())
        .ok_or("このプラグインはもう登録されていません")?;
    prepare(
        cx,
        dirs,
        folder,
        Target::Command {
            command: info.plug,
            handler: Some(info.handler),
        },
    )
}

/// The call of the plug of the jack `Outline` with the handler `handler` of the plugin in `folder`
/// (`Command_FUNCLIST`). What the plug gives to the `Outline` object goes to `collected`, and the call
/// ends it ([`Collected::ending`]). Like an event, it says nothing when it goes well, and is told in the
/// output panel when it does not. `Err`: why it cannot run.
pub(crate) fn prepare_outline(
    cx: &mut App,
    dirs: &SearchDirs,
    folder: &str,
    handler: &str,
    collected: Rc<RefCell<Collected>>,
) -> Result<Prepared, String> {
    let mut prepared = prepare(
        cx,
        dirs,
        folder,
        Target::Event {
            jack: Jack::Outline,
            handler: handler.to_owned(),
        },
    )?;
    collected.borrow_mut().label = prepared.label.clone();
    prepared.api.outline = Some(collected);
    Ok(prepared)
}

/// The calls of the plugin in `folder` for the event jack `jack`, one for each of its plugs, in the order
/// of `plugin.def`. A plugin that cannot run (off, not agreed to, stopped, a language that is not served)
/// gives nothing, without a word: an event is not something the user asked for.
pub(crate) fn prepare_event(
    cx: &mut App,
    dirs: &SearchDirs,
    folder: &str,
    jack: Jack,
) -> Vec<Prepared> {
    let Some(def) = dirs.load_def(folder, DEFAULT_LANGUAGE) else {
        return Vec::new();
    };
    def.plugs
        .into_iter()
        .filter(|plug| plug.jack == jack)
        .filter_map(|plug| {
            prepare(
                cx,
                dirs,
                folder,
                Target::Event {
                    jack,
                    handler: plug.handler,
                },
            )
            .ok()
        })
        .collect()
}

/// The call of the plug of `jack` with the handler `handler` in the plugin in `folder` (a completion
/// jack picks its plug by the number).
pub(crate) fn prepare_plug(
    cx: &mut App,
    dirs: &SearchDirs,
    folder: &str,
    jack: Jack,
    handler: &str,
) -> Result<Prepared, String> {
    prepare(
        cx,
        dirs,
        folder,
        Target::Event {
            jack,
            handler: handler.to_owned(),
        },
    )
}

fn prepare(
    cx: &mut App,
    dirs: &SearchDirs,
    folder: &str,
    target: Target,
) -> Result<Prepared, String> {
    let settings = crate::doc_types::with_user(cx, crate::plugin_commands::settings_of);
    if !settings.enabled {
        return Err(
            "「プラグインを有効にする」が入っていません。入れて OK で保存してから試してください"
                .into(),
        );
    }
    let Some(slot) = settings.find_folder(folder) else {
        return Err(
            "このプラグインはまだ保存されていません。OK で保存してから試してください".into(),
        );
    };
    let def = dirs
        .load_def(folder, DEFAULT_LANGUAGE)
        .ok_or("plugin.def を読めません")?;
    if let Some(why) = def.unsupported() {
        return Err(crate::common_settings::plugin_unsupported_text(&why));
    }
    let plugin_dir = dirs.plugin_dir(folder);
    match ConsentFile::load(&dirs.consent_file())
        .consent_for_dir_cached(&def.id, plugin_dir.as_deref())
    {
        Consent::On => {}
        Consent::Off => {
            return Err(
                "有効にしていません。「有効にする」を押して OK で保存してから試してください".into(),
            );
        }
        Consent::Changed(_) => {
            return Err(
                "ファイルが変わったので止めています。もう一度「有効にする」を押してください".into(),
            );
        }
    }
    let mut event_jack = None;
    let (jack, handler, command, event) = match target {
        Target::Command { command, handler } => {
            let handler = match handler {
                Some(handler) => handler,
                None => command
                    .checked_sub(1)
                    .and_then(|index| def.commands.get(index))
                    .map(|item| item.handler.clone())
                    .ok_or("このプラグインにコマンドがありません")?,
            };
            ("Command", handler, command, false)
        }
        Target::Event { jack, handler } => {
            // The plug must be in the definition as it is now: a plugin that was changed and agreed to
            // again does not keep the automatic runs it no longer declares.
            if !def
                .plugs
                .iter()
                .any(|plug| plug.jack == jack && plug.handler == handler)
            {
                return Err("このプラグインは、このジャックを使わなくなりました".into());
            }
            // A key calls this jack: only a plugin that stays can have it (ADR-0026, chapter 2).
            if jack.needs_resident() && !def.resident {
                return Err(
                    "このジャックは、常駐するプラグイン（plugin.def の [Wsh] に UseCache=1、または [Matsu] に Resident=1）だけが使えます"
                        .into(),
                );
            }
            event_jack = Some(jack);
            (jack.name(), handler, 0, true)
        }
    };
    // An ES module and a TypeScript file would have Node.js read the file again after the user's
    // agreement was checked, so they are refused for now (the same words as the settings page).
    if matches!(
        Language::of_file(Path::new(&handler)),
        Some(Language::TypeScript)
    ) || Path::new(&handler)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mjs"))
    {
        return Err(format!(
            "{handler} は動かせません。.mjs と .ts のプラグインは、まだ動かせません。.cjs にしてください"
        ));
    }
    let language = match Language::of_file(Path::new(&handler)) {
        Some(language) if language.serves_plugins() && language.runs_here() => language,
        _ => {
            return Err(format!(
                "{handler} は動かせません。プラグインのハンドラは .js、.cjs、.py のどれかにしてください"
            ));
        }
    };
    let host_key = (def.id.clone(), language);
    let runtime = cx.default_global::<Runtime>();
    let want_resident = def.resident;
    let reuse = runtime
        .hosts
        .get(&host_key)
        .filter(|host| {
            let host = host.borrow();
            // A plugin that was replaced by another folder gets a host of its own folder.
            host.settings().resident == want_resident
                && Some(&host.settings().dir) == plugin_dir.as_ref()
        })
        .cloned();
    let host = match reuse {
        Some(host) => host,
        None => {
            if let Some(old) = runtime.hosts.remove(&host_key) {
                old.borrow_mut().shutdown();
            }
            let mut settings = HostSettings::new(
                &def.id,
                &plugin_dir.unwrap_or_default(),
                language,
                want_resident,
            );
            if language == Language::Bundled(Engine::QuickJs)
                && let Some(path) = test_interpreter()
            {
                settings.interpreter = path.into();
            }
            let host = Rc::new(RefCell::new(PluginHost::new(settings)));
            runtime.hosts.insert(host_key, host.clone());
            host
        }
    };
    if host.borrow().is_stopped() || is_stopped(&def.id) {
        return Err("失敗が続いたので止めています。もう一度「有効にする」を押すと戻ります".into());
    }
    let label = if def.name.is_empty() {
        folder.to_owned()
    } else {
        def.name.clone()
    };
    Ok(Prepared {
        label,
        host,
        jack,
        handler,
        args: if event {
            json!({})
        } else {
            json!({"command": command})
        },
        api: DefApi::new(dirs, folder, def, command).in_slot(slot),
        event: event_jack,
        folder: folder.to_owned(),
        complement: None,
    })
}

thread_local! {
    /// The program that serves the plugins in a test (the one that is built next to the tests).
    static TEST_INTERPRETER: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

fn test_interpreter() -> Option<PathBuf> {
    TEST_INTERPRETER.with(|path| path.borrow().clone())
}

thread_local! {
    /// A time limit that replaces the one of the jack in a test (a test cannot wait for 10 seconds).
    static TEST_LIMIT: std::cell::Cell<Option<Duration>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_test_limit(limit: Option<Duration>) {
    TEST_LIMIT.with(|cell| cell.set(limit));
}

#[cfg(test)]
pub(crate) fn set_test_interpreter(path: Option<PathBuf>) {
    TEST_INTERPRETER.with(|cell| *cell.borrow_mut() = path);
}

/// How many events of a plugin the window takes before it lets go for a moment.
const EVENTS_IN_A_ROW: usize = 32;

/// Whether an event call (`plugin_events`) may start on a window now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Turn {
    /// Nothing runs.
    Free,
    /// Another event call runs: wait for it.
    AfterEvent,
    /// A command of a plugin or a macro has the window: the event is not given.
    Taken,
}

impl EditorView {
    /// See [`Turn`].
    pub(crate) fn event_turn(&self, cx: &gpui::App) -> Turn {
        if !(self.macros.busy() || self.plugin_in_call(cx)) {
            Turn::Free
        } else if self.macros.plugin_running && self.macros.plugin_event {
            Turn::AfterEvent
        } else {
            Turn::Taken
        }
    }

    /// A flow that waits for event calls (a close, a save) begins to watch the text.
    pub(crate) fn begin_edit_watch(&mut self, cx: &gpui::App) {
        // Flows that overlap (a save that waits, and a close the user asked for meanwhile) share the watch:
        // it starts with the first and ends with the last.
        if self.macros.watching == 0 {
            self.macros.watch_seen = self.document.read(cx).version();
            self.macros.watch_user = false;
        }
        self.macros.watching += 1;
    }

    /// The flow is over; whether the user changed the text meanwhile. What a plugin did is not counted:
    /// every request of a plugin leaves the version it made, and a change that is not one of them was
    /// made by the user (typing, the IME, a command of a menu or a button).
    pub(crate) fn end_edit_watch(&mut self, cx: &gpui::App) -> bool {
        let changed =
            self.macros.watch_user || self.document.read(cx).version() != self.macros.watch_seen;
        self.macros.watching = self.macros.watching.saturating_sub(1);
        if self.macros.watching == 0 {
            self.macros.watch_user = false;
        }
        changed
    }

    /// Event calls of plugins are running or wait for their turn on this window.
    pub(crate) fn event_batches_pending(&self) -> bool {
        self.macros.event_serving != self.macros.event_next
    }

    /// A macro runs on this window (or waits for a dialog).
    pub(crate) fn macro_suspended(&self) -> bool {
        self.macros.is_suspended()
    }

    /// The window began with a file: `EditorStart` is due after it is read.
    pub(crate) fn mark_editor_start_due(&mut self) {
        self.macros.editor_start_due = true;
    }

    pub(crate) fn editor_start_due(&self) -> bool {
        self.macros.editor_start_due
    }

    pub(crate) fn take_editor_start_due(&mut self) -> bool {
        std::mem::take(&mut self.macros.editor_start_due)
    }

    /// A place in the line of the batches of event calls of this window.
    pub(crate) fn take_event_ticket(&mut self) -> u64 {
        let ticket = self.macros.event_next;
        self.macros.event_next += 1;
        ticket
    }

    /// The ticket whose batch runs now.
    pub(crate) fn event_serving(&self) -> u64 {
        self.macros.event_serving
    }

    /// The batch that ran is over: the next ticket is served.
    pub(crate) fn finish_event_ticket(&mut self) {
        self.macros.event_serving += 1;
    }

    /// A call of a plugin runs on this window.
    pub(crate) fn plugin_call_running(&self) -> bool {
        self.macros.plugin_running
    }

    /// Opens the undo block that holds a whole call of a plugin; [`EditorView::leave_undo_block`]
    /// with the depth from before closes it.
    fn open_undo_block(&mut self, cx: &mut Context<Self>) {
        let cursor = &self.cursor;
        self.document.update(cx, |shared, _| {
            shared.parts_mut().0.begin_block(cursor);
        });
    }

    /// Answers one request of a plugin on this view: the `Plugin.*` calls from `api`, every other
    /// call on the document, as a macro's call is (`bridge`). A call that needs a dialog is not
    /// answered here (`matsu_plugin::bridge::UNAVAILABLE`).
    pub(crate) fn answer_plugin_request(
        &mut self,
        text: &str,
        api: &mut dyn PluginApi,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        // An IME composition that the user started ends first, as for a key.
        self.interrupt_composition(cx);
        if self.macros.watching > 0 && self.document.read(cx).version() != self.macros.watch_seen {
            self.macros.watch_user = true;
        }
        let mut env = self.host_env(text, cx);
        // What a plugin does is not recorded in a key macro (`FA_NONRECORD`).
        let suspended = std::mem::replace(&mut self.macros.suspended, true);
        let reply = self.run_with(cx, |command_cx| {
            plugin_host::respond(text, api, |method, params| {
                bridge::call_with(Registry::global(), command_cx, &mut env, method, params)
            })
        });
        self.macros.suspended = suspended;
        self.finish_host_env(env, cx);
        if self.macros.watching > 0 {
            self.macros.watch_seen = self.document.read(cx).version();
        }
        self.indent_guard_follow(cx);
        reply
    }
}

/// The pane that began a call is gone: the document is free for the others again, and the undo
/// block of the call is closed (the cursor of the pane is gone with it).
fn release_document(
    document: &Entity<crate::document::SharedDocument>,
    depth: Option<usize>,
    generation: u64,
    cx: &mut gpui::AsyncApp,
) {
    document.update(cx, |shared, _| {
        // A document that was replaced is another one, with a busy state of its own (another call
        // may have begun on it), and its own history.
        if shared.generation() != generation {
            return;
        }
        shared.set_plugin_busy(false);
        // The undo block of the call was closed when the user did something, and what is open now may
        // be the user's own (an IME composition in another pane).
        let closed = shared.plugin_block_closed();
        shared.set_plugin_block_closed(false);
        if let (Some(depth), false) = (depth, closed) {
            shared
                .parts_mut()
                .0
                .restore_block_depth(depth, &matsu_core::cursor::Cursor::default());
        }
    });
}

/// Whether the request is a call of an object of the plugin itself (`Plugin.*`, `Complement.*`): they read
/// the definition and the options, or fill the list of a completion, and never touch the document.
pub(crate) fn is_plugin_object_call(text: &str) -> bool {
    matsu_plugin::protocol::parse_request(text).is_ok_and(|request| {
        plugin_host::PLUGIN_PREFIXES
            .iter()
            .any(|prefix| request.method.starts_with(prefix))
    })
}

/// Answers a call of an object of the plugin itself without the editor window.
pub(crate) fn answer_plugin_object_call(text: &str, api: &mut dyn PluginApi) -> Option<String> {
    plugin_host::respond(text, api, |method, _| {
        Err(RpcError::new(
            matsu_plugin::protocol::METHOD_NOT_FOUND,
            format!("no such call: {method}"),
        ))
    })
}

/// The answer to a request of a plugin whose document was replaced.
fn replaced_reply(text: &str) -> Option<String> {
    use matsu_plugin::protocol::{self, RpcError};
    match protocol::parse_request(text) {
        Ok(request) => request.id.map(|id| {
            protocol::response_error(
                &id,
                &RpcError::new(
                    protocol::INTERNAL_ERROR,
                    "the document of this call was closed or replaced",
                ),
            )
        }),
        Err(rejection) => Some(protocol::response_error(&rejection.id, &rejection.error)),
    }
}

/// The words for how a call ended, for the output panel.
pub(crate) fn describe(outcome: &Outcome, limit: Duration) -> String {
    match outcome {
        Outcome::Ok => "正常に終了しました".to_owned(),
        Outcome::Failed(_) => "エラーで終了しました".to_owned(),
        Outcome::TimedOut => format!("時間切れで中止しました ({} 秒)", limit.as_secs_f64()),
        Outcome::Cancelled => "中断しました".to_owned(),
        Outcome::Died(text) => format!("プラグインのプロセスが終わりました ({text})"),
    }
}

/// Runs the call on the view and follows it to its end. The output of the plugin and the end go
/// to the output panel; everything the plugin changes in the document is one undo step. Returns
/// whether the call began.
///
/// An event call (a plug of a jack such as `DocumentBeforeSave`) says nothing when it goes well: it
/// happens at each save and each open, and the panel and the status bar are for what the user did. A
/// failure is told as for a command.
pub(crate) fn run(editor: &Entity<EditorView>, prepared: Prepared, cx: &mut App) -> bool {
    let Prepared {
        label,
        host,
        jack,
        handler,
        args,
        mut api,
        event,
        complement: session,
        ..
    } = prepared;
    let event = event.is_some();
    let outline = api.outline.clone();
    let complement = session.zip(api.complement.clone());
    let limit = TEST_LIMIT
        .with(std::cell::Cell::get)
        .unwrap_or_else(|| limit_of(jack));
    // The call of the jack `Outline` ends in its result (the pane waits for it).
    let end = {
        move |ending: Ending| {
            if let Some(collected) = &outline {
                collected.borrow_mut().ending = Some(ending);
            }
        }
    };
    let commands_before = plugin_commands::commands().len();
    // One plugin at a time on a window: the calls would share the undo block of the document, and
    // a macro that runs has the window already.
    let busy = editor.update(cx, |view, cx| {
        // The whole state of the macros: a script, a key macro that plays or waits for a dialog (the
        // settings that started this call may be the dialog it waits for), or another call.
        view.plugin_in_call(cx) || view.macros.busy()
    });
    if busy {
        // An event is dropped when another call has the window (a plugin that makes its own events
        // is not given them: ADR-0026, chapter 2).
        if !event {
            let message =
                format!("プラグイン {label}: ほかのプラグインかマクロが動いている間は動かせません");
            editor.update(cx, |view, cx| {
                view.macros.output.begin();
                view.macros.output.push(Kind::Note, message.clone());
                view.notice = Some(message);
                cx.notify();
            });
        }
        return false;
    }
    if let Err(error) = host.borrow_mut().begin(jack, &handler, &args, limit) {
        end(Ending::Refused(error.to_string()));
        // A plugin that three failures stopped has said so already.
        if !(event && matches!(error, plugin_host::BeginError::Stopped)) {
            let message = format!("プラグイン {label}: {error}");
            editor.update(cx, |view, cx| {
                if event {
                    view.macros.output.show();
                } else {
                    view.macros.output.begin();
                }
                view.macros.output.push(Kind::Note, message.clone());
                view.notice = Some(message);
                cx.notify();
            });
        }
        return false;
    }
    let depth = editor.update(cx, |view, cx| {
        if !event {
            view.macros.output.begin();
            view.macros.output.push(
                Kind::Note,
                format!("プラグイン {label} の {jack} ({handler}) を開始します"),
            );
            cx.notify();
        }
        view.macros.plugin_running = true;
        view.macros.plugin_event = event;
        view.document
            .update(cx, |shared, _| shared.set_plugin_busy(true));
        let depth = view.enter_undo_block(cx);
        // Everything the plugin does is one step of undo: the block is open until the call ends.
        // A completion call opens none: it finds candidates, and the user may type while it runs. The
        // depth is kept all the same, so that a block the handler itself opens (`AddRefUndoBuffer`) and
        // does not close is closed when the call ends.
        if depth.is_some() && complement.is_none() {
            view.open_undo_block(cx);
        }
        // A smart indent is cut off from the user's input when the user does something (`indent`).
        if view.macros.indent_guard.is_some() {
            view.macros.indent_undo_depth = depth;
            view.macros.indent_closed = false;
            view.document
                .update(cx, |shared, _| shared.set_plugin_block_closed(false));
        }
        (depth, view.document.read(cx).generation())
    });
    let (depth, generation) = depth;
    let weak = editor.downgrade();
    // The document outlives the pane that began the call when another pane shows it: what the call
    // holds on the document is let go also when that pane is gone.
    let document = editor.read(cx).document.clone();
    cx.spawn(async move |cx| {
        let mut busy = 0_usize;
        loop {
            let progress = host.borrow_mut().poll(Duration::ZERO);
            let Some(progress) = progress else {
                // The editor of the call is gone (its document was closed): the call ends with it,
                // also when the plugin is computing and says nothing.
                if weak.upgrade().is_none() {
                    host.borrow_mut().cancel();
                    release_document(&document, depth, generation, cx);
                    end(Ending::Done(Outcome::Cancelled));
                    break;
                }
                // The window loaded another document: a plugin that computes without asking
                // anything is stopped too (the next poll tells the end).
                let replaced = weak
                    .update(cx, |view, cx| view.document.read(cx).generation() != generation)
                    .unwrap_or(false);
                if replaced {
                    host.borrow_mut().shutdown();
                    continue;
                }
                // A smart indent whose place is gone (the user typed on or moved) is stopped now, so that
                // the undo block of the call ends and the input after it is not in it.
                let moved = weak
                    .update(cx, |view, cx| view.stop_indent_if_moved(cx))
                    .unwrap_or(false);
                if moved {
                    host.borrow_mut().shutdown();
                    continue;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
                continue;
            };
            // A plugin that never stops printing or asking must not keep the window from painting
            // and taking clicks: after a number of events in a row, the window goes first.
            busy += 1;
            if busy >= EVENTS_IN_A_ROW {
                busy = 0;
                super::yield_now().await;
            }
            let alive = match progress {
                Progress::Output { stream, text } => weak
                    .update(cx, |view, cx| {
                        let kind = match stream {
                            Stream::Stdout => Kind::Out,
                            Stream::Stderr => Kind::Err,
                        };
                        view.macros.output.push(kind, text);
                        // An event says nothing unless the handler printed: then the panel is shown.
                        if event {
                            view.macros.output.show();
                        }
                        cx.notify();
                    })
                    .is_ok(),
                Progress::Request(text) => {
                    let reply = weak.update(cx, |view, cx| {
                        // The call belongs to the document it began on. If the window loaded
                        // another one meanwhile, the plugin is refused and stopped: it must not
                        // edit what it was not given (ADR-0026, chapter 2).
                        if view.document.read(cx).generation() != generation {
                            return (replaced_reply(&text), true);
                        }
                        // A smart indent is for the place where the char was typed: the user went on
                        // typing or moved the caret, so the plugin is stopped. What the user typed
                        // is in the text; nothing of the plugin's is added to it.
                        if view.stop_indent_if_moved(cx) {
                            return (replaced_reply(&text), true);
                        }
                        // A call of an object of the plugin itself (`Plugin.GetOption`, `Complement.AddList`)
                        // does not touch the document: it must not touch the window either (an IME
                        // composition of the user goes on).
                        if is_plugin_object_call(&text) {
                            return (answer_plugin_object_call(&text, &mut api), false);
                        }
                        (view.answer_plugin_request(&text, &mut api, cx), false)
                    });
                    match reply {
                        Ok((reply, replaced)) => {
                            if let Some(line) = reply {
                                host.borrow().reply(line);
                            }
                            if replaced {
                                host.borrow_mut().shutdown();
                            }
                            // The candidates that came with this request are shown at once.
                            if let Some((session, list)) = &complement {
                                weak.update(cx, |view, cx| {
                                    view.complement_arrived(*session, list, cx);
                                })
                                .ok();
                            }
                            true
                        }
                        Err(_) => false,
                    }
                }
                Progress::Done(outcome) => {
                    let stopped = note_outcome(&api.def.id, &outcome);
                    if outcome == Outcome::Ok {
                        let id = api.def.id.clone();
                        cx.update(|app| forgive_hosts(app, &id));
                    }
                    let ended = weak.update(cx, |view, cx| {
                        view.macros.plugin_running = false;
                        view.macros.plugin_event = false;
                        view.macros.indent_guard = None;
                        // Only the document the call began on: a replaced one is not ours to free.
                        let mut closed = std::mem::take(&mut view.macros.indent_closed);
                        view.macros.indent_undo_depth = None;
                        if view.document.read(cx).generation() == generation {
                            // Whoever cut the call off says so on the document (`release_document` reads it too).
                            closed |= view.document.read(cx).plugin_block_closed();
                            view.document.update(cx, |shared, _| {
                                shared.set_plugin_busy(false);
                                shared.set_plugin_block_closed(false);
                            });
                            // The undo block of a smart indent that the user cut off is closed already,
                            // and what is open now is the user's own. A composition that the user began
                            // meanwhile has a block of its own, which closing to the depth from before
                            // the call would end.
                            if !closed && (complement.is_none() || !view.composing) {
                                view.leave_undo_block(depth, cx);
                            }
                        }
                        // A smart indent that was stopped because the user went on says nothing.
                        if std::mem::take(&mut view.macros.indent_stopped)
                            && outcome == Outcome::Cancelled
                        {
                            cx.notify();
                            return;
                        }
                        // An event that went well leaves no trace.
                        if event && outcome == Outcome::Ok && !stopped {
                            cx.notify();
                            return;
                        }
                        if event {
                            // What was printed before stays; the panel is shown for the failure.
                            view.macros.output.show();
                            view.macros.output.push(
                                Kind::Note,
                                format!("プラグイン {label} の {jack} ({handler})"),
                            );
                        }
                        if let Outcome::Failed(text) | Outcome::Died(text) = &outcome {
                            view.macros.output.push(Kind::Err, text.clone());
                        }
                        let message = describe(&outcome, limit);
                        view.notice = Some(if event {
                            format!("プラグイン {label}: {jack}: {message}")
                        } else {
                            format!("プラグイン {label}: {message}")
                        });
                        view.macros.output.push(Kind::Note, message);
                        if stopped {
                            view.macros.output.push(
                                Kind::Note,
                                "失敗が続いたので、このプラグインを止めました。もう一度「有効にする」を押すと戻ります",
                            );
                        }
                        cx.notify();
                    });
                    if ended.is_err() {
                        // The pane is gone: the document is let go all the same.
                        release_document(&document, depth, generation, cx);
                    }
                    // A command the plugin added (`Plugin.AddCommand`) is in the menus, the keys and the
                    // toolbar from now on.
                    if plugin_commands::commands().len() != commands_before {
                        cx.update(crate::plugin_commands::commands_changed);
                    }
                    end(Ending::Done(outcome));
                    break;
                }
            };
            if !alive {
                // The window is gone: the call ends with it (and so does the claim on it).
                host.borrow_mut().cancel();
                release_document(&document, depth, generation, cx);
                end(Ending::Done(Outcome::Cancelled));
                break;
            }
        }
    })
    .detach();
    true
}

#[cfg(test)]
pub(crate) mod tests;
