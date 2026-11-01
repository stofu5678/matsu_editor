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

/// Waits until this window may start an event call. `false`: it may not, because a command of a plugin or
/// a macro has the window (the event is not given), or the window is gone.
async fn wait_for_turn(view: &WeakEntity<EditorView>, cx: &mut AsyncApp) -> bool {
    let mut waited = Duration::ZERO;
    loop {
        match view.read_with(cx, |view, app| view.event_turn(app)) {
            Err(_) | Ok(Turn::Taken) => return false,
            Ok(Turn::Free) => return true,
            // Another event: it ends within its limit, and then this one is next.
            Ok(Turn::AfterEvent) if waited < TURN_LIMIT => {}
            Ok(Turn::AfterEvent) => return false,
        }
        cx.background_executor().timer(TICK).await;
        waited += TICK;
    }
}

/// Runs every call of `jack` on `view`, one after the other, and waits for each.
async fn run_all(view: &WeakEntity<EditorView>, cx: &mut AsyncApp, jack: Jack) {
    if !wait_for_turn(view, cx).await {
        return;
    }
    let calls = cx.update(|app| collect(app, jack));
    for call in calls {
        let Some(entity) = view.upgrade() else {
            return;
        };
        if !wait_for_turn(view, cx).await {
            return;
        }
        if !cx.update(|app| plugin_call::run(entity, call, app)) {
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
    let done = Rc::new(Cell::new(false));
    let weak = view.downgrade();
    let flag = done.clone();
    cx.spawn(async move |cx| {
        run_all(&weak, cx, jack).await;
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
/// (`MYWM_CLOSE` in `CEditWnd::DispatchEvent`). Returns when they are over.
pub(crate) async fn window_closing(views: &[WeakEntity<EditorView>], cx: &mut AsyncWindowContext) {
    for view in views {
        fire_and_wait(view, cx, Jack::DocumentClose).await;
    }
    if let Some(first) = views.first() {
        fire_and_wait(first, cx, Jack::EditorEnd).await;
    }
}

#[cfg(test)]
mod tests;
