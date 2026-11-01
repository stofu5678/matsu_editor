//! What the outline pane does with its place, its sort and its right-click menu.
//!
//! Sakura: `CDlgFuncList::DoMenu` (the menu and what its items do), `ChangeLayout` (the place
//! follows the settings), `OnLButtonUp` (the size is kept when the bar is let go), `OnNotify`
//! (`LVN_COLUMNCLICK`) and `OnCbnSelEndOk` (the sort is kept in the type).

use gpui::{
    AnyElement, App, Context, DismissEvent, Entity, Focusable as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, Pixels, Point, PromptLevel, Subscription, Window,
    anchored, deferred, div, px,
};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use matsu_outline::{
    ListColumn, ListSort, OutlineKind, SortType, TreeNode, ViewType, tree::sort_tree,
};

use super::OutlinePane;
use crate::outline::dock::{self, DockPlace, DockSide, OutlineDock};
use crate::outline::menu::{self, MenuCommand, MenuEntry, MenuState};

/// The right-click menu that is open.
pub(super) struct OpenMenu {
    popup: Entity<PopupMenu>,
    at: Point<Pixels>,
    _dismiss: Subscription,
}

impl OutlinePane {
    /// The place the pane has now: the one of the settings in use (`ProfDockSide` and the others),
    /// or, while the windows are not in sync, the one it had drawn before.
    pub(crate) fn place(&self, cx: &App) -> DockPlace {
        let dock = dock::current(cx);
        if !dock.sync
            && let Some(kept) = self.shown_place.get()
        {
            return kept;
        }
        let place = dock.place(self.file_tree_type_index(cx));
        self.shown_place.set(Some(place));
        place
    }

    /// Changes the place. The settings of the program take it, and the file too when `persist`
    /// (a size that is being dragged waits for the end of the drag). The windows draw again.
    fn change_place(
        &mut self,
        change: impl FnOnce(&mut DockPlace),
        persist: bool,
        cx: &mut Context<Self>,
    ) {
        let dock = dock::current(cx);
        let target = dock.target(self.file_tree_type_index(cx));
        let mut place = self.place(cx);
        change(&mut place);
        self.shown_place.set(Some(place));
        let apply = move |d: &mut OutlineDock| d.set_place(target, place);
        if persist {
            dock::update(cx, apply, move |profile, d| {
                dock::write_place(profile, target, &d.place_of(target))
            });
        } else {
            dock::set_memory(cx, apply);
        }
        self.relayout(cx);
    }

    /// The window lays the pane out again; the other windows too, if they follow.
    fn relayout(&self, cx: &mut Context<Self>) {
        cx.notify();
        cx.refresh_windows();
    }

    /// The pane is open (`bOutlineDockDisp`): kept for the next start.
    pub(crate) fn set_disp(&mut self, open: bool, cx: &mut Context<Self>) {
        self.change_place(|p| p.disp = open, true, cx);
    }

    /// 左ドッキング ... ドッキング禁止.
    pub(crate) fn set_side(&mut self, side: DockSide, cx: &mut Context<Self>) {
        self.change_place(
            |p| {
                p.side = side;
                p.disp = true;
            },
            true,
            cx,
        );
    }

    /// The splitter is dragged: the width (height) of the side the pane is on.
    pub(crate) fn resize(&mut self, size: i32, cx: &mut Context<Self>) {
        let side = self.place(cx).side.size_side();
        self.change_place(|p| p.set_size(side, size), false, cx);
    }

    /// The splitter is let go (`OnLButtonUp`): the size goes to the file.
    pub(crate) fn finish_resize(&mut self, cx: &mut Context<Self>) {
        let dock = dock::current(cx);
        let target = dock.target(self.file_tree_type_index(cx));
        dock::flush(cx, move |profile, d| {
            dock::write_place(profile, target, &d.place_of(target))
        });
    }

    /// ドッキング配置を同期.
    fn toggle_sync(&mut self, cx: &mut Context<Self>) {
        dock::update(cx, |d| d.sync = !d.sync, dock::write_common);
        self.shown_place.set(None);
        self.relayout(cx);
    }

    /// ドッキング配置を共通継承 (0) / タイプ別継承 (1).
    fn set_inherit(&mut self, set: i32, cx: &mut Context<Self>) {
        dock::update(cx, |d| d.dock_set = set, dock::write_common);
        self.shown_place.set(None);
        self.relayout(cx);
    }

    /// 継承情報を統一: the place of this pane goes to `[Common]` and to every type, after the
    /// question.
    fn ask_unify(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = crate::boxes::prompt(
            window,
            PromptLevel::Info,
            "MATSU Editor",
            Some(menu::UNIFY_MESSAGE),
            &["OK", "キャンセル"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update(cx, |this, cx| this.unify(cx)).ok();
            }
        })
        .detach();
    }

    pub(crate) fn unify(&mut self, cx: &mut Context<Self>) {
        let mut place = self.place(cx);
        place.disp = true;
        let count = crate::doc_types::with_user(cx, |user| user.document_types().len());
        dock::update(
            cx,
            move |d| d.unify(place, count),
            move |profile, d| {
                let mut changed = dock::write_place(profile, dock::Target::Common, &d.common)?;
                for index in 0..count {
                    changed |= dock::write_place(
                        profile,
                        dock::Target::Type(index),
                        &d.type_outline(index).place,
                    )?;
                }
                Ok(changed)
            },
        );
        // "（現在開いている他画面の状態も統一します。）": the other windows take it too, also
        // the ones that are not in sync
        let host = self.host.entity_id();
        for (_, workspace) in crate::file_ops::windows::listed_workspaces(cx) {
            if workspace.entity_id() != host
                && let Some(pane) = workspace.read(cx).outline.clone()
                && pane.entity_id() != cx.entity_id()
            {
                pane.read(cx).shown_place.set(None);
            }
        }
        self.shown_place.set(None);
        self.relayout(cx);
    }

    // ---- the sort ----

    /// Takes the sort of the type of the view (`m_nSortCol`, `m_bSortDesc` and `m_nSortType` are
    /// read from the type when the dialog opens and when the type changes). Returns whether the
    /// sort changed.
    pub(super) fn sync_sort(&mut self, cx: &App) -> bool {
        let kept = dock::current(cx).type_outline(self.file_tree_type_index(cx));
        let list_sort = ListSort {
            column: ListColumn::from_index(kept.sort_col),
            descending: kept.sort_desc,
        };
        let sort_type = SortType::from_index(kept.sort_type);
        let changed = list_sort != self.list_sort || sort_type != self.sort_type;
        self.list_sort = list_sort;
        self.sort_type = sort_type;
        changed
    }

    /// The tree and the list are sorted again.
    pub(super) fn resort(&mut self, cx: &mut Context<Self>) {
        if let Some(shown) = &mut self.shown {
            sort_tree(&mut shown.tree, self.sort_type);
        }
        self.rebuild_rows();
        cx.notify();
    }

    /// The sort goes to the type (`SetTypeConfig` after a click on a column or a choice).
    pub(super) fn keep_sort(&mut self, cx: &mut Context<Self>) {
        let index = self.file_tree_type_index(cx);
        let (column, descending, tree) = (
            self.list_sort.column.index(),
            self.list_sort.descending,
            self.sort_type.index(),
        );
        dock::update(
            cx,
            move |d| d.set_sort(index, column, descending, tree),
            move |profile, d| dock::write_sort(profile, index, &d.type_outline(index)),
        );
    }

    // ---- the right-click menu ----

    fn menu_state(&self, cx: &App) -> MenuState {
        let place = self.place(cx);
        let dock = dock::current(cx);
        MenuState {
            tree: self.outline_type == OutlineKind::FileTree
                || self
                    .shown
                    .as_ref()
                    .is_some_and(|s| s.view == ViewType::Tree),
            bookmarks: self.outline_type == OutlineKind::Bookmark,
            copy_enabled: self.copy_enabled(),
            has_selection: self.selected.is_some(),
            side: place.side,
            sync: dock.sync,
            dock_set: dock.dock_set,
        }
    }

    /// `OnContextMenu`: opens the menu at `at`.
    pub(super) fn open_menu(
        &mut self,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = menu::entries(self.menu_state(cx));
        let pane = cx.weak_entity();
        let focus = self.focus.clone();
        let popup = PopupMenu::build(window, cx, |popup, window, cx| {
            let popup = popup.action_context(focus.clone());
            fill(popup, &entries, &pane, &focus, window, cx).scrollable(true)
        });
        let dismiss = cx.subscribe_in(&popup, window, |this, _, _: &DismissEvent, window, cx| {
            this.close_menu(window, cx)
        });
        popup.read(cx).focus_handle(cx).focus(window, cx);
        self.menu = Some(OpenMenu {
            popup,
            at,
            _dismiss: dismiss,
        });
        cx.notify();
    }

    /// Whether the right-click menu is open.
    #[cfg(test)]
    pub(crate) fn menu_is_open(&self) -> bool {
        self.menu.is_some()
    }

    fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.menu.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    /// The popup, a child of the pane while it is open.
    pub(super) fn render_menu(&self) -> Option<AnyElement> {
        let open = self.menu.as_ref()?;
        Some(
            deferred(
                anchored()
                    .position(open.at)
                    .anchor(gpui::Anchor::TopLeft)
                    .snap_to_window_with_margin(px(8.))
                    .child(div().occlude().child(open.popup.clone())),
            )
            .into_any_element(),
        )
    }

    /// What an item of the menu does (`DoMenu` after `TrackPopupMenu`).
    pub(crate) fn run_menu_command(
        &mut self,
        command: MenuCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            MenuCommand::Update => self.reload(cx),
            MenuCommand::Copy => self.copy(cx),
            MenuCommand::ExpandAll => self.expand_all(true, cx),
            MenuCommand::CollapseAll => self.expand_all(false, cx),
            MenuCommand::BookmarkDelete => self.delete_selected_bookmark(cx),
            MenuCommand::BookmarkDeleteAll => {
                if let Some(view) = self.view.upgrade() {
                    view.update(cx, |view, cx| {
                        view.run_with(cx, |c| c.clear_bookmarks());
                    });
                }
                self.reload(cx);
            }
            MenuCommand::Side(side) => self.set_side(side, cx),
            MenuCommand::Sync => self.toggle_sync(cx),
            MenuCommand::Inherit(set) => self.set_inherit(set, cx),
            MenuCommand::Unify => self.ask_unify(window, cx),
            MenuCommand::Close => self.close(window, cx),
        }
    }

    /// すべて展開 / すべて縮小 (the timers 3 and 4 of `CDlgFuncList`). A folder of the file tree
    /// that has not been listed is listed when it opens, and its folders open at the next
    /// すべて展開.
    pub(crate) fn expand_all(&mut self, open: bool, cx: &mut Context<Self>) {
        if !open {
            self.expanded.clear();
        } else if self.outline_type == OutlineKind::FileTree {
            let ids: Vec<usize> = self.file_tree.as_ref().map_or_else(Vec::new, |tree| {
                (0..tree.nodes.len())
                    .filter(|&id| tree.nodes[id].has_children())
                    .collect()
            });
            for id in ids {
                self.expanded.insert(id);
                self.start_load(id, cx);
            }
        } else if let Some(shown) = &self.shown {
            fn collect(nodes: &[TreeNode], into: &mut Vec<usize>) {
                for node in nodes {
                    if !node.children.is_empty() {
                        into.push(node.order);
                        collect(&node.children, into);
                    }
                }
            }
            let mut ids = Vec::new();
            collect(&shown.tree, &mut ids);
            self.expanded.extend(ids);
        }
        self.rebuild_rows();
        cx.notify();
    }

    /// ブックマーク削除: the bookmark of the line of the selected item goes, and the list is made
    /// again.
    fn delete_selected_bookmark(&mut self, cx: &mut Context<Self>) {
        let line = self
            .selected_row()
            .and_then(|row| self.rows.get(row)?.item)
            .and_then(|item| self.shown.as_ref()?.analysis.items.get(item))
            .map(|item| item.line);
        if let (Some(line), Some(view)) = (line, self.view.upgrade()) {
            view.update(cx, |view, cx| {
                view.run_with(cx, |c| c.set_bookmark_line(line.saturating_sub(1), false));
            });
        }
        self.reload(cx);
    }
}

/// Puts `entries` in `popup`; an item runs its command on the pane.
fn fill(
    mut popup: PopupMenu,
    entries: &[MenuEntry],
    pane: &gpui::WeakEntity<OutlinePane>,
    focus: &gpui::FocusHandle,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    for entry in entries {
        popup = match entry {
            MenuEntry::Item {
                label,
                command,
                checked,
                disabled,
            } => {
                let item = PopupMenuItem::new(*label)
                    .checked(*checked)
                    .disabled(*disabled);
                popup.item(if *disabled {
                    item
                } else {
                    let (pane, command) = (pane.clone(), *command);
                    item.on_click(move |_, window, cx| {
                        pane.update(cx, |pane, cx| pane.run_menu_command(command, window, cx))
                            .ok();
                    })
                })
            }
            MenuEntry::Separator => popup.separator(),
            MenuEntry::Submenu { label, entries } => {
                let (entries, pane, focus) = (entries.clone(), pane.clone(), focus.clone());
                popup.submenu(*label, window, cx, move |sub, window, cx| {
                    let sub = sub.action_context(focus.clone());
                    fill(sub, &entries, &pane, &focus, window, cx)
                })
            }
        };
    }
    popup
}
