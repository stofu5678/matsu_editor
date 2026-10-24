//! What the window does with its preview tab (ADR-0024, `crate::preview`): opens it, returns from
//! it to the source, and closes it.

use gpui::{AppContext as _, Context, Window};

use super::content::TabContent;
use crate::app::Workspace;
use crate::panes::PaneSet;
use crate::preview::{PreviewTab, PreviewView};
use crate::view::EditorView;

impl Workspace {
    /// The tab that previews `document` in this window.
    fn preview_tab_of(
        &self,
        document: &gpui::Entity<crate::document::SharedDocument>,
        cx: &gpui::App,
    ) -> Option<usize> {
        (0..self.tab_count()).find(|&i| {
            self.tab_content(i)
                .and_then(TabContent::preview)
                .is_some_and(|p| p.view().read(cx).document() == document)
        })
    }

    /// `F_PREVIEW_TAB`: shows the document of the active tab in a preview tab next to it, or goes to
    /// the preview it already has. From a preview it does nothing.
    pub(crate) fn open_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.content.editor_panes().map(|p| p.active().clone()) else {
            return;
        };
        let document = editor.read(cx).document().clone();
        if let Some(index) = self.preview_tab_of(&document, cx) {
            self.activate_tab(index, window, cx);
            return;
        }
        // A document over the limit is refused before the tab (and its hidden view, which lays
        // the document out) is made.
        if !crate::preview::text::within_source_limit(document.read(cx).doc()) {
            editor.update(cx, |view, cx| {
                view.notice = Some(format!(
                    "大きいので、プレビューしません（{} MiB までのファイルを表示します）。",
                    crate::preview::text::MAX_SOURCE_BYTES >> 20
                ));
                cx.notify();
            });
            return;
        }
        let host = cx.weak_entity();
        let hidden = cx.new(|cx| EditorView::new(document.clone(), cx));
        // The functions of the bar (next and previous window, the list of the tabs, close) run on
        // this view and reach the window through it.
        hidden.update(cx, |view, _| view.split_host = Some(host.clone()));
        let view = cx.new(|cx| PreviewView::new(document, host, cx));
        let tab = TabContent::Preview(PreviewTab::new(PaneSet::new(hidden), view));
        let at = self.active_tab() + 1;
        self.insert_content(tab, at, true, window, cx);
    }

    /// Goes from the active preview to the tab of its source. If that tab was closed, the document
    /// is opened again in a tab next to the preview.
    pub(crate) fn preview_to_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.active_tab();
        let Some(document) = self
            .tab_content(active)
            .and_then(TabContent::preview)
            .map(|p| p.view().read(cx).document().clone())
        else {
            return;
        };
        // The same document, or the same file: a source that was closed and opened again is a new
        // document of the same file.
        let path = document.read(cx).path().map(std::path::Path::to_path_buf);
        let is_source = |view: &gpui::Entity<EditorView>, cx: &gpui::App| {
            let other = view.read(cx).document();
            other == &document
                || path.as_deref().is_some_and(|path| {
                    other
                        .read(cx)
                        .path()
                        .is_some_and(|p| crate::file_ops::windows::same_file(p, path))
                })
        };
        let source = (0..self.tab_count())
            .find(|&i| self.tab_panes(i).is_some_and(|p| is_source(p.active(), cx)));
        // The source may have moved to another window (a separated tab, another group): go there
        // rather than open the document a second time.
        let me = cx.entity_id();
        let elsewhere = crate::file_ops::windows::listed_workspaces(cx)
            .into_iter()
            .filter(|(_, workspace)| workspace.entity_id() != me)
            .find_map(|(handle, workspace)| {
                let workspace = workspace.read(cx);
                (0..workspace.tab_count())
                    .filter_map(|i| workspace.tab_editor(i))
                    .find(|view| is_source(view, cx))
                    .map(|view| (handle, view))
            });
        match source {
            Some(index) => self.activate_tab(index, window, cx),
            None if elsewhere.is_some() => {
                if let Some((handle, view)) = elsewhere {
                    super::flows::reveal(cx, handle, view);
                }
            }
            None => {
                let user = cx.try_global::<crate::settings::UserSettings>().cloned();
                let editor = cx.new(|cx| EditorView::new(document, cx));
                if let Some(user) = &user {
                    editor.update(cx, |view, cx| view.apply_user_settings(user, cx));
                }
                self.insert_tab(PaneSet::new(editor), active + 1, true, window, cx);
            }
        }
    }

    /// Where the focus goes for the active tab.
    pub(crate) fn active_focus(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.content.panel().focus_handle(cx)
    }

    /// Whether the active tab is a preview.
    pub(crate) fn active_is_preview(&self) -> bool {
        self.content.preview().is_some()
    }

    /// Closes the active tab if it is a preview.
    pub(crate) fn close_active_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.active_tab();
        if self
            .tab_content(active)
            .is_some_and(|c| c.preview().is_some())
        {
            self.close_tab_or_window(active, window, cx);
        }
    }

    /// Closes tab `index` without a question, or the window when it is the only tab. A preview has
    /// nothing to save.
    pub(crate) fn close_tab_or_window(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tab_count() > 1 {
            self.close_tab_now(index, window, cx);
        } else {
            window.remove_window();
        }
    }
}
