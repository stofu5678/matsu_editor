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

use gpui::{App, Context, Entity, EntityId, Global, WeakEntity, Window, WindowId};
use matsu_config::unsaved::{DocInfo, Session, SessionWindow, Store, StoreError};
use matsu_core::buffer::Eol;
use matsu_core::buffer::col_to_utf16;

use super::prefs::{KeepPrefs, prefs};
use super::writer::{RETRY_STUCK, Revision, Shared, Snapshot, Writer};
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
    /// The revision the user chose to throw away (「保存しない」 while the document stays open, as in an exit of all
    /// windows): no copy is made of it again; the next change makes one.
    suppressed: Option<Revision>,
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
            suppressed: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn suppress_for_test(&mut self, revision: Revision) {
        self.suppressed = Some(revision);
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
            // A change ends the suppression: a later return to the same revision is a revision like any other.
            if entry.suppressed.is_some_and(|s| s != current) {
                entry.suppressed = None;
            }
            if entry.suppressed == Some(current) {
                entry.seen = Some(current);
                entry.first_pending = None;
                return plan;
            }
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
    /// Documents whose user said 「保存しない」 in a flow that is not over: the copy goes when the close really
    /// happens ([`apply_close`]), not before, so a cancelled flow loses nothing.
    marked: HashMap<EntityId, (WeakEntity<EditorView>, Revision)>,
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
            marked: HashMap::new(),
        }
    }

    /// Tells the user once when something in the folder fails to be written or removed (a document, `session.ini`,
    /// a copy to discard, a clear), and again after it worked in between. `document_failed` is a failure of the
    /// write of a document the window knows.
    fn tell_failure(&mut self, document_failed: bool, view: &Entity<EditorView>, cx: &mut App) {
        let writer_failed =
            matches!(&self.state, State::Running(w) if w.failed() || w.session_failed());
        // Every document counts, not only the ones of this window: one latch for the whole process.
        let any_document_failed = self
            .entries
            .values()
            .any(|e| e.shared.failed.load(std::sync::atomic::Ordering::SeqCst));
        if document_failed || any_document_failed || writer_failed {
            if !self.told_failure {
                self.told_failure = true;
                view.update(cx, |view, cx| {
                    view.notice = Some(
                        "保存していない文書のコピーを書けない、または消せないものがあります。"
                            .to_owned(),
                    );
                    cx.notify();
                });
            }
        } else {
            self.told_failure = false;
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
        // The bookkeeping is cleaned also before the writer exists (nothing was kept yet).
        let writer = match &self.state {
            State::Running(writer) => Some(writer),
            _ => None,
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
            if let (Some(writer), Some(id)) = (writer, entry.id.take()) {
                writer.discard(id);
            }
            false
        });
        let Some(writer) = writer else {
            return;
        };
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

    /// The document was saved: the copy goes (no suppression: a later edit is a new revision).
    fn copy_saved(&mut self, document: EntityId) {
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

    /// The copy of the document is not wanted any more (it was closed or saved): it is discarded now, and the
    /// tabs no longer name it.
    fn discard_document(&mut self, document: &Entity<SharedDocument>, revision: Revision) {
        // Also before the first tick and before the writer exists: the revision is not to be copied later.
        let entry = self
            .entries
            .entry(document.entity_id())
            .or_insert_with(|| Entry::new(Instant::now()));
        // So that the entry goes when the document does, also if no tick ever saw it.
        entry.document.get_or_insert_with(|| document.downgrade());
        entry.suppressed = Some(revision);
        let State::Running(writer) = &self.state else {
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
///
/// The clear is queued, not waited for: the UI thread does not stop for a write that is still going. Returns what
/// to tell the user when the folder cannot be used, so that the deletion cannot be done.
pub(crate) fn clear_copies(cx: &mut App) -> Option<&'static str> {
    let mut keeper = if cx.has_global::<Keeper>() {
        cx.remove_global::<Keeper>()
    } else {
        Keeper::new()
    };
    let ini_path = settings_path(cx);
    keeper.ensure_started(ini_path.as_deref());
    let mut problem = None;
    match &keeper.state {
        State::Running(writer) => writer.clear(),
        _ => {
            problem = Some(
                "別のマツエディタが使っているか、フォルダーを使えないため、残したコピーを消せませんでした。",
            );
        }
    }
    for entry in keeper.entries.values_mut() {
        entry.forget_copy();
    }
    keeper.windows.clear();
    keeper.last_session = Some(Session::default());
    cx.set_global(keeper);
    problem
}

/// A restored document, as the keeper takes it over.
pub(crate) struct Adopted {
    pub view: Entity<EditorView>,
    /// The name of the copy it was brought back from: it is the copy of the document from now on.
    pub id: String,
}

/// The restore opened the folder: the writer goes on with it, and each restored document keeps its copy (under
/// the same name, as written as it is) until it changes. Otherwise the first tick would write a second copy of
/// every restored document, and the next start would bring each back twice.
pub(crate) fn adopt_restored(cx: &mut App, store: Store, items: Vec<Adopted>) {
    let mut keeper = if cx.has_global::<Keeper>() {
        cx.remove_global::<Keeper>()
    } else {
        Keeper::new()
    };
    if matches!(keeper.state, State::NotStarted) {
        keeper.state = match Writer::with_store(store, RETRY_STUCK) {
            Ok(writer) => State::Running(writer),
            Err(_) => State::Unavailable,
        };
    }
    let now = Instant::now();
    for item in items {
        let view = item.view.read(cx);
        let document = view.document().clone();
        let shared = document.read(cx);
        let revision = revision_of(view, shared);
        let info = info_of(view, shared);
        let entry = keeper
            .entries
            .entry(document.entity_id())
            .or_insert_with(|| Entry::new(now));
        entry.document = Some(document.downgrade());
        entry.id = Some(item.id);
        *entry
            .shared
            .written
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(revision);
        *entry
            .shared
            .written_info
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(info);
    }
    cx.set_global(keeper);
}

/// The folder next to the settings file the process uses.
pub(crate) fn settings_path(cx: &App) -> Option<std::path::PathBuf> {
    if let Some(path) = cx
        .try_global::<crate::settings::UserSettings>()
        .and_then(|u| u.path())
    {
        return Some(path.to_owned());
    }
    // The tests never touch the folders of the user who runs them.
    #[cfg(test)]
    return None;
    #[cfg(not(test))]
    settings_path_of_user()
}

#[cfg(not(test))]
fn settings_path_of_user() -> Option<std::path::PathBuf> {
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
    if let Some(max) = prefs.max_bytes() {
        // A document that was brought back has its text as UTF-8, which can be bigger than the bytes it had in
        // its own encoding (Japanese text in Shift_JIS grows by half). It may be as big as it came back, and a
        // little more (an eighth of the limit), so that the copy it came from is not discarded for the way it was
        // restored, nor at the first character typed.
        let came_back = shared.restored_len() as u64;
        let allowed = if came_back > max {
            came_back.saturating_add(max / 8)
        } else {
            max
        };
        if shared.doc().raw_len() as u64 > allowed {
            return Want::TooBig;
        }
    }
    Want::Keep
}

fn revision_of(view: &EditorView, shared: &SharedDocument) -> Revision {
    Revision {
        version: shared.version(),
        encoding: shared.file().encoding,
        bom: shared.file().bom,
        eol: shared.new_line(),
        doc_type: doc_type_of(view, shared),
    }
}

/// The number of the type the user chose, if they did (the file name brings back the others).
fn doc_type_of(view: &EditorView, shared: &SharedDocument) -> Option<i32> {
    shared.type_locked().then(|| view.type_index() as i32)
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
    // A document that was brought back and whose file is not looked at yet keeps the time and the size it came with
    // (the watching of the file may have taken the new time since: the choice 「以後通知メッセージのみ」), so
    // that the check before the save still works after another restart.
    let (stamp, file_size) = match shared.restored_check() {
        Some(check) => (check.modified, check.size),
        None => {
            // The size the file had when the document was opened is not kept; the size now counts only while
            // the time stamp is still the one the document was opened with.
            let size = shared.path().and_then(|path| {
                let meta = std::fs::metadata(path).ok()?;
                (meta.modified().ok() == shared.disk().stamp).then_some(meta.len())
            });
            (shared.disk().stamp, size)
        }
    };
    let stamp = stamp.and_then(|t| t.duration_since(UNIX_EPOCH).ok());
    DocInfo {
        untitled_number: shared.untitled_number(),
        path: shared.path().map(|p| p.to_string_lossy().into_owned()),
        file_modified_secs: stamp.map(|d| d.as_secs() as i64),
        file_modified_nanos: stamp.map(|d| d.subsec_nanos()),
        file_size,
        encoding: shared.file().encoding.sakura_code().to_string(),
        bom: shared.file().bom,
        line_ending: eol_name(shared.new_line()).to_owned(),
        // Only a type the user chose is kept: the file name brings back the others.
        doc_type: doc_type_of(view, shared),
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

/// The user said 「保存しない」 (or saved) in a flow that closes the document: it is remembered, with the revision
/// it has now. Nothing is deleted yet: if the flow is cancelled (the next question of an exit, a file chooser),
/// the copy is still there. [`apply_close`] and [`apply_all_closes`] make it final.
pub(crate) fn mark_close(view: &EditorView, this: WeakEntity<EditorView>, cx: &mut App) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let document = view.document();
    let revision = revision_of(view, document.read(cx));
    let id = document.entity_id();
    let mut keeper = cx.remove_global::<Keeper>();
    keeper.marked.insert(id, (this, revision));
    cx.set_global(keeper);
}

/// A flow asks about the document again: the mark of the earlier answer is gone.
pub(crate) fn unmark_close(view: &EditorView, cx: &mut App) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let id = view.document().entity_id();
    let mut keeper = cx.remove_global::<Keeper>();
    keeper.marked.remove(&id);
    cx.set_global(keeper);
}

/// The document is really closing: if it is marked and still has the revision it had then, its copy is
/// discarded now, without waiting for the housekeeping (a process that quits right after the last window closed
/// would leave it behind), and that revision is not copied again.
pub(crate) fn apply_close(view: &Entity<EditorView>, cx: &mut App) {
    let view = view.read(cx).state_for_close(cx);
    apply_close_of(view, cx);
}

/// [`apply_close`] for code that has the view borrowed (the view is replacing its own document).
pub(crate) fn apply_close_view(view: &EditorView, cx: &mut App) {
    apply_close_of(view.state_for_close(cx), cx);
}

/// The document of a view and the revision it has.
pub(crate) struct CloseState {
    document: Entity<SharedDocument>,
    revision: Revision,
}

impl EditorView {
    fn state_for_close(&self, cx: &App) -> CloseState {
        CloseState {
            document: self.document().clone(),
            revision: revision_of(self, self.document().read(cx)),
        }
    }
}

fn apply_close_of(state: CloseState, cx: &mut App) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let CloseState {
        document,
        revision: current,
    } = state;
    let mut keeper = cx.remove_global::<Keeper>();
    if keeper
        .marked
        .remove(&document.entity_id())
        .is_some_and(|(_, revision)| revision == current)
    {
        keeper.discard_document(&document, current);
    }
    cx.set_global(keeper);
}

/// The editor is about to quit with every question answered: every marked document that is unchanged since its
/// answer loses its copy.
pub(crate) fn apply_all_closes(cx: &mut App) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let marks: Vec<(WeakEntity<EditorView>, Revision)> = {
        let mut keeper = cx.remove_global::<Keeper>();
        let marks = keeper.marked.drain().map(|(_, mark)| mark).collect();
        cx.set_global(keeper);
        marks
    };
    for (view, revision) in marks {
        let Some(view) = view.upgrade() else {
            continue;
        };
        let current = {
            let view = view.read(cx);
            revision_of(view, view.document().read(cx))
        };
        if current == revision {
            let document = view.read(cx).document().clone();
            let mut keeper = cx.remove_global::<Keeper>();
            keeper.discard_document(&document, revision);
            cx.set_global(keeper);
        }
    }
}

/// The document was saved: its copy goes at once (and no snapshot queued before can come after it, the writer
/// does the jobs in order). If the document is edited again before the next tick, the edit makes a new copy.
pub(crate) fn copy_saved(document: EntityId, cx: &mut App) {
    if !cx.has_global::<Keeper>() {
        return;
    }
    let mut keeper = cx.remove_global::<Keeper>();
    keeper.copy_saved(document);
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
    // The window loses the focus (the user goes to another program): the latest text and place are written at
    // once, not after the wait (ADR-0022, section 2).
    cx.observe_window_activation(window, |workspace, window, cx| {
        if !window.is_window_active() {
            tick_window_at(workspace, window, cx, Instant::now(), true);
        }
    })
    .detach();
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
    tick_window_at(workspace, window, cx, Instant::now(), false);
}

/// [`tick_window`] at the time `now`; the tests give the times.
pub(crate) fn tick_window_at(
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
    now: Instant,
    force: bool,
) {
    let prefs = prefs(cx);
    if !prefs.keep {
        // A removal that failed after the user turned the feature off (and chose to delete) is still said.
        if cx.has_global::<Keeper>() {
            let mut keeper = cx.remove_global::<Keeper>();
            keeper.tell_failure(false, &workspace.editor().clone(), cx);
            cx.set_global(keeper);
        }
        return;
    }
    if !cx.has_global::<Keeper>() {
        return;
    }
    let mut keeper = cx.remove_global::<Keeper>();
    let tabs = workspace.tab_editors();
    // The folder is opened (and locked) when the first document has something to keep, not when the editor starts.
    let wants_keep = tabs.iter().any(|view| {
        let view = view.read(cx);
        want_of(view, view.document().read(cx), &prefs) == Want::Keep
    });
    if wants_keep {
        let ini_path = settings_path(cx);
        if let Some(message) = keeper.ensure_started(ini_path.as_deref()) {
            // Said once, in the window that found it out.
            workspace.editor().clone().update(cx, |view, cx| {
                view.notice = Some(message.to_owned());
                cx.notify();
            });
        }
    }
    if !matches!(keeper.state, State::Unavailable) {
        let writer = match &keeper.state {
            State::Running(writer) => Some(writer),
            _ => None,
        };
        let mut ids = Vec::new();
        let mut active = 0;
        let mut failed_notice = None;
        for (index, view) in tabs.iter().enumerate() {
            let document = view.read(cx).document().clone();
            let key = document.entity_id();
            let (want, revision) = {
                let shared = document.read(cx);
                (
                    want_of(view.read(cx), shared, &prefs),
                    revision_of(view.read(cx), shared),
                )
            };
            let entry = keeper.entries.entry(key).or_insert_with(|| Entry::new(now));
            entry.document.get_or_insert_with(|| document.downgrade());
            let mut plan = decide(entry, want, revision, now);
            // On `force` (the window lost the focus) a document that is to be kept is written now when its text,
            // or its caret, view or bookmarks, are not what the last write has.
            let mut info = None;
            if force
                && want == Want::Keep
                // A revision the user threw away stays thrown away.
                && entry.suppressed != Some(revision)
                && !plan.write
                && !entry
                    .shared
                    .in_flight
                    .load(std::sync::atomic::Ordering::SeqCst)
            {
                let now_info = info_of(view.read(cx), document.read(cx));
                if entry.shared.written() != Some(revision)
                    || entry.shared.written_info().as_ref() != Some(&now_info)
                {
                    plan.write = true;
                    entry.first_pending = None;
                    entry.last_attempt = Some(now);
                }
                info = Some(now_info);
            }
            if plan.discard {
                if let (Some(writer), Some(id)) = (writer, entry.id.clone()) {
                    writer.discard(id);
                }
                entry.forget_copy();
            }
            if let (true, Some(writer)) = (plan.write, writer) {
                let id = entry.id.get_or_insert_with(Store::new_id).clone();
                let snapshot = {
                    let shared = document.read(cx);
                    Snapshot {
                        doc: shared.doc().clone(),
                        source: shared.file().source_encoding(),
                        info: info.unwrap_or_else(|| info_of(view.read(cx), shared)),
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
        let view = failed_notice
            .clone()
            .unwrap_or_else(|| workspace.editor().clone());
        keeper.tell_failure(failed_notice.is_some(), &view, cx);
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

/// Waits until the writer has done every job queued so far (for the tests).
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

/// Whether the editor ends without the question 「保存しますか」 about the documents whose copy is kept:
/// `KeepUnsaved` and `KeepUnsavedNoAsk` are both on (ADR-0022, section 5, decision 5).
pub(crate) fn hot_exit_enabled(cx: &App) -> bool {
    let prefs = prefs(cx);
    prefs.keep && prefs.no_ask
}

/// Writes the latest text and place of every document that is to be kept, at once (as when a window loses the
/// focus), in every window.
///
/// `current` is the window that runs the call, if there is one: a window cannot be entered again from inside its own
/// update, so it is used as it is.
fn queue_all_now(cx: &mut App, mut current: Option<&mut Window>) {
    for (handle, workspace) in crate::file_ops::windows::workspaces(cx) {
        match current.as_deref_mut() {
            Some(window) if window.window_handle() == handle => {
                workspace.update(cx, |ws, wcx| {
                    tick_window_at(ws, window, wcx, Instant::now(), true);
                });
            }
            _ => {
                handle
                    .update(cx, |_, window, app| {
                        workspace.update(app, |ws, wcx| {
                            tick_window_at(ws, window, wcx, Instant::now(), true);
                        });
                    })
                    .ok();
            }
        }
    }
}

/// A receiver that answers when the writer has done what it was given so far.
fn barrier(cx: &App) -> Option<std::sync::mpsc::Receiver<()>> {
    match cx.try_global::<Keeper>() {
        Some(Keeper {
            state: State::Running(writer),
            ..
        }) => Some(writer.barrier()),
        _ => None,
    }
}

/// How long the editor waits for the copies to be written before it goes on to ask (a slow or stuck disk).
const SETTLE_TIMEOUT: Duration = Duration::from_secs(10);

/// Before the editor ends: the latest content of every document is written, and the call returns when the writer
/// is done (or after [`SETTLE_TIMEOUT`]). Two rounds, because a write that was running when the first round
/// asked cannot be asked again before it ends. The waiting is done on a thread of the background executor, not
/// on the UI thread.
pub(crate) async fn settle(cx: &mut gpui::AsyncWindowContext) {
    for _ in 0..2 {
        let rx = cx
            .update(|window, app| {
                queue_all_now(app, Some(window));
                barrier(app)
            })
            .ok()
            .flatten();
        let Some(rx) = rx else {
            return;
        };
        cx.background_executor()
            .spawn(async move {
                let _ = rx.recv_timeout(SETTLE_TIMEOUT);
            })
            .await;
    }
}

/// The part of [`settle`] for the end of the process, which cannot wait for a window: the writes are asked for,
/// and a future that ends when they are done is returned for `on_app_quit`.
pub(crate) fn settle_at_quit(cx: &mut App) -> impl std::future::Future<Output = ()> + use<> {
    queue_all_now(cx, None);
    let rx = barrier(cx);
    let executor = cx.background_executor().clone();
    let async_cx = cx.to_async();
    async move {
        let Some(rx) = rx else {
            return;
        };
        executor
            .spawn(async move {
                let _ = rx.recv_timeout(SETTLE_TIMEOUT);
            })
            .await;
        // A write that was running when the first round asked could not be asked again: a second round, now
        // that it is over, writes what was typed meanwhile.
        let second = async_cx.update(|app| {
            queue_all_now(app, None);
            barrier(app)
        });
        if let Some(rx) = second {
            executor
                .spawn(async move {
                    let _ = rx.recv_timeout(SETTLE_TIMEOUT);
                })
                .await;
        }
    }
}

/// Whether the copy of the document of `view` has its latest content, so that the editor may end without asking
/// about it: the document is one that is kept (not a Grep window, not too big), its copy was written for the
/// revision it has now, and the last write did not fail. Needs the folder to be in use by this process.
pub(crate) fn covered(view: &EditorView, cx: &App) -> bool {
    let Some(Keeper {
        state: State::Running(_),
        entries,
        ..
    }) = cx.try_global::<Keeper>()
    else {
        return false;
    };
    let shared = view.document().read(cx);
    if want_of(view, shared, &prefs(cx)) != Want::Keep {
        return false;
    }
    let Some(entry) = entries.get(&view.document().entity_id()) else {
        return false;
    };
    entry.id.is_some()
        && !entry
            .shared
            .failed
            .load(std::sync::atomic::Ordering::SeqCst)
        && entry.shared.written() == Some(revision_of(view, shared))
}
