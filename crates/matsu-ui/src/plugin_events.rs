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

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, AsyncApp, AsyncWindowContext, Entity, WeakEntity};
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginSettings, PluginState};

use crate::macros::plugin_call::{self, Prepared, Turn};
use crate::view::EditorView;

/// How long an event waits for the event before it, at most.
const TURN_LIMIT: Duration = Duration::from_secs(12);

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
        if plugin_call::run(entity, call, app) {
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
        let mut waited = Duration::ZERO;
        while let Some(call) = pending.take() {
            let (step, back) = try_start(view, cx, call, generation);
            match step {
                Step::Started => started = true,
                Step::Skipped => break,
                // Another event or window has the turn; it ends within its limit.
                Step::Wait if waited < TURN_LIMIT => {
                    pending = back;
                    cx.background_executor().timer(TICK).await;
                    waited += TICK;
                }
                Step::Wait => break,
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
    // Nothing to call (plugins off, or none declares the jack): over at once, with no task and no wait.
    let calls = collect(cx, jack);
    if calls.is_empty() {
        return Rc::new(Cell::new(true));
    }
    let done = Rc::new(Cell::new(false));
    let weak = view.downgrade();
    let flag = done.clone();
    let ticket = view.update(cx, |view, _| view.take_event_ticket());
    let generation = view.read(cx).document().read(cx).generation();
    cx.spawn(async move |cx| {
        run_all(&weak, cx, calls, ticket, generation).await;
        flag.set(true);
    })
    .detach();
    done
}

/// Starts the calls of `jack` on `view`, and runs `then` when they are over.
pub(crate) fn fire_then(
    cx: &mut App,
    view: &Entity<EditorView>,
    jack: Jack,
    then: impl FnOnce(&mut App) + 'static,
) {
    let done = fire(cx, view, jack);
    cx.spawn(async move |cx| {
        while !done.get() {
            cx.background_executor().timer(TICK).await;
        }
        cx.update(then);
    })
    .detach();
}

/// The calls of `jack` on `view`, from a flow of the window that has to go on after them (a save, a
/// close). Returns when they are over.
pub(crate) async fn fire_and_wait(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    jack: Jack,
) {
    let Some(entity) = view.upgrade() else {
        return;
    };
    let Ok(done) = cx.update(|_, app| fire(app, &entity, jack)) else {
        return;
    };
    drop(entity);
    while !done.get() {
        cx.background_executor().timer(TICK).await;
    }
}

/// `DocumentClose` for each of `views`, and `EditorEnd` on the first of them: a window is about to go
/// (`MYWM_CLOSE` in `CEditWnd::DispatchEvent`). Returns when they are over, and whether the user changed
/// the text of a tab meanwhile (what the plugins wrote does not count: Sakura closes after the events
/// whatever they did).
pub(crate) async fn window_closing(
    views: &[WeakEntity<EditorView>],
    cx: &mut AsyncWindowContext,
) -> bool {
    windows_closing(&[views.to_vec()], cx).await
}

/// [`window_closing`] for several windows that go together (quitting): every tab is watched from before the
/// first handler runs until after the last one, so that an edit in a window that was done already is seen.
pub(crate) async fn windows_closing(
    windows: &[Vec<WeakEntity<EditorView>>],
    cx: &mut AsyncWindowContext,
) -> bool {
    for view in windows.iter().flatten() {
        view.update(cx, |view, cx| view.begin_edit_watch(cx)).ok();
    }
    for views in windows {
        for view in views {
            fire_and_wait(view, cx, Jack::DocumentClose).await;
        }
        if let Some(first) = views.first() {
            fire_and_wait(first, cx, Jack::EditorEnd).await;
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

#[cfg(test)]
mod tests;
