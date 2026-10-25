//! The preview tab (ADR-0024): the first tab that is not an editor. It shows a document as
//! Markdown or HTML, next to the tab of its source, and follows the source while it is edited.
//!
//! * [`text`]: which format a file is, how much of the document is read (never the whole of a huge
//!   one), and which images may be loaded.
//! * [`view`]: the view, with a header line and the rendered text (gpui-component's text view, which
//!   parses on a background thread).
//! * [`menus`]: the menus of the window while a preview is active (`MenuKind::Preview`).
//!
//! The tab is opened by `F_PREVIEW_TAB` (MATSU Editor's own function, `ui_commands/window/preview_tab.rs`),
//! and the window opens, closes and returns from it in `tabs/preview_tab.rs`.
//!
//! The tab keeps a view of the document that it never draws. The window asks its active tab for a
//! view in many places (the title, the commands, the file watch); with that view they keep working
//! on the document that the preview shows. The view is not one of the window's editors: the tab
//! lists, the search for an open file and the questions before closing do not see it.

pub(crate) mod menus;
pub(crate) mod text;
mod view;

use gpui::{App, Entity, FocusHandle, actions};

use crate::panes::PaneSet;
use crate::status::StatusInfo;
use crate::tabs::content::{CloseCheck, MenuKind, TabPanel};
pub(crate) use view::PreviewView;

actions!(
    matsu_preview,
    [
        /// Reads the document again.
        Refresh,
        /// Goes to the tab of the source (opening it again if it was closed).
        ToSource,
        /// Closes the preview.
        Close,
        /// Shows the document as Markdown.
        AsMarkdown,
        /// Shows the document as HTML.
        AsHtml
    ]
);

/// The keys of the preview. Its key context is "Preview".
pub(crate) fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("f5", Refresh, Some("Preview")),
        gpui::KeyBinding::new("escape", ToSource, Some("Preview")),
        gpui::KeyBinding::new("secondary-w", Close, Some("Preview")),
    ]);
}

/// A preview tab.
pub(crate) struct PreviewTab {
    /// A view of the document, never drawn (see the module).
    panes: PaneSet,
    view: Entity<PreviewView>,
}

impl PreviewTab {
    pub(crate) fn new(panes: PaneSet, view: Entity<PreviewView>) -> Self {
        Self { panes, view }
    }

    pub(crate) fn panes(&self) -> &PaneSet {
        &self.panes
    }

    pub(crate) fn panes_mut(&mut self) -> &mut PaneSet {
        &mut self.panes
    }

    pub(crate) fn view(&self) -> &Entity<PreviewView> {
        &self.view
    }
}

impl TabPanel for PreviewTab {
    fn caption(&self, cx: &App) -> String {
        format!("プレビュー: {}", self.view.read(cx).document_name(cx))
    }

    fn tooltip(&self, cx: &App) -> String {
        match self.view.read(cx).document().read(cx).path() {
            Some(path) => format!("プレビュー: {}", path.display()),
            None => self.caption(cx),
        }
    }

    fn accessible_name(&self, cx: &App) -> String {
        self.caption(cx)
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        gpui::Focusable::focus_handle(self.view.read(cx), cx)
    }

    fn menu_kind(&self) -> MenuKind {
        MenuKind::Preview
    }

    fn status(&self, _: &App) -> Option<StatusInfo> {
        None
    }

    /// A preview changes nothing, so closing it asks nothing.
    fn close_check(&self, _: &App) -> CloseCheck {
        CloseCheck::None
    }
}
