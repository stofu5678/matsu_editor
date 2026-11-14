//! `F_WINDOW_LIST` of the main menu (ウィンドウ → ウィンドウ一覧): one item for each open document.
//!
//! Sakura: `CEditWnd::InitMenu_Special` (`sakura_core/window/CEditWnd.cpp`) calls
//! `CAppNodeManager::GetOpenedWindowArr(.., bSort = TRUE)` (`sakura_core/env/CAppNodeManager.cpp`: the
//! windows in the order of their numbers) and `CEditWnd::WinListMenu`, which makes the text of each with
//! `CFileNameManager::GetMenuFullLabel_WinList` ([`super::list::label`]), puts the check mark on the
//! window that owns the menu, and chooses by `IDM_SELWINDOW + index`: `CEditWnd::OnCommand` brings that
//! window to the front (`ActivateFrameWindow`). The popup of `F_WINLIST` (`CEditWnd::PopupWinList`) shows
//! the same list ([`super::list`]).
//!
//! A document is a tab here (ADR-0015); the tabs of every window come in the order the windows were
//! opened. The menu is made again when the list changes (`crate::app::refresh_window_list`).

use gpui::{App, EntityId};

use super::list::label;
use crate::file_ops::windows::listed_workspaces;
use crate::menu::{Item, Lists, MenuAction};

/// `F_WINDOW_LIST`, the row of the Window menu that stands for the open windows.
pub(crate) const F_WINDOW_LIST: u16 = 29001;

/// The items of `F_WINDOW_LIST`.
pub(crate) struct WindowListMenu(Vec<Item>);

impl WindowListMenu {
    /// The windows as they are now. The check mark is on the document the active window shows.
    pub(crate) fn collect(cx: &App) -> Self {
        let names = crate::file_names::current(cx);
        let shown = cx.active_window().and_then(|active| {
            listed_workspaces(cx)
                .into_iter()
                .find(|(handle, _)| *handle == active)
                .map(|(_, ws)| ws.read(cx).editor().entity_id())
        });
        let items = listed_workspaces(cx)
            .into_iter()
            .flat_map(|(_, ws)| ws.read(cx).tab_editors())
            .enumerate()
            .map(|(index, view)| {
                let shared = view.read(cx).document().read(cx);
                Item {
                    label: label(
                        index,
                        &names.name_of(shared.path(), shared.untitled_number(), true),
                        shared.model().is_modified(),
                        shared.file().encoding,
                    ),
                    action: MenuAction::Window {
                        id: view.entity_id().as_u64(),
                        current: shown == Some(view.entity_id()),
                    },
                    shortcut: None,
                    enabled: true,
                }
            })
            .collect();
        Self(items)
    }

    /// The items, to tell whether the list changed.
    pub(crate) fn into_items(self) -> Vec<Item> {
        self.0
    }
}

impl Lists for WindowListMenu {
    fn items(&self, code: u16) -> Vec<Item> {
        if code == F_WINDOW_LIST {
            self.0.clone()
        } else {
            Vec::new()
        }
    }
}

/// `IDM_SELWINDOW + index`: brings the window of the document `id`, and its tab, to the front.
pub(crate) fn select(cx: &mut App, id: u64) {
    let id = EntityId::from(id);
    if let Some((handle, view)) = crate::file_ops::windows::editors(cx)
        .into_iter()
        .find(|(_, view)| view.entity_id() == id)
    {
        crate::tabs::flows::reveal(cx, handle, view);
    }
}

#[cfg(test)]
mod tests;
