//! The keeper: which documents have a copy, and when it is written (ADR-0022, sections 1 and 2).
//!
//! Every window runs [`tick_window`] twice a second. For each tab it asks whether the document is to be kept
//! (it has unsaved changes, is not a Grep window, is not larger than the limit) and whether its copy is out of
//! date, and hands the [`Writer`] a snapshot when the text has been still for [`DEBOUNCE`], or has been
//! changing for [`MAX_WAIT`]. A document that is no longer to be kept (saved, changed back, closed with 「いいえ」)
//! has its copy discarded.
//!
//! [`decide`] is the whole decision, without GPUI, so that the times can be tested.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use gpui::{App, Context, EntityId, Global, WeakEntity, Window, WindowId};
use matsu_config::unsaved::{DocInfo, Session, SessionWindow, Store, StoreError};
use matsu_core::buffer::Eol;
use matsu_core::buffer::col_to_utf16;

use super::prefs::{KeepPrefs, prefs};
use super::writer::{Revision, Shared, Snapshot, Writer};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::view::EditorView;

/// The text is written this long after the last change (ADR-0022, section 2).
pub(crate) const DEBOUNCE: Duration = Duration::from_secs(2);
/// A document that keeps changing is written at least this often.
pub(crate) const MAX_WAIT: Duration = Duration::from_secs(30);
/// After a write failed, the next try is this much later.
pub(crate) const RETRY: Duration = Duration::from_secs(5);
/// A window that no tick has reported for this long is not in the session any more (it was closed, or it is
/// stuck). A document is gone when its entity is dropped, not when time has passed: a window that stalls for
/// a while does not lose its copies.
pub(crate) const GONE_AFTER: Duration = Duration::from_secs(5);
/// How often a window looks at its documents.
const TICK: Duration = Duration::from_millis(500);

/// What a document needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Want {
    /// It has unsaved changes: keep a copy.
    Keep,
    /// It has none (or is a Grep window): no copy.
    Drop,
    /// It has unsaved changes but is larger than `KeepUnsavedMaxMB`: no copy, and the user is told once.
    TooBig,
}

/// What the keeper knows about one document.
#[derive(Debug)]
pub(crate) struct Entry {
    /// The name of the copy, once it has been queued.
    pub id: Option<String>,
    pub shared: Arc<Shared>,
    /// The revision the timers below count from.
    seen: Option<Revision>,
    last_change: Instant,
    first_pending: Option<Instant>,
    last_attempt: Option<Instant>,
    /// The document, to tell that it is gone (all its views and tabs were dropped).
    pub document: Option<WeakEntity<SharedDocument>>,
    warned_big: bool,
}

impl Entry {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            id: None,
            shared: Arc::new(Shared::default()),
            seen: None,
            last_change: now,
            first_pending: None,
            last_attempt: None,
            document: None,
            warned_big: false,
        }
    }

    /// Forgets the copy (after it was discarded or cleared). The next change starts a new one.
    pub(crate) fn forget_copy(&mut self) {
        self.id = None;
        self.shared = Arc::new(Shared::default());
        self.seen = None;
        self.first_pending = None;
        self.last_attempt = None;
    }
}

/// What to do for one document at one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    pub write: bool,
    pub discard: bool,
    pub warn_too_big: bool,
}

/// The decision of one tick for one document. `current` is the revision it has now.
pub(crate) fn decide(entry: &mut Entry, want: Want, current: Revision, now: Instant) -> Plan {
    let mut plan = Plan::default();
    match want {
        Want::Drop | Want::TooBig => {
            plan.discard = entry.id.is_some();
            entry.seen = None;
            entry.first_pending = None;
            if want == Want::TooBig && !entry.warned_big {
                entry.warned_big = true;
                plan.warn_too_big = true;
            }
            if want == Want::Drop {
                entry.warned_big = false;
            }
        }
        Want::Keep => {
            entry.warned_big = false;
            if entry.shared.written() == Some(current) {
                entry.seen = Some(current);
                entry.first_pending = None;
                return plan;
            }
            // The timers go on while a write runs, so that a change made meanwhile is written as soon as
            // the write is over and the text has been still long enough.
            if entry.seen != Some(current) {
                entry.seen = Some(current);
                entry.last_change = now;
                entry.first_pending.get_or_insert(now);
            }
            if entry
                .shared
                .in_flight
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                return plan;
            }
            let failed = entry
                .shared
                .failed
                .load(std::sync::atomic::Ordering::SeqCst);
            if failed
                && entry
                    .last_attempt
                    .is_some_and(|t| now.duration_since(t) < RETRY)
            {
                return plan;
            }
            let still = now.duration_since(entry.last_change) >= DEBOUNCE;
            let long = entry
                .first_pending
                .is_some_and(|t| now.duration_since(t) >= MAX_WAIT);
            if still || long {
                entry.last_attempt = Some(now);
                entry.first_pending = None;
                plan.write = true;
            }
        }
    }
    plan
}

/// The tabs of one window as the last tick saw them.
#[derive(Debug)]
struct WindowReport {
    ids: Vec<String>,
    active: usize,
    at: Instant,
}

enum State {
    /// The folder has not been opened yet.
    NotStarted,
    Running(Writer),
    /// The folder is in use by another process, or cannot be used. Said once.
    Unavailable,
}

/// The state of the feature in this process.
pub(crate) struct Keeper {
    state: State,
    entries: HashMap<EntityId, Entry>,
    windows: Vec<(WindowId, WindowReport)>,
    active_window: Option<WindowId>,
    /// The session `session.ini` was last asked to have; `None` when that write failed and must be tried again.
    last_session: Option<Session>,
    session_retry_at: Option<Instant>,
    told_failure: bool,
}

impl Global for Keeper {}

impl Keeper {
    fn new() -> Self {
        Self {
            state: State::NotStarted,
            entries: HashMap::new(),
            windows: Vec::new(),
            active_window: None,
            last_session: Some(Session::default()),
            session_retry_at: None,
            told_failure: false,
        }
    }

    /// The session the reports make: the windows that have a copy, their tabs, the front tab and window.
    fn session(&self) -> Session {
        let live: std::collections::HashSet<&str> = self
            .entries
            .values()
            .filter_map(|e| e.id.as_deref())
            .collect();
        let mut windows = Vec::new();
        let mut active_window = 0;
        for (window, report) in &self.windows {
            // A document that is gone since the window reported is left out.
            let docs: Vec<String> = report
                .ids
                .iter()
                .filter(|id| live.contains(id.as_str()))
                .cloned()
                .collect();
            if docs.is_empty() {
                continue;
            }
            let before_active = report.ids[..report.active.min(report.ids.len())]
                .iter()
                .filter(|id| live.contains(id.as_str()))
                .count();
            if self.active_window == Some(*window) {
                active_window = windows.len();
            }
            windows.push(SessionWindow {
                active: before_active.min(docs.len() - 1),
                docs,
            });
        }
        Session {
            windows,
            active_window,
        }
    }

    /// Discards what no window reports any more, and writes `session.ini` when the tabs changed.
    fn housekeeping(&mut self, now: Instant) {
        let State::Running(writer) = &self.state else {
            return;
        };
        self.windows
            .retain(|(_, r)| now.duration_since(r.at) < GONE_AFTER);
        self.entries.retain(|_, entry| {
            if entry
                .document
                .as_ref()
                .is_none_or(|d| d.upgrade().is_some())
            {
                return true;
            }
            if let Some(id) = entry.id.take() {
                writer.discard(id);
            }
            false
        });
        // A write of `session.ini` that failed is asked again, but not more often than a document is.
        if writer.session_failed()
            && self.last_session.is_some()
            && self.session_retry_at.is_none_or(|t| now >= t)
        {
            self.last_session = None;
            self.session_retry_at = Some(now + RETRY);
        }
        let session = self.session();
        if self.last_session.as_ref() != Some(&session) {
            writer.session(session.clone());
            self.last_session = Some(session);
        }
    }

    /// While the feature is off nothing is "gone": the copies stay as the user left them (ADR-0022, section 8:
    /// 「残す」), and the windows that start ticking again are not taken for new ones.
    fn freeze(&mut self, now: Instant) {
        for (_, report) in &mut self.windows {
            report.at = now;
        }
    }

    /// The copy of the document is not wanted any more (it was closed or saved): it is discarded now, and the
    /// tabs no longer name it.
    fn discard_document(&mut self, document: EntityId) {
        let State::Running(writer) = &self.state else {
            return;
        };
        let Some(entry) = self.entries.get_mut(&document) else {
            return;
        };
        if let Some(id) = entry.id.clone() {
            writer.discard(id.clone());
            for (_, report) in &mut self.windows {
                if let Some(position) = report.ids.iter().position(|i| *i == id) {
                    report.ids.remove(position);
                    report.active = report.active.min(report.ids.len().saturating_sub(1));
                }
            }
        }
        entry.forget_copy();
    }

    /// Starts the writer for the settings file at `ini_path` the first time it is needed. Returns what to tell the
    /// user once when the folder cannot be used (another MATSU Editor holds it, or it cannot be made).
    fn ensure_started(&mut self, ini_path: Option<&std::path::Path>) -> Option<&'static str> {
        if !matches!(self.state, State::NotStarted) {
            return None;
        }
        let (state, message) = match ini_path {
            None => (State::Unavailable, None),
            Some(path) => match Writer::start(&matsu_config::unsaved::unsaved_dir(path)) {
                Ok(writer) => (State::Running(writer), None),
                Err(StoreError::Locked) => (
                    State::Unavailable,
                    Some("別のマツエディタが使っているため、保存していない文書を残しません。"),
                ),
                Err(StoreError::Io(_)) => (
                    State::Unavailable,
                    Some("保存していない文書を残すフォルダーを使えません。"),
                ),
            },
        };
        self.state = state;
        message
    }
}

/// Removes every copy and forgets them: the user turned the feature off and chose to delete what is kept
/// (ADR-0022, section 8). The session and the documents go; the folder and its lock stay.
pub(crate) fn clear_copies(cx: &mut App) {
    let mut keeper = if cx.has_global::<Keeper>() {
        cx.remove_global::<Keeper>()
    } else {
        Keeper::new()
    };
    let ini_path = settings_path(cx);
    keeper.ensure_started(ini_path.as_deref());
    if let State::Running(writer) = &keeper.state {
        writer.clear();
        writer.flush();
    }
    for entry in keeper.entries.values_mut() {
        entry.forget_copy();
    }
    keeper.windows.clear();
    keeper.last_session = Some(Session::default());
    cx.set_global(keeper);
}

/// The folder next to the settings file the process uses.
fn settings_path(cx: &App) -> Option<std::path::PathBuf> {
    if let Some(path) = cx
        .try_global::<crate::settings::UserSettings>()
        .and_then(|u| u.path())
    {
        return Some(path.to_owned());
    }
    let candidates = matsu_config::location::candidate_paths(
        matsu_config::location::Os::current(),
        &matsu_config::location::Environment::from_process(),
    );
    matsu_config::location::write_path(&candidates).map(std::path::Path::to_path_buf)
}

/// What a document needs, from its view and the settings.
fn want_of(view: &EditorView, shared: &SharedDocument, prefs: &KeepPrefs) -> Want {
    if view.is_grep_window() {
        return Want::Drop;
    }
    if !(shared.model().is_modified() || shared.format_changed()) {
        return Want::Drop;
    }
    if let Some(max) = prefs.max_bytes()
        && shared.doc().raw_len() as u64 > max
    {
        return Want::TooBig;
    }
    Want::Keep
}

fn revision_of(shared: &SharedDocument) -> Revision {
    Revision {
        version: shared.version(),
        encoding: shared.file().encoding,
        bom: shared.file().bom,
        eol: shared.new_line(),
    }
}

/// The information kept with the copy (ADR-0022, section 3): what is needed to bring the document back and
/// to save it as it was going to be saved.
pub(crate) fn info_of(view: &EditorView, shared: &SharedDocument) -> DocInfo {
    let doc = shared.doc();
    let caret = view.cursor().caret();
    let column = doc
        .line(caret.line)
        .ok()
        .flatten()
        .map_or(caret.col, |line| col_to_utf16(&line.text, caret.col));
    let stamp = shared
        .disk()
        .stamp
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
    // The size the file had when the document was opened is not kept; the size now counts only while the
    // time stamp is still the one the document was opened with.
    let file_size = shared.path().and_then(|path| {
        let meta = std::fs::metadata(path).ok()?;
        (meta.modified().ok() == shared.disk().stamp).then_some(meta.len())
    });
    DocInfo {
        untitled_number: shared.untitled_number(),
        path: shared.path().map(|p| p.to_string_lossy().into_owned()),
        file_modified_secs: stamp.map(|d| d.as_secs() as i64),
        file_modified_nanos: stamp.map(|d| d.subsec_nanos()),
        file_size,
        encoding: shared.file().encoding.sakura_code().to_string(),
        bom: shared.file().bom,
        line_ending: eol_name(shared.new_line()).to_owned(),
        doc_type: None,
        caret_line: caret.line as u64,
        caret_column: column as u64,
        top_line: view.top_row() as u64,
        bookmarks: shared
            .model()
            .bookmarks()
            .iter()
            .map(|line| line as u64)
            .collect(),
    }
}

/// The name of a line ending in the information of a copy.
pub(crate) fn eol_name(eol: Eol) -> &'static str {
    match eol {
        Eol::CrLf => "CRLF",
        Eol::Cr => "CR",
        _ => "LF",
    }
}

/// The work that does not belong to a window: discards what no window reports any more (also when no window is
/// left to tick), and keeps what the user chose to keep while the feature is off.
pub(crate) fn app_tick(cx: &mut App, now: Instant) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let keep = prefs(cx).keep;
    let mut keeper = cx.remove_global::<Keeper>();
    if keep {
        keeper.housekeeping(now);
    } else {
        keeper.freeze(now);
    }
    cx.set_global(keeper);
}

/// The document is going away or was saved: its copy is discarded now, without waiting for the housekeeping
/// (a process that quits right after the last window closed would leave it behind).
pub(crate) fn discard_document(document: EntityId, cx: &mut App) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let mut keeper = cx.remove_global::<Keeper>();
    keeper.discard_document(document);
    cx.set_global(keeper);
}

/// Starts the timer of a window. The first window also starts the keeper.
pub(crate) fn start(window: &mut Window, cx: &mut Context<Workspace>) {
    if !cx.has_global::<Keeper>() {
        cx.set_global(Keeper::new());
        // Documents whose window is gone are discarded even when no window is left to tick.
        let app: &mut App = cx;
        app.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                cx.update(|app| app_tick(app, Instant::now()));
            }
        })
        .detach();
    }
    cx.spawn_in(window, async move |this, cx| {
        loop {
            cx.background_executor().timer(TICK).await;
            let alive = this.update_in(cx, |workspace, window, cx| {
                tick_window(workspace, window, cx);
            });
            if alive.is_err() {
                return;
            }
        }
    })
    .detach();
}

/// One tick of one window.
pub(crate) fn tick_window(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    tick_window_at(workspace, window, cx, Instant::now());
}

/// [`tick_window`] at the time `now`; the tests give the times.
pub(crate) fn tick_window_at(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
    now: Instant,
) {
    let prefs = prefs(cx);
    if !prefs.keep {
        return;
    }
    if !cx.has_global::<Keeper>() {
        return;
    }
    let mut keeper = cx.remove_global::<Keeper>();
    let ini_path = settings_path(cx);
    if let Some(message) = keeper.ensure_started(ini_path.as_deref()) {
        // Said once, in the window that found it out.
        workspace.editor().clone().update(cx, |view, cx| {
            view.notice = Some(message.to_owned());
            cx.notify();
        });
    }
    if let State::Running(writer) = &keeper.state {
        let tabs = workspace.tab_editors();
        let mut ids = Vec::new();
        let mut active = 0;
        let mut failed_notice = None;
        for (index, view) in tabs.iter().enumerate() {
            let document = view.read(cx).document().clone();
            let key = document.entity_id();
            let (want, revision) = {
                let shared = document.read(cx);
                (want_of(view.read(cx), shared, &prefs), revision_of(shared))
            };
            let entry = keeper.entries.entry(key).or_insert_with(|| Entry::new(now));
            entry.document.get_or_insert_with(|| document.downgrade());
            let plan = decide(entry, want, revision, now);
            if plan.discard {
                if let Some(id) = entry.id.clone() {
                    writer.discard(id);
                }
                entry.forget_copy();
            }
            if plan.write {
                let id = entry.id.get_or_insert_with(Store::new_id).clone();
                let snapshot = {
                    let shared = document.read(cx);
                    Snapshot {
                        doc: shared.doc().clone(),
                        info: info_of(view.read(cx), shared),
                    }
                };
                writer.write(id, &entry.shared, revision, snapshot);
            }
            if plan.warn_too_big {
                view.update(cx, |view, cx| {
                    view.notice = Some("大きいため、保存していない内容を残しません。".to_owned());
                    cx.notify();
                });
            }
            if entry
                .shared
                .failed
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                failed_notice = Some(view.clone());
            }
            if let Some(id) = entry.id.clone().filter(|_| want == Want::Keep) {
                if index == workspace.active_tab() {
                    active = ids.len();
                }
                ids.push(id);
            }
        }
        if let Some(view) = failed_notice.filter(|_| !keeper.told_failure) {
            keeper.told_failure = true;
            view.update(cx, |view, cx| {
                view.notice = Some("保存していない文書を残せません。".to_owned());
                cx.notify();
            });
        } else if !writer.failed() {
            keeper.told_failure = false;
        }
        let id = window.window_handle().window_id();
        let report = WindowReport {
            ids,
            active,
            at: now,
        };
        match keeper.windows.iter_mut().find(|(w, _)| *w == id) {
            Some((_, old)) => *old = report,
            None => keeper.windows.push((id, report)),
        }
        if window.is_window_active() {
            keeper.active_window = Some(id);
        }
        keeper.housekeeping(now);
    }
    cx.set_global(keeper);
}

/// Waits until the writer has done every job queued so far (for the tests; the quit flow of a later change
/// uses it too).
#[cfg(test)]
pub(crate) fn flush(cx: &App) {
    if let Some(Keeper {
        state: State::Running(writer),
        ..
    }) = cx.try_global::<Keeper>()
    {
        writer.flush();
    }
}
