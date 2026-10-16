//! A macro waits for a UI command that finishes later.
//!
//! A UI command that opens a dialog or a panel, or that reads a file in the background, returns
//! at once: the work goes on in the window. Sakura's macro runs on the window's own thread, and
//! its dialogs are modal, so the next call of the macro runs after the dialog is closed
//! (`CMacro::HandleCommand` returns when `CViewCommander::HandleCommand` does). This module makes
//! a macro wait the same way: a call of a macro that leaves something open or running is
//! answered (a script) or followed by the next step (a key macro) when that is over.
//!
//! What a call leaves behind is found out, not declared by each command:
//!
//! * flows: the async flows that the call started with `file_ops::start` (an open dialog, a
//!   question before closing, a read of a file) or that a command marked with [`track`];
//! * panels and dialogs that were not open before the call and are open now (Grep, the type
//!   list, the window list, DIFF, compare, the print preview, the favorites dialog and so on);
//! * work in the window: a Grep that runs, a file that is read into it;
//! * the other windows that the call opened: a file that is read into one, a Grep that runs in
//!   one.
//!
//! The wait ends when none is left. How it ends (OK or cancel, loaded or failed) does not change
//! the answer: `CViewCommander::HandleCommand` returns true for a dialog that was cancelled, and
//! a file that cannot be read is told to the user in a box. Only a call that fails at once fails.
//!
//! # Differences from Sakura (decided by the user, 2026-10-17)
//!
//! * Sakura starts another editor process for a new window and does not wait for it. Here the
//!   macro waits until the new window has finished loading its file or has finished its Grep.
//! * Sakura's macro cannot be stopped. Here the cancel button, closing the window and the end of
//!   the script end the wait: nothing is answered after that, and the next macro can start.
//!
//! The macro stays bound to the window that started it, as Sakura's macro runs on its
//! `CEditView`: the polling task belongs to that window's view, and the calls that follow go to
//! it, whichever window the command opened or activated.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{AnyWindowHandle, App, Context, Global, Window};
use ume_commands::command::Arg;

use super::EditorView;
use crate::file_ops::windows::{editors, has_pending};

/// How often a waiting macro looks again. A change of the windows has no event to wait for.
const POLL: Duration = Duration::from_millis(25);

// What is open or running in a window, one bit each.
const SEARCH_PANEL: u32 = 1 << 0;
const GREP_PANEL: u32 = 1 << 1;
const SAVE_AS_DIALOG: u32 = 1 << 2;
const WINDOW_LIST: u32 = 1 << 3;
const COMMAND_LIST: u32 = 1 << 4;
const ABOUT_DIALOG: u32 = 1 << 5;
const DIFF_PANEL: u32 = 1 << 6;
const COMPARE_PANEL: u32 = 1 << 7;
const TYPE_LIST: u32 = 1 << 8;
const PRINT_PREVIEW: u32 = 1 << 9;
const FAVORITE_DIALOG: u32 = 1 << 10;
const GREP_RUNNING: u32 = 1 << 11;
const LOADING: u32 = 1 << 12;
const PRINTING: u32 = 1 << 13;

/// The flows that the call of a macro starts in each window, from the call until its wait is over:
/// [`track`] counts them. The work that the user starts meanwhile (the OK of a dialog that the
/// call opened) belongs to the call as well, so the entry stays while the macro waits.
#[derive(Default)]
struct Armed(HashMap<AnyWindowHandle, Rc<Cell<usize>>>);

impl Global for Armed {}

/// Held by a flow while it runs. Dropping it (the flow ended, or its window closed) ends the
/// flow's part of the wait.
pub(crate) struct FlowGuard(Rc<Cell<usize>>);

impl Drop for FlowGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// Call this where a UI command starts work that outlives the call, and hold the result until
/// the work ends. Gives `None` when no macro waits for `window`, which waits for nothing.
pub(crate) fn track(cx: &App, window: &Window) -> Option<FlowGuard> {
    let counter = cx
        .try_global::<Armed>()?
        .0
        .get(&window.window_handle())?
        .clone();
    counter.set(counter.get() + 1);
    Some(FlowGuard(counter))
}

/// What a call of a macro left open or running.
pub(crate) struct Wait {
    flows: Rc<Cell<usize>>,
    /// What was open or running before the call: that is not the call's.
    baseline: u32,
    /// What the call opened, so far.
    watched: u32,
    /// The editor windows that were there before the call.
    windows: Vec<AnyWindowHandle>,
    /// Which wait this is; a newer one, or a cancel, makes it stale.
    epoch: u64,
}

/// The call of a macro in `window` is over: its flows are no longer counted (unless a newer call
/// has taken the place).
fn disarm(cx: &mut App, window: AnyWindowHandle, flows: &Rc<Cell<usize>>) {
    let armed = cx.default_global::<Armed>();
    if armed.0.get(&window).is_some_and(|f| Rc::ptr_eq(f, flows)) {
        armed.0.remove(&window);
    }
}

impl EditorView {
    /// What is open or running in this window now.
    fn activity(&self, window: &Window, cx: &mut Context<Self>) -> u32 {
        let flags = [
            (self.panel.is_some(), SEARCH_PANEL),
            (self.grep.panel.is_some(), GREP_PANEL),
            (self.save_as_dialog.is_some(), SAVE_AS_DIALOG),
            (self.window_list.is_some(), WINDOW_LIST),
            (self.support.command_list.is_some(), COMMAND_LIST),
            (self.support.about.is_some(), ABOUT_DIALOG),
            (self.diff.panel.is_some(), DIFF_PANEL),
            (self.diff.compare.is_some(), COMPARE_PANEL),
            (self.doc_type.list.is_some(), TYPE_LIST),
            (self.print.preview.is_some(), PRINT_PREVIEW),
            (crate::history::dialog::is_open(cx), FAVORITE_DIALOG),
            (self.grep.is_running(), GREP_RUNNING),
            (has_pending(cx, window.window_handle()), LOADING),
            (self.is_printing(), PRINTING),
        ];
        flags
            .into_iter()
            .filter(|(open, _)| *open)
            .fold(0, |bits, (_, bit)| bits | bit)
    }

    /// Runs Sakura's function `code` for a macro (a key macro's step or a script's call), and
    /// says what it left behind to wait for, if anything. The result is false when the call
    /// failed, and then nothing is waited for.
    pub(super) fn run_macro_function(
        &mut self,
        code: u16,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (bool, Option<Wait>) {
        let flows = Rc::new(Cell::new(0));
        let me = window.window_handle();
        cx.default_global::<Armed>().0.insert(me, flows.clone());
        let mut wait = Wait {
            flows,
            baseline: self.activity(window, cx),
            watched: 0,
            windows: editors(cx).into_iter().map(|(h, _)| h).collect(),
            epoch: 0,
        };
        wait.windows.push(me);
        let ok = self.run_function(code, args, window, cx);
        let waits = ok && !self.is_settled(&mut wait, window, cx);
        if !waits {
            disarm(cx, me, &wait.flows);
        }
        (ok, waits.then_some(wait))
    }

    /// Whether everything the call left behind is over.
    fn is_settled(&self, wait: &mut Wait, window: &Window, cx: &mut Context<Self>) -> bool {
        if wait.flows.get() > 0 {
            return false;
        }
        let now = self.activity(window, cx);
        wait.watched |= now & !wait.baseline;
        if wait.watched & now != 0 {
            return false;
        }
        // The windows that the call opened: a file that is read into one, a Grep that runs in one.
        for (handle, view) in editors(cx) {
            if wait.windows.contains(&handle) {
                continue;
            }
            if has_pending(cx, handle) || view.read(cx).is_grep_running() {
                return false;
            }
        }
        true
    }

    /// Waits for `wait` in this window, and then runs `done` (not when the macro was stopped
    /// meanwhile). The clock of a script stays held by the caller.
    pub(super) fn begin_wait(
        &mut self,
        mut wait: Wait,
        window: &mut Window,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        self.macros.epoch += 1;
        wait.epoch = self.macros.epoch;
        self.macros.waiting = Some(wait.epoch);
        cx.spawn_in(window, async move |this, cx| {
            let mut done = Some(done);
            loop {
                cx.background_executor().timer(POLL).await;
                let finished = this.update_in(cx, |this, window, cx| {
                    // Stopped, or another wait began: nothing to do.
                    if this.macros.waiting != Some(wait.epoch) {
                        disarm(cx, window.window_handle(), &wait.flows);
                        return true;
                    }
                    if !this.is_settled(&mut wait, window, cx) {
                        return false;
                    }
                    this.macros.waiting = None;
                    disarm(cx, window.window_handle(), &wait.flows);
                    if let Some(done) = done.take() {
                        done(this, window, cx);
                    }
                    true
                });
                // An error is a window that is gone.
                if finished.unwrap_or(true) {
                    break;
                }
            }
        })
        .detach();
    }
}
