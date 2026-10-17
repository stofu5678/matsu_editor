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

use gpui::{AnyWindowHandle, App, AsyncApp, Context, EntityId, Global, WeakEntity, Window};
use ume_commands::command::Arg;

use super::EditorView;
use crate::file_ops::windows::{editors, find_tab_of, is_pending_document};

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

/// The flows that the call of a macro starts in each view, from the call until its wait is over:
/// [`track`] counts them. The work that the user starts meanwhile (the OK of a dialog that the
/// call opened) belongs to the call as well, so the entry stays while the macro waits.
#[derive(Default)]
struct Armed(HashMap<EntityId, Rc<Cell<usize>>>);

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
/// the work ends. Gives `None` when no macro waits for this view, which waits for nothing. The
/// view is the one the macro is bound to: the tab it started in (Sakura: its `CEditView`), wherever
/// that tab is shown.
pub(crate) fn track(cx: &Context<EditorView>) -> Option<FlowGuard> {
    let counter = cx.try_global::<Armed>()?.0.get(&cx.entity_id())?.clone();
    counter.set(counter.get() + 1);
    Some(FlowGuard(counter))
}

/// A message box whose answer nobody needs (the OK of a warning): a macro that waits for the call
/// that showed it waits until the box is closed (Sakura's boxes are modal). Without a macro the
/// answer is dropped, as before.
pub(crate) fn keep_box(
    answer: impl std::future::Future + 'static,
    _window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    let Some(tracked) = track(cx) else {
        return;
    };
    cx.spawn(async move |_, _| {
        answer.await;
        drop(tracked);
    })
    .detach();
}

/// What a call of a macro left open or running.
pub(crate) struct Wait {
    flows: Rc<Cell<usize>>,
    /// What was open or running before the call: that is not the call's.
    baseline: u32,
    /// What the call opened, so far.
    watched: u32,
    /// The documents (tabs) that were open before the call.
    documents: Vec<EntityId>,
    /// Which wait this is; a newer one, or a cancel, makes it stale.
    epoch: u64,
}

/// The call of a macro in `view` is over: its flows are no longer counted (unless a newer call has
/// taken the place).
fn disarm(cx: &mut App, view: EntityId, flows: &Rc<Cell<usize>>) {
    let armed = cx.default_global::<Armed>();
    if armed.0.get(&view).is_some_and(|f| Rc::ptr_eq(f, flows)) {
        armed.0.remove(&view);
    }
}

/// Runs `f` on the view with the window that shows its tab now. A macro belongs to the tab, not to
/// the window it started in: the tab may be moved to another window (`F_TAB_SEPARATE`,
/// `F_TAB_JOINTNEXT`) and the first window may close. `fallback` is the window to use when the
/// view is in no tab (a view that no workspace holds). `None` when the view or the window is gone
/// (the tab was closed): the caller ends its wait.
pub(crate) fn follow<R>(
    this: &WeakEntity<EditorView>,
    fallback: AnyWindowHandle,
    cx: &mut AsyncApp,
    f: impl FnOnce(&mut EditorView, &mut Window, &mut Context<EditorView>) -> R,
) -> Option<R> {
    let entity = this.upgrade()?;
    let handle = cx
        .update(|app| find_tab_of(app, this))
        .map_or(fallback, |(handle, _)| handle);
    handle
        .update(cx, |_, window, app| {
            entity.update(app, |view, cx| f(view, window, cx))
        })
        .ok()
}

impl EditorView {
    /// What is open or running in this window now.
    fn activity(&self, cx: &mut Context<Self>) -> u32 {
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
            (is_pending_document(cx, self.document.entity_id()), LOADING),
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
        let me = cx.entity_id();
        cx.default_global::<Armed>().0.insert(me, flows.clone());
        let mut wait = Wait {
            flows,
            baseline: self.activity(cx),
            watched: 0,
            documents: self.open_tabs(cx).into_iter().map(|(d, _, _)| d).collect(),
            epoch: 0,
        };
        let ok = self.run_function(code, args, window, cx);
        let waits = ok && !self.is_settled(&mut wait, cx);
        if !waits {
            disarm(cx, me, &wait.flows);
        }
        (ok, waits.then_some(wait))
    }

    /// The open tabs: the document of each, whether it is still being read into, and whether a
    /// Grep runs in it. This view stands for its own tab (it may be borrowed, so it is not read).
    fn open_tabs(&self, cx: &mut Context<Self>) -> Vec<(EntityId, bool, bool)> {
        let me = cx.entity_id();
        let mut tabs = vec![(
            self.document.entity_id(),
            is_pending_document(cx, self.document.entity_id()),
            self.grep.is_running(),
        )];
        for (_, view) in editors(cx) {
            if view.entity_id() == me {
                continue;
            }
            let view = view.read(cx);
            let document = view.document().entity_id();
            if document != tabs[0].0 {
                let grep = view.is_grep_running();
                tabs.push((document, is_pending_document(cx, document), grep));
            }
        }
        tabs
    }

    /// Whether everything the call left behind is over.
    fn is_settled(&self, wait: &mut Wait, cx: &mut Context<Self>) -> bool {
        if wait.flows.get() > 0 {
            return false;
        }
        let now = self.activity(cx);
        wait.watched |= now & !wait.baseline;
        if wait.watched & now != 0 {
            return false;
        }
        // The tabs (windows) that the call opened: a file that is read into one, a Grep that runs
        // in one.
        self.open_tabs(cx)
            .into_iter()
            .all(|(document, pending, grep)| {
                wait.documents.contains(&document) || !(pending || grep)
            })
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
        let fallback = window.window_handle();
        cx.spawn(async move |this, cx| {
            let mut done = Some(done);
            loop {
                cx.background_executor().timer(POLL).await;
                let finished = follow(&this, fallback, cx, |this, window, cx| {
                    let me = cx.entity_id();
                    // Stopped, or another wait began: nothing to do.
                    if this.macros.waiting != Some(wait.epoch) {
                        disarm(cx, me, &wait.flows);
                        return true;
                    }
                    if !this.is_settled(&mut wait, cx) {
                        return false;
                    }
                    this.macros.waiting = None;
                    disarm(cx, me, &wait.flows);
                    if let Some(done) = done.take() {
                        done(this, window, cx);
                    }
                    true
                });
                // `None`: the tab was closed, and the macro with it.
                if finished.unwrap_or(true) {
                    break;
                }
            }
        })
        .detach();
    }
}
