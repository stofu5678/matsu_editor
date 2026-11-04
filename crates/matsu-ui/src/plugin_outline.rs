//! The `Outline` jack of the plugins (ADR-0026, chapter 3 and the seventh PR): a plugin makes an outline
//! of the document, and the outline pane shows it.
//!
//! What is Sakura's (`CViewCommander::Command_FUNCLIST` in `sakura_core/cmd/CViewCommander_Outline.cpp`,
//! `sakura_core/plugin/COutlineIfObj.h`, `CJackManager::RegisterPlug`, `CPropTypesScreen::AddOutlineMethod`):
//!
//! * the plug is called once when the outline is analyzed (the pane opens, reloads, follows another
//!   document or another type), never for a key press, and only the one the type settings chose;
//! * the choice is a number stored in `nDefaultOutline`: the function code of the plug
//!   (`CPlug::GetOutlineType`, [`matsu_commands::plugin_commands::plug_code`]). The type settings list
//!   the outlines of the plugins that are loaded after Sakura's own, by their label;
//! * the number of a plugin that is not loaded (off, removed, not agreed to) is not a choice of the
//!   list, and the analysis falls to the text topics (`MakeTopicList_txt`);
//! * the plug gets the `Outline` object: `AddFuncInfo` to `AddFuncInfo4`, `SetTitle`, `SetListType`,
//!   `SetLabel`, with Sakura's arguments. It reads the document with `Editor.*`.
//!
//! What is not Sakura's (ADR-0026, chapter 2): the call has a time limit of 10 seconds
//! (`matsu_plugin::plugin_host::LIMITS`). A plug that fails, throws or is too slow does not fall to the
//! built-in analysis: the pane shows an empty list and says why. A file read by ranges has no outline
//! (the same as the built-in analyzers: [`crate::outline::pane::HUGE_FILE_MESSAGE`]).
//!
//! The pane starts the call ([`crate::outline::pane`]); this module knows which plugs there are, builds
//! the choices of the type settings, and collects what the `Outline` object is given.

use std::cell::RefCell;

use gpui::App;
use matsu_commands::plugin_commands::{self, plug_code};
use matsu_config::plugins::consent::{Consent, ConsentFile};
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginSettings, PluginState};
use matsu_outline::{FuncInfo, FuncInfoArr, list_type};
use matsu_plugin::Value;
use matsu_plugin::plugin_host::Outcome;
use matsu_plugin::protocol::{INTERNAL_ERROR, RpcError};

use crate::settings::UserSettings;

/// `FUNCINFO_INFOMASK`: the low bits of the last argument of `AddFuncInfo2` and `AddFuncInfo4` are the
/// depth, the rest is the information.
const INFO_MASK: i32 = 0xFFFF;

/// An outline a plugin offers: one `Outline` plug of a plugin that may run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutlinePlug {
    /// The value of `nDefaultOutline` that chooses it (the function code of the plug).
    pub code: i32,
    /// The slot of the plugin in the plugin table.
    pub slot: usize,
    /// The folder of the plugin under `plugins\`.
    pub folder: String,
    /// The label of the plug (`Outline.Label`, else the handler): the text in the list of the type
    /// settings.
    pub label: String,
    /// The handler file of the plug.
    pub handler: String,
}

/// The plugs of the jack `Outline` of the plugins that may run now, in the order Sakura registers
/// them: the plugin table, then the plugs in the order of the plugin. A plugin that may run is
/// registered, switched on (`EnablePlugin`), readable, of a kind MATSU runs, and agreed to by the
/// user with its files as they are.
fn plugs_of(user: &UserSettings) -> Vec<OutlinePlug> {
    let profile = user.profile().cloned().unwrap_or_default();
    let settings = PluginSettings::from_profile(&profile);
    let Some(dirs) = crate::plugin_commands::dirs_of(user).filter(|_| settings.enabled) else {
        return Vec::new();
    };
    let consent = ConsentFile::load(&dirs.consent_file());
    let mut plugs = Vec::new();
    for (slot, rec) in settings.table.iter().enumerate() {
        if !rec.is_used() || rec.state == PluginState::Deleted || rec.id.is_empty() {
            continue;
        }
        let Some(def) = dirs.load_def(&rec.name, DEFAULT_LANGUAGE) else {
            continue;
        };
        if def.id != rec.id || def.unsupported().is_some() {
            continue;
        }
        let dir = dirs.plugin_dir(&rec.name);
        if !matches!(
            consent.consent_for_dir(&def.id, dir.as_deref()),
            Consent::On
        ) {
            continue;
        }
        for plug in def.plugs.iter().filter(|plug| plug.jack == Jack::Outline) {
            let Some(code) = plug_code(slot, plug.index) else {
                continue;
            };
            plugs.push(OutlinePlug {
                code: i32::from(code),
                slot,
                folder: rec.name.clone(),
                label: plug.label.clone(),
                handler: plug.handler.clone(),
            });
        }
    }
    plugs
}

/// The outline plug with the number `code` that may run now (`GetUsablePlug( PP_OUTLINE, code )`).
pub(crate) fn find(cx: &App, code: i32) -> Option<OutlinePlug> {
    // A number that cannot be the code of a plug of a plugin is none (the check is cheap; the
    // settings files are read only for a number that could be one).
    let code = u16::try_from(code).ok()?;
    if !plugin_commands::is_plugin_code(code) {
        return None;
    }
    crate::doc_types::with_user(cx, plugs_of)
        .into_iter()
        .find(|plug| plug.code == i32::from(code))
}

thread_local! {
    /// The choices the type settings list after Sakura's own: the label and the number of each
    /// outline plug. The UI thread keeps it up to date ([`refresh_for`]).
    static CHOICES: RefCell<Vec<(String, i32)>> = const { RefCell::new(Vec::new()) };

    /// The windows (their editors) in which an `Outline` call is going: Sakura does not analyze again
    /// while a plug runs (`bIsProcessing`, "アウトラインプラグイン内でのEditor.Outline呼び出しによる再入を禁止する").
    /// Sakura has a process for each window; here only the window of the call is held back.
    static RUNNING: RefCell<Vec<gpui::EntityId>> = const { RefCell::new(Vec::new()) };
}

/// Makes the choices of the type settings from the settings now. Call it when the plugins or what the
/// user agreed to may have changed ([`crate::plugin_commands::refresh_for`] does, and the type
/// settings do when they open).
pub(crate) fn refresh_for(user: &UserSettings) {
    let choices = plugs_of(user)
        .into_iter()
        .map(|plug| (plug.label, plug.code))
        .collect();
    CHOICES.with(|cell| *cell.borrow_mut() = choices);
}

/// [`refresh_for`] for the settings of the application.
pub(crate) fn refresh(cx: &App) {
    crate::doc_types::with_user(cx, refresh_for);
}

/// The outlines of the plugins as the type settings list them: the label and the number.
pub(crate) fn choices() -> Vec<(String, i32)> {
    CHOICES.with(|cell| cell.borrow().clone())
}

/// Whether an `Outline` call is going in the window of the editor `view`.
pub(crate) fn is_running_in(view: gpui::EntityId) -> bool {
    RUNNING.with(|running| running.borrow().contains(&view))
}

/// How the call ended, when it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// The handler ran to its end, failed, or was stopped.
    Done(Outcome),
    /// The call did not begin (the plugin is stopped, or was turned off); the reason.
    Refused(String),
}

/// What an `Outline` call is given (`COutlineIfObj`), and how it ended.
pub(crate) struct Collected {
    items: FuncInfoArr,
    /// `Outline.SetTitle`.
    title: Option<String>,
    /// `Outline.SetListType`: `OUTLINE_PLUGIN` until the plug chooses.
    list_type: i32,
    /// The name of the plugin, for what is said of a failure.
    pub(crate) label: String,
    /// `None` while the call is going.
    pub(crate) ending: Option<Ending>,
    /// The editor the call is going in ([`Collected::in_view`]).
    view: Option<gpui::EntityId>,
}

impl Collected {
    /// An empty result for a call that is about to begin.
    pub(crate) fn new() -> Self {
        Self {
            items: FuncInfoArr::new(),
            title: None,
            list_type: list_type::PLUGIN,
            label: String::new(),
            ending: None,
            view: None,
        }
    }

    /// The call goes in the editor `view`: it counts as going there ([`is_running_in`]) until the result
    /// is dropped.
    pub(crate) fn in_view(mut self, view: gpui::EntityId) -> Self {
        RUNNING.with(|running| running.borrow_mut().push(view));
        self.view = Some(view);
        self
    }

    /// What the plug gave, as an analysis, when it went well; else an empty list with the reason.
    pub(crate) fn to_analysis(&self, limit: std::time::Duration) -> matsu_outline::Analysis {
        let failure = match &self.ending {
            Some(Ending::Done(Outcome::Ok)) => None,
            Some(Ending::Done(outcome)) => Some(describe_failure(&self.label, outcome, limit)),
            Some(Ending::Refused(reason)) => Some(format!("プラグイン {}: {reason}", self.label)),
            None => Some(format!("プラグイン {}: 終わる前に中止しました", self.label)),
        };
        let kind = matsu_outline::OutlineKind::from_list_type(self.list_type);
        match failure {
            None => matsu_outline::Analysis {
                items: self.items.clone(),
                kind,
                title: self.title.clone(),
                error: None,
            },
            // No built-in analysis in its place (ADR-0026, chapter 2): an empty list, and the reason.
            Some(message) => matsu_outline::Analysis {
                items: FuncInfoArr::new(),
                kind: matsu_outline::OutlineKind::List,
                title: Some(self.label.clone()),
                error: Some(message),
            },
        }
    }

    /// The calls of the `Outline` object (`COutlineIfObj::HandleCommand`).
    pub(crate) fn call(&mut self, name: &str, params: &[Value]) -> Result<Value, RpcError> {
        match name {
            "Outline.AddFuncInfo" => self.add(name, params, false, false),
            "Outline.AddFuncInfo2" => self.add(name, params, true, false),
            "Outline.AddFuncInfo3" => self.add(name, params, false, true),
            "Outline.AddFuncInfo4" => self.add(name, params, true, true),
            "Outline.SetTitle" => {
                self.title = Some(text_at(params, 0, name)?);
                Ok(Value::Null)
            }
            "Outline.SetListType" => {
                let number = number_at(params, 0, name)?;
                self.list_type = i32::try_from(number).unwrap_or(list_type::LIST);
                Ok(Value::Null)
            }
            "Outline.SetLabel" => {
                let info = number_at(params, 0, name)?;
                let text = text_at(params, 1, name)?;
                let info = u32::try_from(info).map_err(|_| {
                    RpcError::invalid_params(format!("{name}: argument 1 must not be negative"))
                })?;
                self.items.set_append_text(info, text, true);
                Ok(Value::Null)
            }
            other => Err(RpcError::new(
                matsu_plugin::protocol::METHOD_NOT_FOUND,
                format!("no such call: {other}"),
            )),
        }
    }

    /// `AddFuncInfo`, `AddFuncInfo2`, `AddFuncInfo3` and `AddFuncInfo4`: the line, the column, the text
    /// and the information (the depth with `depth`, as the low bits of the last number), and the file for
    /// `file`. The positions are Sakura's logic positions, from 1.
    fn add(
        &mut self,
        name: &str,
        params: &[Value],
        depth: bool,
        file: bool,
    ) -> Result<Value, RpcError> {
        let line = number_at(params, 0, name)?;
        let col = number_at(params, 1, name)?;
        let text = text_at(params, 2, name)?;
        let param = number_at(params, 3, name)? as i32;
        let file = if file {
            text_at(params, 4, name)?
        } else {
            String::new()
        };
        let (info, depth) = if depth {
            let depth = param & INFO_MASK;
            (param.wrapping_sub(depth), depth)
        } else {
            (param, 0)
        };
        // (A position before the first line or column is kept as it is in Sakura; here it is the first
        // place of the item list that has none, 0.)
        self.items.push_item(FuncInfo {
            line: usize::try_from(line).unwrap_or(0),
            col: usize::try_from(col).unwrap_or(0),
            name: text,
            depth: usize::try_from(depth).unwrap_or(0),
            info: info as u32,
            file,
        });
        Ok(Value::Null)
    }
}

impl Drop for Collected {
    fn drop(&mut self) {
        if let Some(view) = self.view {
            RUNNING.with(|running| {
                let mut running = running.borrow_mut();
                if let Some(at) = running.iter().position(|id| *id == view) {
                    running.remove(at);
                }
            });
        }
    }
}

/// The words for a call that did not go well, for the message of the pane.
fn describe_failure(label: &str, outcome: &Outcome, limit: std::time::Duration) -> String {
    let what = crate::macros::plugin_call::describe(outcome, limit);
    match outcome {
        Outcome::Failed(text) | Outcome::Died(text) => {
            let first = text.lines().next().unwrap_or_default();
            format!("プラグイン {label}: {what}: {first}")
        }
        _ => format!("プラグイン {label}: {what}"),
    }
}

fn text_at(params: &[Value], index: usize, name: &str) -> Result<String, RpcError> {
    match params.get(index) {
        Some(Value::String(text)) => Ok(text.clone()),
        // WSH hands every argument as a string (`BSTR`); a number is its text.
        Some(Value::Number(number)) => Ok(number.to_string()),
        _ => Err(RpcError::invalid_params(format!(
            "{name}: argument {} must be a string",
            index + 1
        ))),
    }
}

/// An integer argument: Sakura reads it with `_wtoi` / `_wtol` from the text of the argument.
fn number_at(params: &[Value], index: usize, name: &str) -> Result<i64, RpcError> {
    let bad =
        || RpcError::invalid_params(format!("{name}: argument {} must be a number", index + 1));
    match params.get(index) {
        Some(Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|f| f as i64))
            .ok_or_else(bad),
        Some(Value::String(text)) => Ok(atoi(text)),
        _ => Err(bad()),
    }
}

/// `_wtoi`: the leading blanks, a sign and the digits; the rest is ignored, and no digits is 0.
fn atoi(text: &str) -> i64 {
    let text = text.trim_start();
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    let value = digits[..end].parse::<i64>().unwrap_or(0);
    if negative { -value } else { value }
}

/// The error of the editor's own side, for a call the plugin may not make.
pub(crate) fn not_in_an_outline(name: &str) -> RpcError {
    RpcError::new(
        INTERNAL_ERROR,
        format!("{name}: only a plug of the jack Outline can use the Outline object"),
    )
}

#[cfg(test)]
pub(crate) mod tests;
