//! Watching the files of the open documents, and the rest of the common settings page ファイル.
//!
//! * [`state`]: what a document remembers of its file on disk (the time stamp, the mode of the
//!   watching, whether the file can be written).
//! * [`check`]: the check for a file that another program changed, and the question
//!   (`CAutoReloadAgent`).
//! * [`auto_save`] and [`timer`]: the automatic save (`CAutoSaveAgent`, `CPassiveTimer`).
//! * [`open_checks`]: the warnings before a file is opened (`CLoadAgent::OnCheckLoad`).
//! * [`start`]: the timer of a window that runs the checks (`CEditWnd::OnEditTimer`).

pub(crate) mod auto_save;
pub(crate) mod check;
pub(crate) mod open_checks;
pub(crate) mod state;
pub(crate) mod timer;

use std::time::{Duration, Instant};

use gpui::{Context, Entity, Window};

use crate::app::Workspace;
use crate::file_ops::prefs::prefs;
use crate::view::EditorView;

/// `SetTimer( GetHwnd(), IDT_EDIT, 500, nullptr )`: how often the window looks at the clock.
const TICK: Duration = Duration::from_millis(500);

/// `IncrementTimerCount(6)`: the file is looked at on every sixth tick, that is every 3 seconds.
const TICKS_PER_CHECK: u32 = 6;

/// Starts the timer of a window (`CEditWnd::OnEditTimer`). Each tick: the automatic save, when its
/// interval has passed; every sixth tick, the check for a file that another program changed,
/// while this window is the active one (`CAutoReloadAgent::_ToDoChecking`).
///
/// A check and a save each run on a task of their own, so that a question that waits for the
/// user stops neither the other one nor the other documents. What must not run twice at the same
/// time on one document is marked before the task starts (`DiskState::pausing`).
pub(crate) fn start(window: &mut Window, cx: &mut Context<Workspace>) {
    cx.spawn_in(window, async move |this, cx| {
        let mut ticks = 0u32;
        let mut save_timer = timer::PassiveTimer::default();
        loop {
            cx.background_executor().timer(TICK).await;
            ticks = (ticks + 1) % TICKS_PER_CHECK;
            let alive = this.update_in(cx, |workspace, window, cx| {
                // The check goes first: it marks its document, and the save then leaves it alone.
                if ticks == 0 && window.is_window_active() {
                    spawn_check(workspace.panes.active().clone(), window, cx);
                }
                let prefs = prefs(cx);
                if save_timer.check(Instant::now(), prefs.auto_save, prefs.auto_save_interval) {
                    for editor in workspace.tab_editors() {
                        if auto_save::begin_in(&editor, cx) {
                            let editor = editor.downgrade();
                            cx.spawn_in(window, async move |_, cx| {
                                auto_save::run(&editor, cx).await;
                            })
                            .detach();
                        }
                    }
                }
            });
            if alive.is_err() {
                return;
            }
        }
    })
    .detach();
}

/// Looks at the file of `active` on a task of its own, if it is to be looked at. Also for the
/// window that became the active one: look at the file at once, not at the next sixth tick
/// (`WM_SETFOCUS` sets `m_nTimerCount` to 9).
pub(crate) fn spawn_check(
    active: Entity<EditorView>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let Some((path, stamp)) = check::begin_in(&active, cx) else {
        return;
    };
    let active = active.downgrade();
    cx.spawn_in(window, async move |_, cx| {
        check::run(&active, cx, path, stamp).await;
    })
    .detach();
}

#[cfg(test)]
mod tests;
