//! The event jacks of the plugins (ADR-0026, chapter 3 and the sixth PR): `EditorStart`, `EditorEnd`,
//! `DocumentOpen`, `DocumentClose`, `DocumentBeforeSave` and `DocumentAfterSave`.
//!
//! The moments and the order are Sakura's (`CJackManager::InvokePlugins` and its callers:
//! `CNormalProcess.cpp`, `CEditWnd.cpp` `MYWM_CLOSE`, `CDocFileOperation.cpp`):
//!
//! * a file is read: `DocumentOpen` (after the macro of オープン後), also after a save under a name;
//! * a file is closed, read again, or a window closes: `DocumentClose` (after the question about
//!   the changes); a window that closes then gets `EditorEnd`;
//! * a save: `DocumentBeforeSave`; under a name, `DocumentClose` for the document that was; then the
//!   file is written; `DocumentAfterSave`; under a name, the macro of オープン後 and `DocumentOpen`;
//! * a window opens: `EditorStart`. MATSU has one process for all the windows, so the editor of Sakura
//!   is a window of ours (ADR-0026, chapter 2).
//!
//! Sakura calls a plug and waits for it, with no time limit, and goes on whatever the plug did
//! (`InvokePlugins` returns nothing). Here the call waits as long as the limit of the jack
//! (`matsu_plugin::plugin_host::LIMITS`, 5 seconds), without freezing the window; a plug that fails, throws
//! or is too slow is told in the output panel and the flow goes on, so a save is never refused or lost
//! because of a plugin.
//!
//! No plugin code runs for a key press: these are the moments of a file and a window.
//!
//! A jack is delivered to the plugins one after the other in the order of the plugin table, and for each
//! plugin in the order of its `[Plug]`. An event that comes while a command of a plugin runs is not given
//! (the handler's own actions do not call the plugins again); one that comes while another event runs waits
//! for its turn, so that `EditorStart` is over before the first `DocumentOpen`.
//!
//! A macro (a key macro or a script) that saves or reads a file gets no events for it, though Sakura calls the
//! plugins there. The event call takes the state of the window that a running macro holds (the flag of a
//! plugin call, the block of undo, `Turn::Taken`), and the macro waits for the very flow whose events would
//! need it; letting both in needs a state of its own for the event calls, which no foundation has yet.
//!
//! Sakura has one document in a window and never meets a window that was told `EditorEnd` and stays. Here a
//! tab can open, or the user can type, while the handlers of `EditorEnd` run: the window then is told
//! `EditorStart` again ([`restart_ended`]), and a flow that starts over (the exit of all windows, the close
//! of a window) tells each close once ([`CloseLog`]).
//!
//! A window that begins with a Grep (`-GREPMODE`) gets `EditorStart` and `DocumentOpen` after the search, as in
//! `CNormalProcess` ([`start_after_grep`]).

use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, AsyncApp, AsyncWindowContext, Entity, EntityId, WeakEntity};
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginSettings, PluginState};

use crate::app::Workspace;
use crate::macros::plugin_call::{self, Prepared, Turn};
use crate::view::EditorView;

/// How long the window waits before it says that it waits for a plugin.
const WAIT_NOTICE_AFTER: Duration = Duration::from_millis(300);

/// How often the waits look again.
const TICK: Duration = Duration::from_millis(10);

/// What the window says while it waits for a plugin.
const WAITING: &str = "プラグインを待っています";

/// The plugins that may run, in the order of the table, with their folders. Empty when plugins are off.
fn running_folders(cx: &mut App) -> Vec<String> {
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

/// The calls to make for `jack`, in order.
fn collect(cx: &mut App, jack: Jack) -> Vec<Prepared> {
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return Vec::new();
    };
    let mut calls = Vec::new();
    for folder in running_folders(cx) {
        calls.extend(plugin_call::prepare_event(cx, &dirs, &folder, jack));
    }
    calls
}

/// Whether a plugin that may run declares one of `jacks`. A window closes without waiting when none does,
/// as it did before the jacks.
pub(crate) fn has_plugs(cx: &mut App, jacks: &[Jack]) -> bool {
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return false;
    };
    running_folders(cx).iter().any(|folder| {
        dirs.load_def(folder, DEFAULT_LANGUAGE)
            .is_some_and(|def| def.plugs.iter().any(|plug| jacks.contains(&plug.jack)))
    })
}

/// What the attempt to start one call came to.
enum Step {
    /// The call runs.
    Started,
    /// It does not run, and will not: a command of a plugin or a macro has the window (the event is not
    /// given), the window is gone, or the call was refused.
    Skipped,
    /// Not now: another event runs on the window, or the host of the plugin is busy with another window.
    Wait,
}

/// Starts `call` if the window and the host of its plugin are free. Looking and starting are one step, so
/// that nothing else begins between them.
fn try_start(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncApp,
    call: Prepared,
    generation: u64,
) -> (Step, Option<Prepared>) {
    cx.update(|app| {
        let Some(entity) = view.upgrade() else {
            return (Step::Skipped, None);
        };
        // The event is for the document the window had when it was fired (ADR-0026, chapter 2): a file that
        // was opened into the window meanwhile is not given to the handlers of the old one.
        if entity.read(app).document().read(app).generation() != generation {
            return (Step::Skipped, None);
        }
        match entity.read(app).event_turn(app) {
            Turn::Taken => return (Step::Skipped, None),
            Turn::AfterEvent => return (Step::Wait, Some(call)),
            Turn::Free => {}
        }
        // One host serves all the windows: a call of the same plugin on another window goes first.
        if call.host_busy() {
            return (Step::Wait, Some(call));
        }
        // The call may have waited: the switch, the agreement and the files are looked at again now, and
        // the handler is read from the disk in the same step (`run`), so nothing changes in between.
        let Some(dirs) = crate::plugin_commands::search_dirs(app) else {
            return (Step::Skipped, None);
        };
        let Some(call) = call.refresh(app, &dirs) else {
            return (Step::Skipped, None);
        };
        if plugin_call::run(&entity, call, app) {
            (Step::Started, None)
        } else {
            (Step::Skipped, None)
        }
    })
}

/// Runs the calls of one event on `view` when the events before it on this window are over, so that a batch
/// is never cut in two by another (`EditorStart` ends before the first `DocumentOpen` begins).
async fn run_all(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncApp,
    calls: Vec<Prepared>,
    ticket: u64,
    generation: u64,
) {
    loop {
        match view.read_with(cx, |view, _| view.event_serving()) {
            Ok(serving) if serving == ticket => break,
            Ok(_) => cx.background_executor().timer(TICK).await,
            Err(_) => return,
        }
    }
    run_calls(view, cx, calls, generation).await;
    view.update(cx, |view, _| view.finish_event_ticket()).ok();
}

/// Runs every call of one event on `view`, one after the other, and waits for each.
async fn run_calls(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncApp,
    calls: Vec<Prepared>,
    generation: u64,
) {
    for call in calls {
        let mut pending = Some(call);
        let mut started = false;
        while let Some(call) = pending.take() {
            let (step, back) = try_start(view, cx, call, generation);
            match step {
                Step::Started => started = true,
                Step::Skipped => break,
                // Another event or window has the turn. Every call has its time limit, so the line moves;
                // the call stays in it until the host is free (only a document that is gone, or a plugin
                // that may not run any more, ends it: `Skipped`).
                Step::Wait => {
                    pending = back;
                    cx.background_executor().timer(TICK).await;
                }
            }
        }
        if !started {
            continue;
        }
        // The call is on its way; the window is free to paint meanwhile.
        let mut waited = Duration::ZERO;
        let mut told = false;
        while view
            .read_with(cx, |view, _| view.plugin_call_running())
            .unwrap_or(false)
        {
            cx.background_executor().timer(TICK).await;
            waited += TICK;
            if !told && waited >= WAIT_NOTICE_AFTER {
                told = true;
                view.update(cx, |view, cx| {
                    view.notice = Some(WAITING.to_owned());
                    cx.notify();
                })
                .ok();
            }
        }
        if told {
            view.update(cx, |view, cx| {
                if view.notice.as_deref() == Some(WAITING) {
                    view.notice = None;
                    cx.notify();
                }
            })
            .ok();
        }
    }
}

/// Starts the calls of `jack` on `view`. The returned flag is set when they are over (also when the window
/// went away meanwhile).
pub(crate) fn fire(cx: &mut App, view: &Entity<EditorView>, jack: Jack) -> Rc<Cell<bool>> {
    let generation = view.read(cx).document().read(cx).generation();
    fire_for(cx, view, jack, generation)
}

/// [`fire`] for the document with the generation `generation`: a flow that began with a document passes
/// it, so that the event is not given to the one that replaced it while the flow waited.
pub(crate) fn fire_for(
    cx: &mut App,
    view: &Entity<EditorView>,
    jack: Jack,
    generation: u64,
) -> Rc<Cell<bool>> {
    if view.read(cx).document().read(cx).generation() != generation {
        return Rc::new(Cell::new(true));
    }
    // Nothing to call (plugins off, or none declares the jack): over at once, with no task and no wait.
    let calls = collect(cx, jack);
    if calls.is_empty() {
        return Rc::new(Cell::new(true));
    }
    let done = Rc::new(Cell::new(false));
    let weak = view.downgrade();
    let flag = done.clone();
    let ticket = view.update(cx, |view, _| view.take_event_ticket());
    cx.spawn(async move |cx| {
        run_all(&weak, cx, calls, ticket, generation).await;
        flag.set(true);
    })
    .detach();
    done
}

/// The calls of `jack` on `view`, from a flow of the window that has to go on after them (a save, a
/// close). Returns when they are over.
pub(crate) async fn fire_and_wait(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    jack: Jack,
) {
    let generation = view
        .read_with(cx, |view, app| view.document().read(app).generation())
        .ok();
    fire_and_wait_for(view, cx, jack, generation).await;
}

/// [`fire_and_wait`] for the document of the flow (`generation`; `None`: the window is gone).
pub(crate) async fn fire_and_wait_for(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    jack: Jack,
    generation: Option<u64>,
) {
    let (Some(entity), Some(generation)) = (view.upgrade(), generation) else {
        return;
    };
    let Ok(done) = cx.update(|_, app| fire_for(app, &entity, jack, generation)) else {
        return;
    };
    drop(entity);
    while !done.get() {
        cx.background_executor().timer(TICK).await;
    }
}

/// `EditorStart` of a window whose first file was loaded into it first (Sakura loads, and then calls the
/// plugins: `CNormalProcess`). Does nothing if it was called already or is not due.
pub(crate) async fn start_editor_if_due(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) {
    let due = view
        .update(cx, |view, _| view.take_editor_start_due())
        .unwrap_or(false);
    if due {
        fire_and_wait(view, cx, Jack::EditorStart).await;
    }
}

/// What the plugins were told about a close that is not over yet. A flow that asks again, or looks again
/// after the handlers, keeps one log for all its rounds: a view is told `DocumentClose` once and a window
/// `EditorEnd` once, however many times the round starts over; and a flow that ends with the windows still
/// there ([`restart_ended`]) takes the `EditorEnd` back with an `EditorStart`.
#[derive(Default)]
pub(crate) struct CloseLog {
    /// The documents that were told `DocumentClose`: a tab and the generation of the document it shows (a
    /// document that replaces another in the tab is another one).
    closed: HashSet<(EntityId, u64)>,
    /// The windows that were told `EditorEnd`.
    ended: Vec<EndedWindow>,
}

/// A window that was told `EditorEnd`. The window is known by its workspace, not by a tab: the tabs can move
/// and change places while the handlers run.
struct EndedWindow {
    /// The workspace, or the tab when it has none.
    key: EntityId,
    workspace: Option<WeakEntity<Workspace>>,
    /// The tab that the event went to.
    first: WeakEntity<EditorView>,
}

/// The events of a window that began with a Grep (`-GREPMODE`): when the search is over, `EditorStart` (if it
/// is due) and `DocumentOpen` (`CNormalProcess`: `DoGrep`, then `PP_EDITOR_START`, then `PP_DOCUMENT_OPEN`). The
/// search does not block the window as it does in Sakura, so this waits for it.
pub(crate) async fn start_after_grep(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    // The document of the window when the launch began: a file that the user opens meanwhile has its own
    // `DocumentOpen`.
    let generation = view
        .read_with(cx, |view, app| view.document().read(app).generation())
        .ok();
    while view
        .read_with(cx, |view, _| view.grep_busy())
        .unwrap_or(false)
    {
        cx.background_executor().timer(TICK).await;
    }
    start_editor_if_due(view, cx).await;
    fire_and_wait_for(view, cx, Jack::DocumentOpen, generation).await;
}

/// `DocumentClose` for each of `views`, and `EditorEnd` on the first of them: a window is about to go
/// (`MYWM_CLOSE` in `CEditWnd::DispatchEvent`). Returns when they are over, and whether the user changed
/// the text of a tab meanwhile (what the plugins wrote does not count: Sakura closes after the events
/// whatever they did). The caller says [`restart_ended`] if the window does not go after all.
pub(crate) async fn window_closing(
    views: &[WeakEntity<EditorView>],
    log: &mut CloseLog,
    cx: &mut AsyncWindowContext,
) -> bool {
    windows_closing(&[views.to_vec()], log, cx).await
}

/// [`window_closing`] for several windows that go together (quitting): every tab is watched from before the
/// first handler runs until after the last one, so that an edit in a window that was done already is seen.
pub(crate) async fn windows_closing(
    windows: &[Vec<WeakEntity<EditorView>>],
    log: &mut CloseLog,
    cx: &mut AsyncWindowContext,
) -> bool {
    for view in windows.iter().flatten() {
        view.update(cx, |view, cx| view.begin_edit_watch(cx)).ok();
    }
    for views in windows {
        for view in views {
            let generation = view
                .read_with(cx, |view, app| view.document().read(app).generation())
                .ok();
            // Told already by an earlier round of this flow: not told twice.
            if let Some(generation) = generation
                && log.closed.insert((view.entity_id(), generation))
            {
                fire_and_wait_for(view, cx, Jack::DocumentClose, Some(generation)).await;
            }
        }
        if let Some(first) = views.first() {
            let workspace = first
                .read_with(cx, |view, _| view.split_host.clone())
                .ok()
                .flatten();
            let key = workspace
                .as_ref()
                .map_or_else(|| first.entity_id(), |workspace| workspace.entity_id());
            if !log.ended.iter().any(|ended| ended.key == key) {
                log.ended.push(EndedWindow {
                    key,
                    workspace,
                    first: first.clone(),
                });
                fire_and_wait(first, cx, Jack::EditorEnd).await;
            }
        }
    }
    let mut changed = false;
    for view in windows.iter().flatten() {
        changed |= view
            .update(cx, |view, cx| view.end_edit_watch(cx))
            .unwrap_or(false);
    }
    changed
}

/// The flow gave up, or the window stayed: every window that was told `EditorEnd` and is still there is
/// told `EditorStart` again, so that the plugins see the editor that goes on (a tab opened meanwhile, text
/// typed meanwhile, a window that opened). The log starts over.
pub(crate) async fn restart_ended(log: &mut CloseLog, cx: &mut AsyncWindowContext) {
    log.closed.clear();
    for ended in std::mem::take(&mut log.ended) {
        // The tab that shows now, if the tab of the event went away or to another window.
        let shown = ended
            .workspace
            .and_then(|workspace| {
                cx.update(|_, app| Some(workspace.upgrade()?.read(app).editor().downgrade()))
                    .ok()
                    .flatten()
            })
            .unwrap_or(ended.first);
        fire_and_wait(&shown, cx, Jack::EditorStart).await;
    }
}

#[cfg(test)]
mod close_path_tests;
#[cfg(test)]
mod tests;
