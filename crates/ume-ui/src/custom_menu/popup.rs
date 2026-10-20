//! Showing a custom menu: a GPUI popup at a point of the window.
//!
//! Sakura: `CEditView::CreatePopUpMenuSub` opens it with `TrackPopupMenu` at the mouse pointer, 4 pixels
//! higher (`po.y -= 4`), for the right click and for a key alike. Here the right click opens it at the
//! pointer ([`CustomMenuState::mouse_at`]); a key opens it at the pointer too, where the pointer is (Sakura
//! does the same: `::GetCursorPos` whatever ran the function).

use gpui::{
    AnyElement, Context, DismissEvent, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, MenuItem, ParentElement, Pixels, Point, Subscription, Window, anchored, deferred,
    div, point, px,
};
use gpui_component::menu::PopupMenu;
use ume_commands::keymap::DisplayStyle;
use ume_config::settings::custom_menu::INDEX_FOR_RBUTTONUP;

use super::ChainedLists;
use super::model::{self, CustomMenuList};
use crate::app::gpui_items;
use crate::history::menu::RecentLists;
use crate::menu::Registered;
use crate::menu_bar::{fill, popup_max_height};
use crate::settings::UserSettings;
use crate::view::EditorView;

/// How far above the pointer the popup starts (`po.y -= 4`).
const ABOVE_POINTER: f32 = 4.0;

/// The popup that is open, and where the next one opens.
#[derive(Default)]
pub(crate) struct CustomMenuState {
    open: Option<OpenPopup>,
    /// The pointer position of the right click that is being handled: the menu opens there instead of at
    /// the caret.
    pub(super) mouse_at: Option<Point<Pixels>>,
}

struct OpenPopup {
    popup: Entity<PopupMenu>,
    at: Point<Pixels>,
    /// The height of the window that the popup was sized for.
    window_height: Pixels,
    _dismiss: Subscription,
}

impl CustomMenuState {
    /// Shows `popup` at `at` in place of the one that is open (the menu of a macro's `CreateMenu`
    /// uses the same place). `window_height` is the height the popup was sized for.
    pub(crate) fn show(
        &mut self,
        popup: Entity<PopupMenu>,
        at: Point<Pixels>,
        window_height: Pixels,
        dismiss: Subscription,
    ) {
        self.open = Some(OpenPopup {
            popup,
            at,
            window_height,
            _dismiss: dismiss,
        });
    }

    /// Whether a popup is open.
    #[cfg(test)]
    pub(crate) fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Where the open popup starts.
    #[cfg(test)]
    pub(crate) fn open_at(&self) -> Option<Point<Pixels>> {
        self.open.as_ref().map(|o| o.at)
    }
}

impl EditorView {
    /// Whether a custom menu is open (`m_bInMenuLoop`).
    pub(crate) fn custom_menu_is_open(&self) -> bool {
        self.custom_menu.open.is_some()
    }

    /// `Command_CUSTMENU`: shows custom menu `index` (0 is the right-click menu). A menu with no items
    /// shows nothing, as in Sakura. Returns whether a popup opened.
    pub(crate) fn show_custom_menu(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let menus = cx
            .try_global::<UserSettings>()
            .map(UserSettings::custom_menus)
            .unwrap_or_default();
        let empty = menus.get(index).is_none_or(|m| m.items.is_empty());
        // The right-click menu (0) also shows the keyword help items, whatever it holds
        // (`CreatePopUpMenu_R` does not look at the number of items).
        let (help_top, help_bottom) = if index == INDEX_FOR_RBUTTONUP {
            let by_mouse = self.custom_menu.mouse_at.is_some();
            self.key_help_menu_items(by_mouse, window, cx)
        } else {
            (Vec::new(), Vec::new())
        };
        if empty && help_top.is_empty() && help_bottom.is_empty() {
            return false;
        }
        let recent = RecentLists::new(crate::history::current(cx))
            .with_names(crate::file_names::current(cx));
        let custom = CustomMenuList(&menus);
        let macros = crate::macros::registered_table(cx);
        let macro_list = crate::macros::MacroList(macros.clone());
        let rest = ChainedLists(&custom, &macro_list);
        let lists = ChainedLists(&recent, &rest);
        let entries = if empty {
            Vec::new()
        } else {
            model::entries(
                index,
                &model::Context {
                    menus: &menus,
                    macros: &macros,
                    keymap: &self.keymap,
                    available: &Registered,
                    lists: &lists,
                    parentheses: crate::doc_types::with_user(cx, |user| user.key_parentheses()),
                },
            )
        };
        self.open_menu_entries_with(entries, help_top, help_bottom, window, cx);
        true
    }

    /// [`open_menu_entries`](Self::open_menu_entries) at `at`, the place of a right click.
    pub(crate) fn open_menu_entries_at(
        &mut self,
        entries: Vec<crate::menu::Entry>,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.custom_menu.mouse_at = Some(at);
        self.open_menu_entries(entries, window, cx);
        self.custom_menu.mouse_at = None;
    }

    /// Shows `entries` in a popup at the pointer (or at the place of the right click that is
    /// being handled), 4 pixels higher. The status bar's line ending menu uses it too.
    pub(crate) fn open_menu_entries(
        &mut self,
        entries: Vec<crate::menu::Entry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_menu_entries_with(entries, Vec::new(), Vec::new(), window, cx);
    }

    /// [`open_menu_entries`](Self::open_menu_entries) with items of the program's own before and
    /// after the entries (the keyword help items of the right-click menu).
    fn open_menu_entries_with(
        &mut self,
        entries: Vec<crate::menu::Entry>,
        before: Vec<MenuItem>,
        after: Vec<MenuItem>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let at = self
            .custom_menu
            .mouse_at
            .take()
            .map(|p| point(p.x, p.y - px(ABOVE_POINTER)))
            // Sakura: `::GetCursorPos` for a key as well. GPUI gives the last position it saw the pointer at in
            // this window; if the pointer is outside, that is the place where it left (or the corner, if
            // it never came in), and the popup is moved into the window (`snap_to_window_with_margin`).
            .unwrap_or_else(|| {
                let p = window.mouse_position();
                point(p.x, p.y - px(ABOVE_POINTER))
            });
        self.open_popup_menu_with(entries, before, after, at, window, cx);
    }

    /// Shows `entries` as a popup menu whose top left corner is at `at`, a point of the window.
    /// The items run as those of the main menu do: the functions through
    /// [`RunFunction`](crate::view::RunFunction), the recent files and folders through their own
    /// actions.
    pub(crate) fn open_popup_menu(
        &mut self,
        entries: Vec<crate::menu::Entry>,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_popup_menu_with(entries, Vec::new(), Vec::new(), at, window, cx);
    }

    /// [`open_popup_menu`](Self::open_popup_menu) with items of the program's own before and after
    /// the entries.
    fn open_popup_menu_with(
        &mut self,
        entries: Vec<crate::menu::Entry>,
        before: Vec<MenuItem>,
        after: Vec<MenuItem>,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let style = if cfg!(target_os = "macos") {
            DisplayStyle::Mac
        } else {
            DisplayStyle::Sakura
        };
        let mut items: Vec<_> = before
            .into_iter()
            .chain(gpui_items(entries, style))
            .chain(after)
            .map(MenuItem::owned)
            .collect();
        self.mark_menu_items(&mut items, window, cx);
        let handle = self.focus_handle.clone();
        let max_height = popup_max_height(window.viewport_size().height, at.y);
        let popup = PopupMenu::build(window, cx, |menu, window, cx| {
            let menu = menu.action_context(handle.clone());
            fill(menu, items, Some(&handle), window, cx)
                .scrollable(true)
                .max_h(max_height)
        });
        let dismiss = cx.subscribe_in(&popup, window, |this, _, _: &DismissEvent, window, cx| {
            this.close_custom_menu(window, cx)
        });
        popup.read(cx).focus_handle(cx).focus(window, cx);
        self.custom_menu.open = Some(OpenPopup {
            popup,
            at,
            window_height: window.viewport_size().height,
            _dismiss: dismiss,
        });
        cx.notify();
    }

    /// The check marks and the gray items of the moment (`CreatePopUpMenuSub` asks `IsFuncChecked`
    /// and `IsFuncEnable` for each item).
    pub(crate) fn mark_menu_items(
        &self,
        items: &mut [gpui::OwnedMenuItem],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The window keeps the outline pane. (The view is not inside an update of its window here:
        // a menu opens from a mouse event or a function that was dispatched to the view.)
        let host = self.split_host.as_ref().and_then(gpui::WeakEntity::upgrade);
        let outline = host
            .as_ref()
            .is_some_and(|workspace| workspace.read(cx).outline.is_some());
        let documents = host.as_ref().map_or(1, |workspace| {
            workspace.read(cx).tab_count()
                + crate::app::documents_elsewhere(cx, workspace.entity_id())
        });
        let topmost = crate::window_ops::is_topmost(window, cx);
        let states = crate::menu_state::FuncStates::of_view(self, outline, topmost, documents, cx);
        crate::menu_state::apply(items, &states);
    }

    /// Closes the popup and gives the focus back to the text.
    pub(crate) fn close_custom_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.custom_menu.open.take().is_some() {
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// The popup, to be a child of the view while it is open. A menu of Sakura is modal and a resize of the
    /// window ends it; the popup is sized for the window it opened in, so it closes when the height changes.
    pub(crate) fn custom_menu_popup(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if self
            .custom_menu
            .open
            .as_ref()
            .is_some_and(|o| o.window_height != window.viewport_size().height)
        {
            self.close_custom_menu(window, cx);
        }
        let open = self.custom_menu.open.as_ref()?;
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
}
