//! What a view gives to and takes from the other panes of its window (`crate::panes`).
//!
//! Sakura's `CEditView::CopyViewStatus` (`sakura_core/view/CEditView.cpp`) copies the view of one
//! pane to another when a split starts or ends: the caret, the selection and the scroll position.
//! A new pane also starts with the settings of the pane it comes from, and the few things that
//! belong to the window rather than to a pane (the search pattern, the key macro) follow the
//! active pane.

use gpui::{Context, Entity, WeakEntity};
use ume_commands::keymap::KeyMap;
use ume_core::cursor::Cursor;

use super::{EditorView, ViewPrefs};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::geometry::Metrics;
use crate::macros::MacroState;
use crate::search::SearchState;

/// The caret, the selection and the scroll position of a view (`CopyViewStatus`).
#[derive(Clone, Debug)]
pub(crate) struct ViewStatus {
    cursor: Cursor,
    /// The left column and the top row, not pixels: panes can have different font sizes.
    left_col: f32,
    top_row: f32,
    scroll_past_end: bool,
}

/// What a new pane takes from the pane it is made from.
pub(crate) struct PaneSeed {
    status: ViewStatus,
    keymap: KeyMap,
    prefs: ViewPrefs,
    font_family: gpui::SharedString,
    font_size: f32,
    metrics: Metrics,
    search: SearchState,
}

/// What belongs to the window and so goes to the pane the user works in.
pub(crate) struct WindowState {
    search: SearchState,
    macros: Option<MacroState>,
    show_status_bar: bool,
}

impl EditorView {
    /// A pane of `document` that starts like `seed`'s pane.
    pub(crate) fn new_pane(
        document: Entity<SharedDocument>,
        seed: PaneSeed,
        host: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self::new(document, cx);
        view.keymap = seed.keymap;
        view.prefs = seed.prefs;
        view.font_family = seed.font_family;
        view.font_size = seed.font_size;
        view.metrics = seed.metrics;
        view.search = seed.search;
        view.split_host = Some(host);
        view.relayout_in(cx);
        view.apply_view_status(seed.status);
        view
    }

    pub(crate) fn pane_seed(&self) -> PaneSeed {
        PaneSeed {
            status: self.view_status(),
            keymap: self.keymap.clone(),
            prefs: self.prefs.clone(),
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            metrics: self.metrics,
            search: self.search.clone(),
        }
    }

    pub(crate) fn view_status(&self) -> ViewStatus {
        ViewStatus {
            cursor: self.cursor.clone(),
            left_col: self.scroll_x / self.metrics.cell_width,
            top_row: self.scroll_y / self.metrics.line_height,
            scroll_past_end: self.scroll_past_end,
        }
    }

    /// Takes the caret, the selection and the scroll position of another pane.
    pub(crate) fn apply_view_status(&mut self, status: ViewStatus) {
        self.cursor = status.cursor;
        self.scroll_x = status.left_col * self.metrics.cell_width;
        self.scroll_y = status.top_row * self.metrics.line_height;
        self.scroll_past_end = status.scroll_past_end;
        self.marked = None;
        self.drag = super::Drag::None;
        self.clamp_scroll();
    }

    /// The scroll position, for keeping panes in step.
    pub(crate) fn scroll(&self) -> (f32, f32) {
        (self.scroll_x, self.scroll_y)
    }

    /// Moves the view to a scroll position that another pane has (synchronized scrolling).
    pub(crate) fn set_scroll(&mut self, x: Option<f32>, y: Option<(f32, bool)>) {
        if let Some(x) = x {
            self.scroll_x = x;
        }
        if let Some((y, past_end)) = y {
            self.scroll_y = y;
            self.scroll_past_end = past_end;
        }
        self.clamp_scroll();
    }

    pub(crate) fn scroll_past_end(&self) -> bool {
        self.scroll_past_end
    }

    /// Takes what belongs to the window from the pane the user leaves.
    pub(crate) fn take_window_state(&mut self, with_macros: bool) -> WindowState {
        WindowState {
            search: self.search.clone(),
            macros: with_macros.then(|| std::mem::take(&mut self.macros)),
            show_status_bar: self.prefs.show_status_bar,
        }
    }

    /// Gives the pane the user comes to what belongs to the window.
    pub(crate) fn put_window_state(&mut self, state: WindowState) {
        self.search = state.search;
        if let Some(macros) = state.macros {
            self.macros = macros;
        }
        self.prefs.show_status_bar = state.show_status_bar;
    }
}

impl EditorView {
    /// Splits the window the way `kind` says. The window changes the panes, one of which is this
    /// view, so it does so after the command ends.
    pub(crate) fn request_split(
        &mut self,
        kind: crate::panes::SplitKind,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> ume_commands::command::Result {
        let Some(host) = self.split_host.clone() else {
            return Err(ume_commands::command::CommandError::Unavailable(
                "the view has no window to split",
            ));
        };
        window.defer(cx, move |window, cx| {
            host.update(cx, |workspace, cx| workspace.split(kind, window, cx))
                .ok();
        });
        Ok(())
    }
}
