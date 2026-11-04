//! The completion jacks of the plugins (ADR-0026, chapter 3 and the eighth PR): `ComplementGlobal` and
//! `Complement`.
//!
//! Sakura (`CHokanMgr::Search`, `sakura_core/env/CHokanMgr.cpp:169-190`) asks the plugins after its own
//! sources: first every plug of `ComplementGlobal`, in the order of the plugin table, then the plug of
//! `Complement` that the document type chose (`nHokanType`, in the type settings 支援). Each gets a
//! `Complement` object (`CComplementIfObj`, `sakura_core/plugin/CComplementIfObj.h`): `GetCurrentWord`
//! is the word before the caret, `GetOption` the options (`0x01` ignore the case of letters, `0x02` the
//! words of the document are a source), and `AddList` adds a candidate to the list; it answers the
//! number of candidates, or -1 when the list has the word already (`CHokanMgr::AddKouhoUnique`).
//!
//! What MATSU does differently, as ADR-0026 decided:
//!
//! * The plugins are asked once, when the completion command (`F_HOKAN`) opens the list. Sakura asks
//!   again for every character typed while the list is open; MATSU narrows the candidates it has (the
//!   plugins' ones by the beginning of the new word) and does not run any plugin code for a key. The
//!   automatic completion does not exist in MATSU, so nothing calls the plugins by itself.
//! * Only a plugin that stays (`UseCache=1`) can have these jacks: a key must not wait for a process
//!   to start (`Jack::needs_resident`).
//! * The call has a time limit of one second (`matsu_plugin::plugin_host::LIMITS`). What was added
//!   before the limit stays; the process is killed at the limit, so nothing comes later.
//! * The list opens with the candidates of the other sources, and the plugins' ones are added as they
//!   arrive. If the other sources have fewer than two, nothing is shown until the plugins are done, so
//!   that Sakura's rules hold (no candidate: nothing; one: it is inserted at once).
//! * If the text or the caret moves while the plugins run, what they add is dropped (ADR-0026: a
//!   completion is not for a document that changed).
//! * A call of one window at a time (`plugin_call::run`): if another plugin or a macro runs on the
//!   document, or the host of a plugin is busy, the call is skipped and the list is made without it.
//!
//! The choice of the plugin of `Complement` is in the type settings ([`choices`], [`chosen_slot`]).

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, Context};
use matsu_commands::plugin_commands;
use matsu_config::plugins::consent::{Consent, ConsentFile};
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginSettings, PluginState, SearchDirs};
use matsu_core::buffer::Pos;

use crate::macros::plugin_call::{self, Prepared};
use crate::support::completion::Completion;
use crate::view::EditorView;

/// How often the task that follows the calls looks again.
const TICK: Duration = Duration::from_millis(10);

/// The most candidates one list takes from plugins (a plugin that adds without end must not fill the
/// memory). Past it `AddList` answers -1.
const MAX_ADDED: usize = 10_000;

/// The option bit: ignore the case of letters (`bHokanLoHiCase`).
const OPTION_IGNORE_CASE: i64 = 0x01;
/// The option bit: the words of the document are a source (`bHokanByFile`).
const OPTION_BY_FILE: i64 = 0x02;

/// The list that `Complement.AddList` fills. It starts with the candidates of the other sources, so
/// that a word the list has is not added twice and the number it answers is Sakura's (the size of the
/// whole list).
pub(crate) struct Collector {
    words: Vec<String>,
    seen: HashSet<String>,
    /// How many of `words` came from the other sources.
    base: usize,
    /// How many of `words` the window has taken.
    taken: usize,
}

impl Collector {
    pub(crate) fn new(already: Vec<String>) -> Self {
        Self {
            seen: already.iter().cloned().collect(),
            base: already.len(),
            taken: already.len(),
            words: already,
        }
    }

    /// `AddList`: the number of candidates now, or -1 if the list has the word (or is full).
    pub(crate) fn add(&mut self, word: &str) -> i64 {
        if self.words.len() - self.base >= MAX_ADDED || !self.seen.insert(word.to_owned()) {
            return -1;
        }
        self.words.push(word.to_owned());
        i64::try_from(self.words.len()).unwrap_or(i64::MAX)
    }

    /// The candidates added since the last time.
    fn take_new(&mut self) -> Vec<String> {
        let new = self.words[self.taken..].to_vec();
        self.taken = self.words.len();
        new
    }
}

/// The list shared by the call and the window.
pub(crate) type Shared = Rc<RefCell<Collector>>;

/// What a completion that asks plugins remembers about the moment it began.
struct Session {
    id: u64,
    version: u64,
    caret: Pos,
}

/// The plugins' side of the completion of a window.
#[derive(Default)]
pub(crate) struct ComplementState {
    last_id: u64,
    session: Option<Session>,
    /// The candidates the plugins gave to the list that is open, in the order they came.
    words: Vec<String>,
}

impl ComplementState {
    /// The plugins' candidates of the open list.
    pub(crate) fn words(&self) -> &[String] {
        &self.words
    }

    /// Whether the plugins are still being asked.
    pub(crate) fn waiting(&self) -> bool {
        self.session.is_some()
    }

    /// The list is closed or began again: what is still to come is dropped, and so are the words.
    pub(crate) fn clear(&mut self) {
        self.session = None;
        self.words.clear();
    }

    /// Stops asking (the text changed, or the list went away); the words that came stay.
    pub(crate) fn stop(&mut self) {
        self.session = None;
    }
}

/// The folders of the plugins that may run, in the order of the plugin table.
fn running_folders(cx: &App) -> Vec<String> {
    let profile =
        crate::doc_types::with_user(cx, |user| user.profile().cloned()).unwrap_or_default();
    let settings = PluginSettings::from_profile(&profile);
    if !settings.enabled {
        return Vec::new();
    }
    settings
        .table
        .iter()
        .filter(|rec| rec.is_used() && rec.state != PluginState::Deleted && !rec.name.is_empty())
        .map(|rec| rec.name.clone())
        .collect()
}

/// The calls to make for the completion of `key`: every plug of `ComplementGlobal`, then the plug of
/// `Complement` that the document type `type_index` chose.
fn prepare_calls(
    cx: &mut App,
    type_index: usize,
    session: u64,
    key: &str,
    option: i64,
    list: &Shared,
) -> Vec<Prepared> {
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return Vec::new();
    };
    let mut calls = Vec::new();
    for folder in running_folders(cx) {
        calls.extend(plugin_call::prepare_event(
            cx,
            &dirs,
            &folder,
            Jack::ComplementGlobal,
        ));
    }
    let chosen = crate::doc_types::with_user(cx, |user| {
        user.shared_types().get(type_index).map(|t| t.hokan_type)
    });
    if let Some(folder) = chosen.and_then(|code| chosen_folder(cx, code)) {
        // Only the plug without a number: the type settings name the plugin by the code of its plug 0
        // (`CJackManager::RegisterPlug`), and Sakura finds no other plug by it.
        if let Some(handler) = complement_plug(&dirs, &folder).map(|(handler, _)| handler)
            && let Ok(call) =
                plugin_call::prepare_plug(cx, &dirs, &folder, Jack::Complement, &handler)
        {
            calls.push(call);
        }
    }
    calls
        .into_iter()
        .map(|call| call.for_complement(session, key, option, list.clone()))
        .collect()
}

/// The handler and the label of the plug of `Complement` without a number in the plugin in `folder`.
fn complement_plug(dirs: &SearchDirs, folder: &str) -> Option<(String, String)> {
    let def = dirs.load_def(folder, DEFAULT_LANGUAGE)?;
    def.plugs
        .iter()
        .find(|plug| plug.jack == Jack::Complement && plug.index == 0)
        .map(|plug| (plug.handler.clone(), plug.label.clone()))
}

/// The slot of the plugin that a stored `nHokanType` names, if it names one.
pub(crate) fn chosen_slot(code: i32) -> Option<usize> {
    let code = u16::try_from(code).ok()?;
    match plugin_commands::decode(code)? {
        (slot, 0) => Some(slot),
        _ => None,
    }
}

/// The folder of the plugin that the stored `nHokanType` `code` names, if it is registered.
fn chosen_folder(cx: &App, code: i32) -> Option<String> {
    let slot = chosen_slot(code)?;
    let profile =
        crate::doc_types::with_user(cx, |user| user.profile().cloned()).unwrap_or_default();
    let settings = PluginSettings::from_profile(&profile);
    settings
        .table
        .get(slot)
        .filter(|rec| rec.is_used() && rec.state != PluginState::Deleted && !rec.name.is_empty())
        .map(|rec| rec.name.clone())
}

/// What the choice 「入力補完機能の種類」 of the type settings offers, after なし: the label and the stored
/// number (`nHokanType`) of each plugin that the user enabled and that has a `Complement` plug without
/// a number and stays. Sakura lists the plugins as they are registered, by the label of the plug
/// (`CJackManager::RegisterPlug`, `CPropTypesSupport::AddHokanMethod`).
pub(crate) fn choices(cx: &App) -> Vec<(String, i32)> {
    let Some(key) = choices_key(cx) else {
        return Vec::new();
    };
    CHOICES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((cached, list)) = cache.as_ref()
            && *cached == key
        {
            return list.clone();
        }
        let list = compute_choices(cx);
        *cache = Some((key, list.clone()));
        list
    })
}

type ChoicesCache = Option<(Vec<String>, Vec<(String, i32)>)>;

thread_local! {
    /// The choices last made and what they were made from. The page asks at every draw, and making them
    /// hashes the files of the plugins (the agreement of the user), which the UI thread must not do
    /// every frame.
    static CHOICES: RefCell<ChoicesCache> = const { RefCell::new(None) };
}

/// Forgets the choices made last: the next ask makes them again (the type settings call it when they open,
/// so that plugins changed on the disk meanwhile are seen).
pub(crate) fn forget_choices() {
    CHOICES.with(|cache| *cache.borrow_mut() = None);
}

/// What the choices depend on that is cheap to read: the switch, the plugin table, and the size and time of
/// the agreement file. The agreement itself is checked again when a plugin is called.
fn choices_key(cx: &App) -> Option<Vec<String>> {
    let dirs = crate::plugin_commands::search_dirs(cx)?;
    let profile =
        crate::doc_types::with_user(cx, |user| user.profile().cloned()).unwrap_or_default();
    let settings = PluginSettings::from_profile(&profile);
    let mut key = vec![
        dirs.consent_file().display().to_string(),
        settings.enabled.to_string(),
    ];
    for rec in &settings.table {
        key.push(format!(
            "{}|{}|{:?}|{}",
            rec.id,
            rec.name,
            rec.state,
            rec.is_used()
        ));
    }
    let stamp = std::fs::metadata(dirs.consent_file())
        .ok()
        .map(|meta| (meta.len(), meta.modified().ok()));
    key.push(format!("{stamp:?}"));
    Some(key)
}

fn compute_choices(cx: &App) -> Vec<(String, i32)> {
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return Vec::new();
    };
    let profile =
        crate::doc_types::with_user(cx, |user| user.profile().cloned()).unwrap_or_default();
    let settings = PluginSettings::from_profile(&profile);
    if !settings.enabled {
        return Vec::new();
    }
    let consent = ConsentFile::load(&dirs.consent_file());
    let mut found = Vec::new();
    for (slot, rec) in settings.table.iter().enumerate() {
        if !rec.is_used() || rec.state == PluginState::Deleted || rec.name.is_empty() {
            continue;
        }
        let Some(def) = dirs.load_def(&rec.name, DEFAULT_LANGUAGE) else {
            continue;
        };
        if def.id != rec.id || def.unsupported().is_some() || !def.resident {
            continue;
        }
        let dir = dirs.plugin_dir(&rec.name);
        if !matches!(
            consent.consent_for_dir(&def.id, dir.as_deref()),
            Consent::On
        ) {
            continue;
        }
        let Some((_, label)) = complement_plug(&dirs, &rec.name) else {
            continue;
        };
        if let Some(code) = plugin_commands::slot_code(slot) {
            found.push((label, i32::from(code)));
        }
    }
    found
}

impl EditorView {
    /// `F_HOKAN` opened the list `found`: asks the plugins that have a completion jack. Returns whether
    /// any call was made; if not, nothing here changed and the list is Sakura's alone.
    pub(crate) fn start_complement(
        &mut self,
        found: &Completion,
        ignore_case: bool,
        by_file: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        // Another plugin or a macro has the window, or the user is composing: no call is made.
        if self.plugin_in_call(cx) || self.composing {
            return false;
        }
        self.support.complement.last_id += 1;
        let id = self.support.complement.last_id;
        let option = if ignore_case { OPTION_IGNORE_CASE } else { 0 }
            | if by_file { OPTION_BY_FILE } else { 0 };
        let list: Shared = Rc::new(RefCell::new(Collector::new(found.items.clone())));
        let type_index = self.type_index();
        let calls = prepare_calls(cx, type_index, id, &found.key, option, &list);
        if calls.is_empty() {
            return false;
        }
        self.support.complement.session = Some(Session {
            id,
            version: self.document.read(cx).version(),
            caret: self.cursor.caret(),
        });
        cx.spawn(async move |this, cx| {
            for call in calls {
                let started = cx.update(|app| {
                    let Some(entity) = this.upgrade() else {
                        return false;
                    };
                    // The list may be closed, or begin again, while an earlier call ran.
                    if !entity.read(app).complement_is_current(id) {
                        return false;
                    }
                    // The plugin may have been switched off, or its files changed, while an earlier
                    // call ran: the agreement of the user is looked at again right before the call.
                    let Some(dirs) = crate::plugin_commands::search_dirs(app) else {
                        return false;
                    };
                    let Some(call) = call.refresh(app, &dirs) else {
                        return false;
                    };
                    // The host serves one call at a time: if the plugin runs for another window, this
                    // source is left out without a word (ADR-0026: a completion is not waited for).
                    if call.host_busy() {
                        return false;
                    }
                    plugin_call::run(entity, call, app)
                });
                if !started {
                    continue;
                }
                while this
                    .read_with(cx, |view, _| view.plugin_call_running())
                    .unwrap_or(false)
                {
                    cx.background_executor().timer(TICK).await;
                }
            }
            this.update(cx, |view, cx| view.complement_finished(id, cx))
                .ok();
        })
        .detach();
        true
    }

    /// Whether the session `id` is the one asking the plugins now, for a list that is still open (a
    /// list the user closed with Esc is not asked for any more).
    fn complement_is_current(&self, id: u64) -> bool {
        self.support.completion.is_some()
            && self
                .support
                .complement
                .session
                .as_ref()
                .is_some_and(|session| session.id == id)
    }

    /// A request of a plugin was answered: the candidates it added are shown (`list` is the one of the
    /// session `id`). If the text or the caret moved since the list opened, they are dropped, and the
    /// plugins are not asked any more.
    pub(crate) fn complement_arrived(&mut self, id: u64, list: &Shared, cx: &mut Context<Self>) {
        let Some(session) = self
            .support
            .complement
            .session
            .as_ref()
            .filter(|session| session.id == id)
        else {
            return;
        };
        if self.document.read(cx).version() != session.version
            || self.cursor.caret() != session.caret
        {
            self.stop_complement();
            return;
        }
        let new = list.borrow_mut().take_new();
        if new.is_empty() {
            return;
        }
        self.support.complement.words.extend(new.iter().cloned());
        if let Some(open) = self.support.completion.as_mut() {
            for word in new {
                if !open.items.contains(&word) {
                    open.items.push(word);
                }
            }
        }
        cx.notify();
    }

    /// Every call of the session `id` is over: the list is final. A list that was kept back (the other
    /// sources had fewer than two candidates) follows Sakura's rules now: no candidate, nothing is shown;
    /// one, it is inserted at once.
    fn complement_finished(&mut self, id: u64, cx: &mut Context<Self>) {
        if self
            .support
            .complement
            .session
            .as_ref()
            .is_none_or(|session| session.id != id)
        {
            return;
        }
        self.support.complement.session = None;
        let Some(open) = self.support.completion.as_mut() else {
            return;
        };
        let kept_back = !open.is_shown();
        open.pending = false;
        if kept_back {
            let only = (open.items.len() == 1).then(|| open.items[0].clone());
            if open.items.len() < 2 {
                self.support.completion = None;
            }
            if let Some(word) = only {
                self.insert_completion(&word, cx);
            }
        }
        cx.notify();
    }

    /// The plugins are not asked any more (the text changed, or the list went away). A list that was
    /// kept back is closed: it never was on the screen.
    pub(crate) fn stop_complement(&mut self) {
        let was_waiting = self.support.complement.waiting();
        self.support.complement.stop();
        if was_waiting && let Some(open) = self.support.completion.as_mut() {
            let kept_back = !open.is_shown();
            open.pending = false;
            if kept_back {
                self.support.completion = None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
