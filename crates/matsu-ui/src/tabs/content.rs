//! What a tab holds (ADR-0024): today the panes of an editor, and the contract that every kind of
//! tab will have to keep.
//!
//! [`TabContent`] is what a parked tab keeps. It has one kind, the editor ([`PaneSet`]); a kind
//! that is not an editor (the preview, a table, a terminal) is a new variant that implements
//! [`TabPanel`]. The active tab's content is `Workspace::content`, a `TabContent` too.
//!
//! [`TabPanel`] is what the window asks of a tab without knowing its kind: the texts of the tab
//! bar, the focus, which menus to show, what the status bar says and whether closing needs a
//! question. The window already asks all of them but the last (the tab bar, the focus, the
//! menu state, the status bar); the close flow (`file_ops::close`) still asks the document and
//! moves to `close_check` when a kind has no document.

use gpui::{App, FocusHandle, Focusable as _};

use crate::panes::PaneSet;
use crate::status::StatusInfo;

/// Which menus a tab shows (ADR-0024: the menus follow the kind of the active tab). Only the
/// editor's, Sakura's main menu, exists today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MenuKind {
    Editor,
    /// The preview tab: its own small menus (`crate::preview::menus`).
    Preview,
    /// A second kind for the tests of the menu switching: one menu with one item.
    #[cfg(test)]
    Probe,
}

impl MenuKind {
    /// Every kind, for making the menus of all of them.
    pub(crate) const ALL: &'static [MenuKind] = &[
        MenuKind::Editor,
        MenuKind::Preview,
        #[cfg(test)]
        MenuKind::Probe,
    ];
}

/// What closing a tab needs to ask first. Not asked through the contract yet: the close flow
/// (`file_ops::close`) asks the document directly, and moves here when a kind has no document.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CloseCheck {
    /// Nothing to ask: the tab can go.
    None,
    /// The tab has changes that would be lost; the question is "save changes?".
    UnsavedChanges,
}

/// The contract of a tab, whatever it holds.
pub(crate) trait TabPanel {
    /// The text of the tab in the bar (`CTabWnd::GetTabName`).
    fn caption(&self, cx: &App) -> String;
    /// The tooltip of the tab.
    fn tooltip(&self, cx: &App) -> String;
    /// The name an assistive technology reads for the tab.
    fn accessible_name(&self, cx: &App) -> String;
    /// Where the focus goes when the tab becomes active.
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    /// Which menus the window shows while this tab is active.
    fn menu_kind(&self) -> MenuKind;
    /// What the status bar shows, or `None` when the tab has no status bar.
    fn status(&self, cx: &App) -> Option<StatusInfo>;
    /// What closing the tab has to ask first.
    #[allow(dead_code)]
    fn close_check(&self, cx: &App) -> CloseCheck;
}

/// What a tab keeps: the active one in `Workspace::content`, the others in `Tabs`.
pub(crate) enum TabContent {
    Editor(PaneSet),
    /// The preview of a document (ADR-0024). It keeps a view of the document out of sight, so that
    /// the window always has a view to ask (`panes`); it is not one of the window's editors
    /// (`editor_panes`).
    Preview(crate::preview::PreviewTab),
}

impl TabContent {
    /// The panes the window asks of its active tab: an editor's, or the hidden view of a preview's
    /// document.
    pub(crate) fn panes(&self) -> &PaneSet {
        match self {
            Self::Editor(panes) => panes,
            Self::Preview(preview) => preview.panes(),
        }
    }

    pub(crate) fn panes_mut(&mut self) -> &mut PaneSet {
        match self {
            Self::Editor(panes) => panes,
            Self::Preview(preview) => preview.panes_mut(),
        }
    }

    /// The panes of the editor this tab is, or `None` for a tab of another kind.
    pub(crate) fn editor_panes(&self) -> Option<&PaneSet> {
        match self {
            Self::Editor(panes) => Some(panes),
            Self::Preview(_) => None,
        }
    }

    /// The panes of the editor this tab is, taken out of the tab.
    pub(crate) fn into_editor(self) -> Option<PaneSet> {
        match self {
            Self::Editor(panes) => Some(panes),
            Self::Preview(_) => None,
        }
    }

    /// The preview this tab is, or `None`.
    pub(crate) fn preview(&self) -> Option<&crate::preview::PreviewTab> {
        match self {
            Self::Preview(preview) => Some(preview),
            Self::Editor(_) => None,
        }
    }

    pub(crate) fn panel(&self) -> &dyn TabPanel {
        match self {
            Self::Editor(panes) => panes,
            Self::Preview(preview) => preview,
        }
    }
}

impl TabPanel for PaneSet {
    fn caption(&self, cx: &App) -> String {
        crate::app::Workspace::caption_of(self.active().read(cx), cx)
    }

    fn tooltip(&self, cx: &App) -> String {
        crate::app::Workspace::tooltip_of(self.active().read(cx), cx)
    }

    fn accessible_name(&self, cx: &App) -> String {
        self.caption(cx)
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.active().focus_handle(cx)
    }

    fn menu_kind(&self) -> MenuKind {
        MenuKind::Editor
    }

    fn status(&self, cx: &App) -> Option<StatusInfo> {
        Some(self.active().read(cx).status(cx))
    }

    fn close_check(&self, cx: &App) -> CloseCheck {
        // The views of one tab share the document, so the active pane speaks for all of them.
        let shared = self.active().read(cx).document().read(cx);
        if shared.model().is_modified() || shared.format_changed() {
            CloseCheck::UnsavedChanges
        } else {
            CloseCheck::None
        }
    }
}
