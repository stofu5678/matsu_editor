//! The AI log tab (ADR-0030 decision 6): a read-only tab that lists what was sent to a generative AI and what came
//! back, one record per run, the newest last. It is how "a shell runs behind the scenes" is made visible: the
//! command line of a running one-liner is there with 「実行中」, and it can be selected and copied into a shell tab.
//!
//! * [`records`]: the records as plain data (the limits, the cutting, the text shown). Memory only.
//! * `view`: the view, the records as text in gpui-component's text view (selection and copy).
//! * [`menus`]: the menus of the window while the log is active (`MenuKind::AiLog`).
//!
//! The window has at most one log tab (`tabs/ai_log_tab.rs`). The records belong to the tab: they are never written
//! to a file, they are not in the autosave of unsaved documents (the tab is not an editor, so the keeper does not
//! see it), and they are gone when the tab closes.
//!
//! The API for the one-liner flow (AI-4 of ADR-0030): [`begin`] opens the tab if it is not open, without taking the
//! focus from the document, and adds a running record; [`RecordHandle::finish`] ends it.
//!
//! Like the preview and the shell, the tab keeps a view of an empty document that it never draws, so that the
//! window always has a view to ask.

pub(crate) mod menus;
pub(crate) mod records;
mod view;

use gpui::{App, Context, Entity, FocusHandle, WeakEntity, Window, actions};

use crate::app::Workspace;
use crate::panes::PaneSet;
use crate::status::StatusInfo;
use crate::tabs::content::{CloseCheck, MenuKind, TabPanel};
pub(crate) use records::{Finished, Started};
pub(crate) use view::AiLogView;

actions!(
    matsu_ai_log,
    [
        /// Copies the selected text.
        Copy,
        /// Selects all the text of the log.
        SelectAll,
        /// Closes the log tab.
        Close
    ]
);

/// The keys of the log. Its key context is "AiLog".
pub(crate) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("secondary-c", Copy, Some("AiLog")),
        gpui::KeyBinding::new("secondary-a", SelectAll, Some("AiLog")),
        gpui::KeyBinding::new("secondary-w", Close, Some("AiLog")),
    ]);
}

/// The tab's caption.
const CAPTION: &str = "AI ログ";

/// The AI log tab.
pub(crate) struct AiLogTab {
    /// A view of an empty document, never drawn (see the module).
    panes: PaneSet,
    view: Entity<AiLogView>,
}

impl AiLogTab {
    pub(crate) fn new(panes: PaneSet, view: Entity<AiLogView>) -> Self {
        Self { panes, view }
    }

    pub(crate) fn panes(&self) -> &PaneSet {
        &self.panes
    }

    pub(crate) fn panes_mut(&mut self) -> &mut PaneSet {
        &mut self.panes
    }

    pub(crate) fn view(&self) -> &Entity<AiLogView> {
        &self.view
    }
}

impl TabPanel for AiLogTab {
    fn caption(&self, _: &App) -> String {
        CAPTION.to_owned()
    }

    fn tooltip(&self, _: &App) -> String {
        format!("{CAPTION}: 生成 AI に送った文と答え（メモリだけに持ち、閉じると消えます）")
    }

    fn icon(&self, _: &App) -> crate::tabs::icon::TabIcon {
        crate::tabs::icon::TabIcon::AiLog
    }

    fn accessible_name(&self, cx: &App) -> String {
        self.caption(cx)
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        gpui::Focusable::focus_handle(self.view.read(cx), cx)
    }

    fn menu_kind(&self) -> MenuKind {
        MenuKind::AiLog
    }

    fn status(&self, _: &App) -> Option<StatusInfo> {
        None
    }

    /// The log changes no document, so closing it asks nothing. A run that is still going is not stopped by it;
    /// its end is dropped.
    fn close_check(&self, _: &App) -> CloseCheck {
        CloseCheck::None
    }
}

/// A record that was added by [`begin`], for ending it. It does not keep the tab alive: when the tab was closed
/// meanwhile, [`finish`](Self::finish) does nothing.
#[derive(Clone)]
pub(crate) struct RecordHandle {
    view: WeakEntity<AiLogView>,
    id: records::RecordId,
}

impl RecordHandle {
    /// Ends the record with the answer, the stderr, the exit code and the time it took.
    pub(crate) fn finish(&self, finished: Finished, cx: &mut App) {
        self.view
            .update(cx, |view, cx| view.finish(self.id, finished, cx))
            .ok();
    }
}

/// Adds a running record to the log of `workspace`'s window, opening the log tab next to the active one if the
/// window has none. The tab does not take the focus: the document the user is working in keeps it.
pub(crate) fn begin(
    workspace: &mut Workspace,
    started: Started,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> RecordHandle {
    let view = workspace.ai_log_view(false, window, cx);
    let id = view.update(cx, |view, cx| view.begin(started, cx));
    RecordHandle {
        view: view.downgrade(),
        id,
    }
}

/// Shows the log: goes to the tab if the window has one, opens it otherwise.
pub(crate) fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    workspace.ai_log_view(true, window, cx);
}
