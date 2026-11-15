//! What the window does with its AI log tab (ADR-0030, `crate::ai_log`): opens it, and finds it again so that the
//! window has one only.

use gpui::{AppContext as _, Context, Entity, Window};

use super::content::TabContent;
use crate::ai_log::{AiLogTab, AiLogView};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::panes::PaneSet;
use crate::view::EditorView;

impl Workspace {
    /// The view of the log of this window, if it has the tab.
    pub(crate) fn ai_log_of_window(&self) -> Option<Entity<AiLogView>> {
        (0..self.tab_count()).find_map(|i| {
            self.tab_content(i)
                .and_then(TabContent::ai_log)
                .map(|tab| tab.view().clone())
        })
    }

    /// The tab that has the log of this window.
    fn ai_log_tab(&self) -> Option<usize> {
        (0..self.tab_count()).find(|&i| self.tab_content(i).is_some_and(|c| c.ai_log().is_some()))
    }

    /// The log of this window: the tab it has, or a new one next to the active tab. `focus` goes to the tab (a
    /// user asked for the log); without it the tab stays where it is, and a new one opens behind the active one,
    /// so that the document keeps the focus.
    pub(crate) fn ai_log_view(
        &mut self,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<AiLogView> {
        if let Some(index) = self.ai_log_tab() {
            if focus {
                self.activate_tab(index, window, cx);
            }
            return self.ai_log_of_window().expect("the tab is the log");
        }
        let host = cx.weak_entity();
        // The hidden view of an empty document, so that the window always has a view to ask.
        let document = cx.new(|_| SharedDocument::empty());
        let hidden = cx.new(|cx| EditorView::new(document, cx));
        hidden.update(cx, |view, _| view.split_host = Some(host.clone()));
        let view = cx.new(|cx| AiLogView::new(host, cx));
        let tab = TabContent::AiLog(AiLogTab::new(PaneSet::new(hidden), view.clone()));
        let at = self.active_tab() + 1;
        self.insert_content(tab, at, focus, window, cx);
        view
    }
}
