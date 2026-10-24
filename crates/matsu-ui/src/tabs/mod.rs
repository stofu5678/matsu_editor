//! Tabs and groups (ADR-0015): a MATSU Editor window holds its documents in tabs.
//!
//! A tab is what a window held before tabs: a document and its panes (`crate::panes`). Sakura's
//! "group" is one window here: Sakura makes every document its own window and stacks them in one
//! frame, which GPUI cannot do (ADR-0015). The tab bar starts hidden as in Sakura
//! (`m_bDispTabWnd` is off), and then every document has its own window as before.
//!
//! * [`order`]: which tab follows a close, where a tab moves (numbers only).
//! * [`settings`]: `bDispTabWnd`, `bDispTabWndMultiWin` and `bNewWindow` of `[Common]`.
//! * [`bar`]: the tab bar.
//! * [`flows`]: what the tab functions do across windows: gather, spread, separate, join, and
//!   where a new document opens.
//!
//! The content of the tab that is active is `Workspace::content` (its panes, `Workspace::panes()`),
//! so the code of the split view works on the active tab as before; the other tabs keep their panes in [`Tabs`]. A key macro
//! belongs to the tab where it was started (Sakura records in one edit window), while the search
//! pattern and the status bar follow the user from tab to tab.

pub(crate) mod bar;
pub(crate) mod caption;
pub(crate) mod content;
#[cfg(test)]
mod content_tests;
#[cfg(test)]
mod drop_tests;
#[cfg(test)]
mod file_tests;
pub(crate) mod flows;
#[cfg(test)]
mod kind_menu_tests;
pub(crate) mod list;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod menu_tests;
pub(crate) mod order;
#[cfg(test)]
mod pending_tests;
mod preview_tab;
#[cfg(test)]
mod preview_tests;
mod semantics;
pub(crate) mod settings;
#[cfg(test)]
mod tests;

use gpui::{Context, Entity, Window};

use crate::app::Workspace;
use crate::panes::PaneSet;
use crate::view::EditorView;

/// A tab. The content of the active tab is `Workspace::content`, so its own is `None`.
pub(crate) struct Tab {
    id: u64,
    parked: Option<content::TabContent>,
}

/// The tabs of a window, in the order of the bar.
pub(crate) struct Tabs {
    list: Vec<Tab>,
    active: usize,
    /// The ids of the tabs, the one that was active last first (Sakura's window list is ordered
    /// by activity, `GetOpenedWindowArr`).
    recent: Vec<u64>,
    next_id: u64,
    /// The tests give a tab a menu kind other than the editor's (`Workspace::menu_kind`).
    #[cfg(test)]
    pub(crate) kinds: std::collections::HashMap<u64, content::MenuKind>,
}

impl Tabs {
    pub(crate) fn new() -> Self {
        Self {
            list: vec![Tab {
                id: 0,
                parked: None,
            }],
            active: 0,
            recent: vec![0],
            next_id: 1,
            #[cfg(test)]
            kinds: std::collections::HashMap::new(),
        }
    }

    fn ids(&self) -> Vec<u64> {
        self.list.iter().map(|t| t.id).collect()
    }

    fn touch(&mut self, id: u64) {
        self.recent.retain(|r| *r != id);
        self.recent.insert(0, id);
    }
}

impl Workspace {
    pub(crate) fn tab_count(&self) -> usize {
        self.tabs.list.len()
    }

    pub(crate) fn active_tab(&self) -> usize {
        self.tabs.active
    }

    /// Which menus the window shows: those of the kind of the active tab (ADR-0024).
    pub(crate) fn menu_kind(&self) -> content::MenuKind {
        #[cfg(test)]
        if let Some(kind) = self.tabs.kinds.get(&self.tabs.list[self.tabs.active].id) {
            return *kind;
        }
        self.content.panel().menu_kind()
    }

    /// Gives the menu bar of this window the menus of the active tab's kind, and on macOS asks for
    /// the native menu to be made again if the kind changed. Called when the active tab changes.
    pub(crate) fn sync_menu_kind(&mut self, cx: &mut Context<Self>) {
        let kind = self.menu_kind();
        self.menu_bar.update(cx, |bar, cx| bar.set_kind(kind, cx));
        crate::app::defer_refresh_menu_states(cx, false);
    }

    /// The panes of the active tab, which is an editor for now (`content::TabContent`).
    pub(crate) fn panes(&self) -> &PaneSet {
        self.content.panes()
    }

    pub(crate) fn panes_mut(&mut self) -> &mut PaneSet {
        self.content.panes_mut()
    }

    /// The panes of tab `index`.
    fn tab_panes(&self, index: usize) -> Option<&PaneSet> {
        self.tab_content(index)?.editor_panes()
    }

    /// What tab `index` holds.
    pub(crate) fn tab_content(&self, index: usize) -> Option<&content::TabContent> {
        if index == self.tabs.active {
            Some(&self.content)
        } else {
            self.tabs.list.get(index)?.parked.as_ref()
        }
    }

    /// The view that stands for tab `index` in the tab bar: its active pane, also for a tab that is
    /// not an editor (the view of its document that it keeps out of sight).
    pub(crate) fn tab_bar_view(&self, index: usize) -> Option<Entity<EditorView>> {
        self.tab_content(index).map(|c| c.panes().active().clone())
    }

    /// Whether one of the tabs is a preview (`crate::preview`).
    pub(crate) fn has_preview_tab(&self) -> bool {
        (0..self.tab_count()).any(|i| self.tab_content(i).is_some_and(|c| c.preview().is_some()))
    }

    /// Tab `index` as the window sees every kind of tab (`content::TabPanel`).
    pub(crate) fn tab_panel(&self, index: usize) -> Option<&dyn content::TabPanel> {
        Some(self.tab_content(index)?.panel())
    }

    /// The view that stands for tab `index`: its active pane.
    pub(crate) fn tab_editor(&self, index: usize) -> Option<Entity<EditorView>> {
        self.tab_panes(index).map(|set| set.active().clone())
    }

    /// The views that stand for the tabs, in the order of the bar.
    pub(crate) fn tab_editors(&self) -> Vec<Entity<EditorView>> {
        (0..self.tab_count())
            .filter_map(|i| self.tab_editor(i))
            .collect()
    }

    /// Every view of the window: all the panes of all the tabs.
    pub(crate) fn all_views(&self) -> Vec<Entity<EditorView>> {
        (0..self.tab_count())
            .filter_map(|i| self.tab_panes(i))
            .flat_map(|set| set.numbered().into_iter().map(|(_, view)| view))
            .collect()
    }

    /// The tab that has `view` among its panes.
    pub(crate) fn tab_of(&self, view: &Entity<EditorView>) -> Option<usize> {
        (0..self.tab_count()).find(|&i| self.tab_panes(i).is_some_and(|set| set.contains(view)))
    }

    /// Makes tab `index` the active one, and gives it the focus.
    pub(crate) fn activate_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index == self.tabs.active || index >= self.tab_count() {
            return;
        }
        let old_view = self.panes().active().clone();
        let Some(incoming) = self.tabs.list[index].parked.take() else {
            return;
        };
        let parked = std::mem::replace(&mut self.content, incoming);
        self.tabs.list[self.tabs.active].parked = Some(parked);
        self.tabs.active = index;
        let id = self.tabs.list[index].id;
        self.tabs.touch(id);
        let new_view = self.panes().active().clone();
        self.move_window_state_between(&old_view, &new_view, false, window, cx);
        window.focus(&self.content.panel().focus_handle(cx), cx);
        self.sync_menu_kind(cx);
        cx.notify();
    }

    /// Makes the tab that has `view` the active one.
    pub(crate) fn show_view(
        &mut self,
        view: &Entity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.tab_of(view) {
            self.activate_tab(index, window, cx);
        }
    }

    /// Adds a tab with `panes` at position `at` (the end if it is past it). With `activate` it
    /// becomes the active tab. The panes may come from another window.
    pub(crate) fn insert_tab(
        &mut self,
        panes: PaneSet,
        at: usize,
        activate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.insert_content(content::TabContent::Editor(panes), at, activate, window, cx);
    }

    /// [`insert_tab`](Self::insert_tab) for a tab of any kind.
    pub(crate) fn insert_content(
        &mut self,
        content: content::TabContent,
        at: usize,
        activate: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut incoming = content;
        // Only an editor's panes belong to the window as the views of its documents.
        let numbered = match &mut incoming {
            content::TabContent::Editor(panes) => {
                panes.forget_subscriptions();
                panes.numbered()
            }
            content::TabContent::Preview(_) => Vec::new(),
        };
        let id = self.tabs.next_id;
        self.tabs.next_id += 1;
        let at = at.min(self.tab_count());
        // The new tab is built as the active one and the old one is parked; if the new tab is not
        // to be active, the old one comes back at the end.
        let previous = self.tabs.list[self.tabs.active].id;
        let old_view = self.panes().active().clone();
        let parked = std::mem::replace(&mut self.content, incoming);
        self.tabs.list[self.tabs.active].parked = Some(parked);
        self.tabs.list.insert(at, Tab { id, parked: None });
        self.tabs.active = at;
        for (index, pane) in numbered {
            self.adopt_into_window(index, pane, window, cx);
        }
        self.tabs.touch(id);
        let new_view = self.panes().active().clone();
        self.move_window_state_between(&old_view, &new_view, false, window, cx);
        if activate {
            window.focus(&self.content.panel().focus_handle(cx), cx);
        } else if let Some(back) = self.tabs.list.iter().position(|t| t.id == previous) {
            self.activate_tab(back, window, cx);
        }
        self.sync_menu_kind(cx);
        cx.notify();
    }

    /// Takes tab `index` out of the window without closing it, to put it in another window. The
    /// window must keep a tab, so this is for windows with at least two. If the tab is the active
    /// one, the tab that was active before takes its place. A tab that is not an editor stays
    /// (`None`): it belongs to its window.
    pub(crate) fn take_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<PaneSet> {
        self.tab_panes(index)?;
        self.remove_tab(index, window, cx)?.into_editor()
    }

    /// Takes tab `index` out of the window, of any kind. Like [`take_tab`](Self::take_tab) for the
    /// rest.
    pub(crate) fn remove_tab(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<content::TabContent> {
        if self.tab_count() < 2 || index >= self.tab_count() {
            return None;
        }
        if index == self.tabs.active {
            let ids = self.tabs.ids();
            let next = order::after_close(&ids, ids[index], &self.tabs.recent)?;
            let next = ids.iter().position(|id| *id == next)?;
            self.activate_tab(next, window, cx);
        }
        let tab = self.tabs.list.remove(index);
        if index < self.tabs.active {
            self.tabs.active -= 1;
        }
        self.tabs.recent.retain(|id| *id != tab.id);
        cx.notify();
        tab.parked
    }

    /// Takes every tab out of the window, for a window that is about to close. The window has no
    /// panes afterwards and must not be drawn again.
    pub(crate) fn take_all_tabs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<PaneSet> {
        let mut taken = Vec::new();
        // From the right, so that the order of the bar is kept in the end.
        while self.tab_count() > 1 {
            let last = self.tab_count() - 1;
            // A tab that is not an editor is closed with its window.
            if let Some(set) = self
                .remove_tab(last, window, cx)
                .and_then(content::TabContent::into_editor)
            {
                taken.push(set);
            }
        }
        // A window left with only a preview keeps it: there are no panes to take, and the window
        // must still be drawable if the caller does not close it (a join that finds nothing).
        if self.content.editor_panes().is_some() {
            let emptied = content::TabContent::Editor(PaneSet::emptied());
            taken.extend(std::mem::replace(&mut self.content, emptied).into_editor());
        }
        taken.reverse();
        taken
    }

    /// Closes tab `index` without asking (the questions are asked before). The last tab is not
    /// closed here: the window goes with it.
    pub(crate) fn close_tab_now(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.remove_tab(index, window, cx).is_some()
    }

    /// Moves tab `from` to position `to` in the bar. The active tab stays active.
    pub(crate) fn move_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let count = self.tab_count();
        if from == to || from >= count || to >= count {
            return;
        }
        let active_id = self.tabs.list[self.tabs.active].id;
        let tab = self.tabs.list.remove(from);
        self.tabs.list.insert(to, tab);
        self.tabs.active = self
            .tabs
            .list
            .iter()
            .position(|t| t.id == active_id)
            .unwrap_or(0);
        cx.notify();
    }

    /// The tooltip of a tab (`CTabWnd::GetTabName` with `bFull`, for `TTN_GETDISPINFO`): the path of
    /// the file with the replacements of 「ファイル名表示」 and, if the setting says so, shortened,
    /// but not cut to the width; the caption of a document that has no file.
    pub(crate) fn tooltip_of(view: &EditorView, cx: &gpui::App) -> String {
        match view.document().read(cx).path() {
            Some(path) => crate::file_names::current(cx).name_of(Some(path), None, false),
            None => Self::caption_of(view, cx),
        }
    }

    /// The text of the tab in the bar (`CTabWnd::GetTabName`): the format of the tab caption
    /// (`szTabWndCaption`, [`caption`]) expanded for the document. Sakura's default format makes
    /// the name of the file, or `(無題)`, then the marks of unsaved changes, view mode (or a file
    /// that cannot be written) and key macro recording.
    pub(crate) fn caption_of(view: &EditorView, cx: &gpui::App) -> String {
        view.expand_format(&caption::format(cx), cx)
    }
}

impl EditorView {
    /// Runs `f` on the window of this view after the command that is running ends: the command
    /// holds this view, and the window changes the views it holds.
    pub(crate) fn with_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
    ) -> matsu_commands::command::Result {
        let Some(host) = self.split_host.clone() else {
            return Err(matsu_commands::command::CommandError::Unavailable(
                "the view has no window",
            ));
        };
        window.defer(cx, move |window, cx| {
            host.update(cx, |workspace, cx| f(workspace, window, cx))
                .ok();
        });
        Ok(())
    }

    /// Runs `f` after the command that is running ends, on the application (the functions that
    /// look at every window cannot run while this view or its window is borrowed).
    pub(crate) fn defer_in_window(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Window, &mut gpui::App) + 'static,
    ) -> matsu_commands::command::Result {
        window.defer(cx, f);
        Ok(())
    }
}
