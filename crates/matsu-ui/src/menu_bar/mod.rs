// Based on `src/menu/app_menu_bar.rs` of gpui-component 0.7.1
// (https://github.com/longbridge/gpui-kit), Copyright 2024 - 2026 Longbridge
// <https://longbridge.com>, licensed under the Apache License, Version 2.0
// (https://www.apache.org/licenses/LICENSE-2.0). The crate has no NOTICE file.
//
// THIS FILE WAS MODIFIED by the MATSU Editor project on 2026-10-16. It is no longer the original.
// What changed:
// * `AppMenuBar` is `MenuBar`, `AppMenu` is `BarMenu`, and the code lives in this crate, so it can
//   only use the public API of gpui-component. gpui-component's `PopupMenu::with_menu_items` and
//   `set_action_context` are not public; `fill` below does the same with the public builders, and builds the popup again each time the menu opens (so the action context is
//   always the current one).
// * The popup is as tall as the window allows, from its top down to the bottom of the window
//   (`popup_max_height`). gpui-component limits a menu of more than 20 items to half of the
//   window, 450 px at most, which cut Sakura's long menus off (the scroll bar shows on hover only).
// * Access keys (Alt+letter, a letter in an open menu, a tap of Alt) as in Windows, which
//   gpui-component's menu bar does not have: the keys, the marked state of the bar and the fields
//   `armed`, `alt` and `key_pos` are new, and so is the `access` module.
// * The names, the `init` function (the key bindings are the ones `gpui_component::init` binds
//   for the context "AppMenuBar") and the test were adapted to that.
//
// The full text of the license is in the LICENSE-APACHE file of gpui-component and at the URL
// above. Licensed under the Apache License, Version 2.0; you may not use this file except in
// compliance with the License. Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.

//! The menu bar of the window for Windows and Linux (macOS has a native one).
//!
//! The behavior is gpui-component's `AppMenuBar`: the menus of the application
//! (`GlobalState::app_menus`), keyboard navigation, switching between open menus by hovering, and
//! the focus that goes back to where it was. The difference is the height of a long menu, see
//! [`popup_max_height`].
//!
//! It also has the access keys of a Win32 menu bar (Sakura's menus are Win32 menus): see the `keys` module
//! for Alt+letter and the tap of Alt, and `BarMenu::on_key_down` for the letters in an open menu.

mod access;
mod keys;
pub(crate) mod popup_keys;

use crate::app::Workspace;
use crate::menu_bar::access::{
    AltTap, Clickable, after_down, after_up, as_in_popup, is_clickable, item_key, key_of,
    next_match, presses_to_reach,
};
use crate::menu_bar::popup_keys::{KeyList, Rebuild};
use crate::menu_width::{Layout, Opening, Place, Row};
use crate::tabs::content::MenuKind;
use gpui::{
    Action, App, AppContext as _, Bounds, ClickEvent, Context, DismissEvent, Entity, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent,
    OwnedMenu, OwnedMenuItem, ParentElement, Pixels, Render, Role, SharedString, Size,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, anchored, deferred, div,
    prelude::FluentBuilder, px,
};
use gpui_base::ElementExt as _;
use gpui_base::actions::{Cancel, SelectDown, SelectLeft, SelectRight, SelectUp};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::PopupMenu;
use gpui_component::{GlobalState, InteractiveElementExt as _, Selectable, Sizable, h_flex};
use std::borrow::Cow;
use std::rc::Rc;

/// The key context that `gpui_component::init` binds Escape, Left and Right to.
const CONTEXT: &str = "AppMenuBar";

/// The space the popup keeps to the bottom edge of the window: the margin of
/// `snap_to_window_with_margin` below, and the border of the popup.
const BOTTOM_MARGIN: f32 = 10.0;
/// The popup is placed this far below the button (`top_1`).
const POPUP_OFFSET: f32 = 4.0;
/// Names the popup in the tests (`debug_selector` does nothing in release builds).
const POPUP_SELECTOR: &str = "matsu-menu-popup";
/// A window so small that a menu would have no room still gets this much.
const MIN_MENU_HEIGHT: f32 = 120.0;

/// The height of a menu that opens `top` pixels from the top of a window `window_height` high:
/// everything down to the bottom of the window, less a small margin. A menu that is taller than
/// this scrolls; one that fits does not.
pub fn popup_max_height(window_height: Pixels, top: Pixels) -> Pixels {
    (window_height - top - px(BOTTOM_MARGIN)).max(px(MIN_MENU_HEIGHT))
}

/// The application menu bar.
pub struct MenuBar {
    menus: Vec<Entity<BarMenu>>,
    selected_index: Option<usize>,
    action_context: Option<FocusHandle>,
    /// The window the bar belongs to: its state gives the check marks and the gray items.
    workspace: Option<WeakEntity<Workspace>>,
    /// Whose menus the bar shows: those of the kind of the active tab (ADR-0024).
    kind: MenuKind,
    /// The menu that is marked for the arrow keys after a tap of Alt, with no menu open.
    armed: Option<usize>,
    /// Whether Alt is down and nothing else happened yet (a lone Alt marks the bar).
    alt: AltTap,
}

impl MenuBar {
    /// Create a new menu bar from the menus in gpui-component's global state.
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let mut this = Self {
                selected_index: None,
                action_context: None,
                workspace: None,
                kind: MenuKind::Editor,
                armed: None,
                alt: AltTap::default(),
                menus: Vec::new(),
            };
            this.reload(cx);
            this
        })
    }

    /// Tells the bar which window it is in (`IsFuncChecked` and `IsFuncEnable` read that window).
    pub(crate) fn set_workspace(&mut self, workspace: WeakEntity<Workspace>) {
        self.workspace = Some(workspace);
    }

    /// Puts the check marks and the gray items of the window on `items`, as Sakura's `InitMenu`
    /// does each time a menu opens. Items stay as they are before the bar knows its window.
    pub(crate) fn mark_items(
        bar: &Entity<Self>,
        items: &mut [OwnedMenuItem],
        window: &mut Window,
        cx: &mut App,
    ) {
        let topmost = crate::window_ops::is_topmost(window, cx);
        let workspace = bar.read(cx).workspace.clone();
        let Some(workspace) = workspace.as_ref().and_then(WeakEntity::upgrade) else {
            return;
        };
        // A kind of tab that has no state of the editor leaves the items as they are made, except that the
        // shell grays its copy item while nothing is selected.
        let Some(states) = workspace
            .read(cx)
            .menu_states(topmost, workspace.entity_id(), cx)
        else {
            if workspace.read(cx).menu_kind() == MenuKind::Terminal {
                crate::terminal::menus::mark_items(items, workspace.read(cx), cx);
            }
            return;
        };
        crate::menu_state::apply(items, &states);
    }

    /// Shows the menus of `kind`, if the bar shows those of another kind now.
    pub(crate) fn set_kind(&mut self, kind: MenuKind, cx: &mut Context<Self>) {
        if self.kind != kind {
            self.kind = kind;
            self.reload(cx);
        }
    }

    /// The names of the menus of the bar, for the tests.
    #[cfg(test)]
    pub(crate) fn menu_names(&self, cx: &App) -> Vec<String> {
        self.menus
            .iter()
            .map(|m| m.read(cx).name.to_string())
            .collect()
    }

    /// Reload the menus from the app: those of the kind of the active tab when the application has
    /// made them (`crate::app::KindMenus`), else gpui-component's copy.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let menu_bar = cx.entity();
        let menus: Vec<OwnedMenu> = crate::app::KindMenus::of(cx, self.kind)
            .unwrap_or_else(|| GlobalState::global(cx).app_menus().to_vec());
        self.menus = menus
            .into_iter()
            .enumerate()
            .map(|(ix, menu)| BarMenu::new(ix, menu, menu_bar.clone(), cx))
            .collect();
        self.selected_index = None;
        self.action_context = None;
        self.armed = None;
        cx.notify();
    }

    fn on_move_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_index) = self.selected_index else {
            return;
        };
        let new_ix = if selected_index == 0 {
            self.menus.len().saturating_sub(1)
        } else {
            selected_index.saturating_sub(1)
        };
        self.set_selected_index(Some(new_ix), window, cx);
    }

    fn on_move_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        let Some(selected_index) = self.selected_index else {
            return;
        };
        let new_ix = if selected_index + 1 >= self.menus.len() {
            0
        } else {
            selected_index + 1
        };
        self.set_selected_index(Some(new_ix), window, cx);
    }

    fn on_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.set_selected_index(None, window, cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_selected_index_restoring(ix, true, window, cx);
    }

    /// `set_selected_index`; with `restore_focus` false, closing the menu leaves the focus where
    /// it is (a command of the menu has moved it already: see `handle_dismiss`).
    fn set_selected_index_restoring(
        &mut self,
        ix: Option<usize>,
        restore_focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_index.is_none() && ix.is_some() {
            self.action_context = window.focused(cx);
        } else if ix.is_none() {
            if restore_focus && let Some(action_context) = self.action_context.as_ref() {
                action_context.focus(window, cx);
            }
            self.action_context = None;
        }
        self.selected_index = ix;
        if ix.is_some() {
            self.armed = None;
        }
        cx.notify();
    }

    #[inline]
    fn has_activated_menu(&self) -> bool {
        self.selected_index.is_some()
    }
}

impl Render for MenuBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("app-menu-bar")
            .role(Role::MenuBar)
            .aria_label("メニューバー")
            .key_context(CONTEXT)
            .on_action(cx.listener(Self::on_move_left))
            .on_action(cx.listener(Self::on_move_right))
            .on_action(cx.listener(Self::on_cancel))
            .size_full()
            .gap_x_1()
            .overflow_x_scroll()
            .lock_scroll_axis()
            .children(self.menus.clone())
    }
}

/// Whether the focus is in the window but outside `popup`, once the popup announced that it is
/// dismissed: an item has run and moved the focus on (into a dialog it opened). The code that
/// closes a popup gives the focus back to the text only when this is false.
pub(crate) fn focus_left_popup(popup: &Entity<PopupMenu>, window: &Window, cx: &App) -> bool {
    window.focused(cx).is_some() && !popup.read(cx).focus_handle(cx).contains_focused(window, cx)
}

pub(crate) fn fill(
    mut menu: PopupMenu,
    items: Vec<OwnedMenuItem>,
    action_context: Option<&FocusHandle>,
    place: Place,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    // The items of an owned menu as a popup menu: what gpui-component's `with_menu_items` does,
    // with its public builders.
    //
    // With the icons on (`bMenuIcon`) and an icon among the items, every item is drawn by
    // `menu_icons::item_element` (a column for the pictures, then the label), and a submenu gets
    // an invisible icon as wide as that column so that the labels line up.
    let scale = window.scale_factor();
    let with_icons = crate::menu_icons::enabled(cx)
        && items.iter().any(|item| match item {
            OwnedMenuItem::Action { action, .. } => {
                function_picture(action.as_ref(), scale).is_some()
            }
            _ => false,
        });
    let icons = with_icons.then(|| px(crate::menu_icons::column_width(scale)));
    let (items, opening, widths) = fit_widths(items, window, place, icons);
    let mut widths = widths.into_iter();
    for item in items {
        menu = match item {
            OwnedMenuItem::Action {
                name,
                action,
                checked,
                disabled,
                ..
            } if with_icons => {
                let picture = function_picture(action.as_ref(), scale);
                menu.menu_element_with_disabled(action, disabled, move |_, _| {
                    crate::menu_icons::item_element(
                        SharedString::from(name.clone()),
                        picture.clone(),
                        checked,
                        disabled,
                        scale,
                    )
                })
            }
            OwnedMenuItem::Action {
                name,
                action,
                checked,
                disabled,
                ..
            } => menu.menu_with_check_and_disabled(name, checked, action, disabled),
            OwnedMenuItem::Separator => menu.separator(),
            OwnedMenuItem::Submenu(submenu) => {
                let context = action_context.cloned();
                let icon = with_icons.then(|| {
                    gpui_component::Icon::empty().mr(px(crate::menu_icons::submenu_padding(scale)))
                });
                let items = submenu.items;
                // The submenu's own place: on the left side it starts where its own width puts it.
                let child = opening.child_for(widths.next().unwrap_or_default());
                // The submenu takes the access keys too (`CMenuDrawer::OnMenuChar` answers
                // `WM_MENUCHAR` for whichever menu has the focus): it is registered with the
                // popups whose letters `popup_keys::key` handles, with the way to fill it again.
                let keys = KeyList::of_owned(items.clone());
                let fill_again: Rebuild = Rc::new(move |sub, window, cx| {
                    let sub = match &context {
                        Some(handle) => sub.action_context(handle.clone()),
                        None => sub,
                    };
                    // As gpui-component does: a long submenu scrolls within its default height.
                    let scrollable = items.len() > 20;
                    fill(sub, items.clone(), context.as_ref(), child, window, cx)
                        .scrollable(scrollable)
                });
                menu.submenu_with_icon(icon, submenu.name, window, cx, move |sub, window, cx| {
                    popup_keys::register(&cx.entity(), keys.clone(), fill_again.clone(), cx);
                    fill_again(sub, window, cx)
                })
            }
            OwnedMenuItem::SystemMenu(_) => menu,
        };
    }
    menu.max_w(opening.max_width)
}

/// [`fit_widths`] without the widths of the submenus.
#[cfg(test)]
fn fit_width(
    items: Vec<OwnedMenuItem>,
    window: &Window,
    place: Place,
    icons: Option<Pixels>,
) -> (Vec<OwnedMenuItem>, Opening) {
    let (items, opening, _) = fit_widths(items, window, place, icons);
    (items, opening)
}

/// `items` with the labels that are wider than the room of the menu at `place` cut
/// (`menu_width`), how the menu opens (its `max_w` and the place of its submenus), and the width
/// each submenu would like, in the order of the submenus.
fn fit_widths(
    items: Vec<OwnedMenuItem>,
    window: &Window,
    place: Place,
    icons: Option<Pixels>,
) -> (Vec<OwnedMenuItem>, Opening, Vec<Pixels>) {
    let layout = Layout {
        icons,
        checks: items
            .iter()
            .any(|item| matches!(item, OwnedMenuItem::Action { checked: true, .. })),
        submenus: items
            .iter()
            .any(|item| matches!(item, OwnedMenuItem::Submenu(_))),
    };
    let fit = crate::menu_width::Fit::new(window, place, layout);
    let mut own = Pixels::ZERO;
    let mut submenu = Pixels::ZERO;
    let mut widths = Vec::new();
    let items = items
        .into_iter()
        .map(|item| match item {
            OwnedMenuItem::Action {
                name,
                action,
                os_action,
                checked,
                disabled,
            } => {
                let name = fit.owned(name, Row::Action);
                own = own.max(fit.width_of(&name, Row::Action));
                OwnedMenuItem::Action {
                    name,
                    action,
                    os_action,
                    checked,
                    disabled,
                }
            }
            OwnedMenuItem::Submenu(mut menu) => {
                let cut = match fit.label(&menu.name, Row::Submenu) {
                    Cow::Owned(cut) => Some(cut),
                    Cow::Borrowed(_) => None,
                };
                if let Some(cut) = cut {
                    menu.name = cut.into();
                }
                own = own.max(fit.width_of(&menu.name, Row::Submenu));
                // The labels of the submenu are cut when it is filled, to its own room.
                let width = widest_label(&menu.items, &fit);
                widths.push(width);
                submenu = submenu.max(width);
                OwnedMenuItem::Submenu(menu)
            }
            item => item,
        })
        .collect();
    (items, fit.opening(own, submenu), widths)
}

/// The width the widest label of `items` would like, as the rows of a menu of their own: the
/// items and the headings of the submenus in them (a heading takes room in the menu too). The
/// pictures are those of the parent's menu.
fn widest_label(items: &[OwnedMenuItem], fit: &crate::menu_width::Fit) -> Pixels {
    let fit = fit.with(Layout {
        icons: fit.layout().icons,
        checks: items
            .iter()
            .any(|item| matches!(item, OwnedMenuItem::Action { checked: true, .. })),
        submenus: items
            .iter()
            .any(|item| matches!(item, OwnedMenuItem::Submenu(_))),
    });
    items
        .iter()
        .filter_map(|item| match item {
            OwnedMenuItem::Action { name, .. } => Some((name.as_str(), Row::Action)),
            OwnedMenuItem::Submenu(menu) => Some((menu.name.as_ref(), Row::Submenu)),
            _ => None,
        })
        .map(|(name, row)| fit.natural(name, row))
        .fold(Pixels::ZERO, Pixels::max)
}

/// The picture of the icon of the function an item runs, if it runs one that has an icon.
fn function_picture(action: &dyn Action, scale_factor: f32) -> Option<std::sync::Arc<gpui::Image>> {
    let run = action.as_any().downcast_ref::<crate::view::RunFunction>()?;
    crate::menu_icons::picture(run.code, scale_factor)
}

/// A menu in the menu bar.
pub(crate) struct BarMenu {
    menu_bar: Entity<MenuBar>,
    ix: usize,
    name: SharedString,
    menu: OwnedMenu,
    popup_menu: Option<Entity<PopupMenu>>,
    /// The size of the window that `popup_menu` was sized for (its height and its width).
    popup_window_size: Size<Pixels>,
    /// Where the button was at the last paint, to size the popup below it.
    bounds: Bounds<Pixels>,
    /// The item selected in the open popup. The popup does not tell, so this follows the Up and
    /// Down actions that reach it (`track_down`, `track_up`). Pointing at an item with the mouse
    /// is not seen, so a mouse move sets `pos_stale` and the popup is built again before the
    /// access keys move the selection.
    key_pos: Option<usize>,
    /// The mouse moved over the open menu, which may have changed its selection: `key_pos` is not
    /// to be trusted until the popup is built again (its selection is `None` then).
    pos_stale: bool,
    /// An access key that waits for the popup to be built again and drawn.
    pending_key: Option<char>,
    /// Opening the submenu that the keys selected: 1, the Right key waits for the next draw; 2,
    /// it was sent and the next draw checks that the focus went into the submenu.
    pending_right: u8,
    /// The popup for `pending_key` has been through one render.
    pending_drawn: bool,
    /// Whether the selection can stop on each item the open popup was built with (the gray ones
    /// cannot).
    popup_items: Vec<Reachable>,
    _subscription: Option<Subscription>,
}

/// An item of the open popup as far as the keys are concerned: the selection can stop on it.
struct Reachable(bool);

impl Clickable for Reachable {
    fn clickable(&self) -> bool {
        self.0
    }
}

impl BarMenu {
    fn new(ix: usize, menu: OwnedMenu, menu_bar: Entity<MenuBar>, cx: &mut App) -> Entity<Self> {
        let name = menu.name.clone();
        cx.new(|_| Self {
            ix,
            menu_bar,
            name,
            menu,
            popup_menu: None,
            popup_window_size: Size::default(),
            bounds: Bounds::default(),
            key_pos: None,
            pos_stale: false,
            pending_key: None,
            pending_drawn: false,
            pending_right: 0,
            popup_items: Vec::new(),
            _subscription: None,
        })
    }

    /// Whether this menu's popup is the one the menu bar currently has open.
    fn is_open(&self, cx: &App) -> bool {
        self.menu_bar.read(cx).selected_index == Some(self.ix)
    }

    fn build_popup_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let popup_menu = match self.popup_menu.as_ref() {
            Some(menu) => menu.clone(),
            None => {
                let action_context = self.menu_bar.read(cx).action_context.clone();
                let mut items = self.menu.items.clone();
                MenuBar::mark_items(&self.menu_bar, &mut items, window, cx);
                let items = as_in_popup(items);
                self.popup_items = items
                    .iter()
                    .map(|item| Reachable(is_clickable(item)))
                    .collect();
                let max_height = popup_max_height(
                    window.viewport_size().height,
                    self.bounds.bottom() + px(POPUP_OFFSET),
                );
                let place = Place::at(window, self.bounds.left());
                let popup_menu = PopupMenu::build(window, cx, |menu, window, cx| {
                    let menu = match &action_context {
                        Some(handle) => menu.action_context(handle.clone()),
                        None => menu,
                    };
                    fill(menu, items, action_context.as_ref(), place, window, cx)
                        .scrollable(true)
                        .max_h(max_height)
                });
                self._subscription =
                    Some(cx.subscribe_in(&popup_menu, window, Self::handle_dismiss));
                self.popup_menu = Some(popup_menu.clone());
                self.key_pos = None;
                self.pos_stale = false;
                self.popup_window_size = window.viewport_size();
                popup_menu
            }
        };

        let focus_handle = popup_menu.read(cx).focus_handle(cx);
        if !focus_handle.contains_focused(window, cx) {
            focus_handle.focus(window, cx);
        }
        popup_menu
    }

    fn handle_dismiss(
        &mut self,
        popup: &Entity<PopupMenu>,
        _: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The popup announces its dismissal after the item has run. A command that opened a
        // dialog drawn in the window (search, Grep, jump, ...) has moved the focus into it by
        // now; giving the focus back to the text then would leave the dialog without it, and the
        // keys typed next would go into the document. The focus goes back only when it is still
        // in the menu (or nowhere): Escape, a click outside, a command that moves nothing.
        let moved_away = window.focused(cx).is_some()
            && !popup.read(cx).focus_handle(cx).contains_focused(window, cx);
        self._subscription.take();
        self.popup_menu.take();
        self.menu_bar.update(cx, |state, cx| {
            state.set_selected_index_restoring(None, !moved_away, window, cx);
        });
    }

    fn handle_trigger_click(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, ClickEvent::Mouse(_)) {
            return;
        }
        self.toggle(window, cx);
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let is_open = self.is_open(cx);
        self.menu_bar.update(cx, |state, cx| {
            let new_ix = if is_open { None } else { Some(self.ix) };
            state.set_selected_index(new_ix, window, cx);
        });
    }

    /// A letter in the open menu (`CMenuDrawer::OnMenuChar`): the item with that access key runs
    /// (a submenu opens), or, if several have it, the selection goes to the next of them. Case does
    /// not matter. Only the menu itself is handled; the letters of a submenu that has the focus
    /// come through `popup_keys::key` (`fill` registers every submenu).
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let m = keystroke.modifiers;
        if !self.is_open(cx)
            || event.prefer_character_input
            || keystroke.is_ime_in_progress()
            || m.control
            || m.alt
            || m.platform
            || m.function
        {
            return;
        }
        let Some(key) = key_of(&keystroke.key) else {
            return;
        };
        if self.access_key(key, window, cx) {
            cx.stop_propagation();
        }
    }

    /// The letter `key` in the open menu. Returns whether an item has it (the key is taken).
    fn access_key(&mut self, key: char, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(popup) = self.popup_menu.clone() else {
            return false;
        };
        if !popup.read(cx).focus_handle(cx).is_focused(window) {
            return false;
        }
        // The marks of this moment (the popup may have been built before an edit); the popup's
        // own items say which ones the selection can stop on.
        let mut items = self.menu.items.clone();
        MenuBar::mark_items(&self.menu_bar, &mut items, window, cx);
        let items = as_in_popup(items);
        let with_key: Vec<usize> = (0..items.len())
            .filter(|&i| item_key(&items[i]) == Some(key))
            .collect();
        if with_key.is_empty() {
            return false;
        }
        // Several items with the key: the selection goes round them. A gray item cannot be
        // selected, so the round passes over it.
        if with_key.len() > 1 {
            let reachable: Vec<usize> = with_key
                .into_iter()
                .filter(|&i| {
                    is_clickable(&items[i])
                        && self.popup_items.get(i).is_some_and(Clickable::clickable)
                })
                .collect();
            if !reachable.is_empty() && self.restart_popup(key, cx) {
                return true;
            }
            if let Some(next) = next_match(&reachable, self.key_pos) {
                self.select_item(next, window, cx);
            }
            return true;
        }
        let first = with_key[0];
        match items.into_iter().nth(first) {
            Some(OwnedMenuItem::Action {
                disabled: false,
                action,
                ..
            }) => {
                // The menu closes and the focus goes back, then the item runs where it was.
                self.menu_bar.update(cx, |bar, cx| {
                    bar.set_selected_index(None, window, cx);
                });
                window.dispatch_action(action, cx);
            }
            Some(OwnedMenuItem::Submenu(_)) => {
                if self.restart_popup(key, cx) {
                    return true;
                }
                self.select_item(first, window, cx);
                // The Right key opens the submenu once the popup has drawn the selection (the
                // popup places its submenu while it draws): see `watch_mouse`.
                self.pending_right = 1;
                cx.notify();
            }
            _ => {}
        }
        true
    }

    /// If the mouse may have moved the selection of the popup, builds the popup again (its selection
    /// is `None` then, which is known) and takes `key` once it is drawn. Returns whether it did.
    fn restart_popup(&mut self, key: char, cx: &mut Context<Self>) -> bool {
        if !self.pos_stale {
            return false;
        }
        self.popup_menu = None;
        self._subscription = None;
        self.pending_key = Some(key);
        self.pending_drawn = false;
        cx.notify();
        true
    }

    /// An element that sees every mouse move of the window (the popup may select the item under
    /// the pointer, and a listener on the popup's own element would not hear it, as the popup
    /// occludes what is under it) and every draw.
    fn watch_mouse(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        gpui::canvas(
            |_, _, _| (),
            move |_, (), window, cx| {
                // Every draw: the Right key that waited for the popup to draw its selection. A
                // popup near the right edge of the window opens its submenus to the left, where
                // the Left key goes in and the Right key does not: if the focus did not move, try
                // the other one.
                let step = this.update(cx, |menu, _| menu.pending_right);
                if step == 1 {
                    this.update(cx, |menu, _| menu.pending_right = 2);
                    window.defer(cx, |window, cx| {
                        window.dispatch_action(Box::new(SelectRight), cx);
                    });
                } else if step == 2 {
                    let stayed = this.update(cx, |menu, cx| {
                        menu.pending_right = 0;
                        menu.top_popup_has_focus(window, cx)
                    });
                    if stayed {
                        window.defer(cx, |window, cx| {
                            window.dispatch_action(Box::new(SelectLeft), cx);
                        });
                    }
                }
                window.on_mouse_event(move |_: &MouseMoveEvent, phase, _, cx| {
                    if phase == gpui::DispatchPhase::Capture {
                        this.update(cx, |menu, _| {
                            if menu.popup_menu.is_some() {
                                menu.pos_stale = true;
                            }
                        });
                    }
                });
            },
        )
        .absolute()
        .size_0()
    }

    /// Whether the keys go to the menu itself, not to a submenu.
    fn top_popup_has_focus(&self, window: &Window, cx: &App) -> bool {
        self.popup_menu
            .as_ref()
            .is_some_and(|popup| popup.read(cx).focus_handle(cx).is_focused(window))
    }

    /// The Down key reaches the popup (before it handles it): note where the selection goes.
    fn track_down(&mut self, _: &SelectDown, window: &mut Window, cx: &mut Context<Self>) {
        if self.top_popup_has_focus(window, cx) {
            self.key_pos = Some(after_down(&self.popup_items, self.key_pos));
        }
    }

    /// The Up key reaches the popup: note where the selection goes.
    fn track_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        if self.top_popup_has_focus(window, cx) {
            self.key_pos = Some(after_up(&self.popup_items, self.key_pos));
        }
    }

    /// Moves the selection of the popup to item `target` with the Down key, from where the access
    /// keys left it.
    fn select_item(&mut self, target: usize, window: &mut Window, cx: &mut Context<Self>) {
        // Each press passes `track_down`, which moves `key_pos` as the popup moves.
        for _ in 0..presses_to_reach(&self.popup_items, self.key_pos, target) {
            window.dispatch_action(Box::new(SelectDown), cx);
        }
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "the signature is fixed by GPUI's on_hover listener, which passes &bool"
    )]
    fn handle_hover(&mut self, hovered: &bool, window: &mut Window, cx: &mut Context<Self>) {
        if !*hovered {
            return;
        }
        if !self.menu_bar.read(cx).has_activated_menu() {
            return;
        }
        self.menu_bar.update(cx, |state, cx| {
            state.set_selected_index(Some(self.ix), window, cx);
        });
    }
}

impl Render for BarMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.is_open(cx);
        let marked = is_open || self.menu_bar.read(cx).armed == Some(self.ix);
        if !is_open && self.popup_menu.is_some() {
            // Built again when it opens, with the focus and the window size of that moment.
            self.popup_menu = None;
            self._subscription = None;
        }
        if is_open && self.popup_window_size != window.viewport_size() {
            // The window was resized while the menu is open: size the popup again.
            self.popup_menu = None;
            self._subscription = None;
        }
        let this = cx.entity();

        // The popup is built before the tree. A key that waited for a new popup follows the
        // second render after it was built: by then the popup has been drawn, and the actions
        // that move its selection find it under the focus.
        let popup = is_open.then(|| self.build_popup_menu(window, cx));
        if let Some(key) = self.pending_key {
            if self.pending_drawn {
                self.pending_key = None;
                self.pending_drawn = false;
                let this = cx.entity();
                window.defer(cx, move |window, cx| {
                    this.update(cx, |menu, cx| {
                        menu.access_key(key, window, cx);
                    });
                });
            } else {
                self.pending_drawn = true;
                // Draws the menu once more (a notify inside a render does not).
                let this = cx.entity();
                window.defer(cx, move |_, cx| this.update(cx, |_, cx| cx.notify()));
            }
        }

        div()
            .id(self.ix)
            .relative()
            .on_prepaint(move |bounds, _, cx| this.update(cx, |menu, _| menu.bounds = bounds))
            .child(
                Button::new("menu")
                    .small()
                    .py_0p5()
                    .compact()
                    .ghost()
                    .label(self.name.clone())
                    .accessibility_label(crate::a11y::without_key_hint(&self.name))
                    .open(marked)
                    .on_mouse_down(
                        MouseButton::Left,
                        window.listener_for(&cx.entity(), move |this, _, window, cx| {
                            // Stop propagation to avoid dragging the window.
                            window.prevent_default();
                            cx.stop_propagation();
                            this.toggle(window, cx);
                        }),
                    )
                    .on_click(cx.listener(Self::handle_trigger_click)),
            )
            .on_hover(cx.listener(Self::handle_hover))
            .child(self.watch_mouse(cx))
            .on_key_down(cx.listener(Self::on_key_down))
            .capture_action(cx.listener(Self::track_down))
            .capture_action(cx.listener(Self::track_up))
            .when_some(popup, |this, popup| {
                this.child(deferred(
                    anchored()
                        .anchor(gpui::Anchor::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(
                            div()
                                .size_full()
                                .occlude()
                                .top_1()
                                .debug_selector(|| POPUP_SELECTOR.into())
                                .child(popup),
                        ),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Menu, MenuItem, TestAppContext, actions};

    use super::*;

    actions!(menu_bar_tests, [Nothing]);

    #[test]
    fn an_item_of_a_function_with_an_icon_has_a_picture_and_one_without_has_none() {
        use crate::view::RunFunction;
        // F_FILESAVE has a toolbar button, so an icon; a function with no button has none, and
        // neither has an action that runs no function.
        let save: Box<dyn Action> = Box::new(RunFunction { code: 30103 });
        assert!(function_picture(save.as_ref(), 1.0).is_some());
        assert!(function_picture(save.as_ref(), 2.0).is_some());
        let none: Box<dyn Action> = Box::new(RunFunction { code: 20001 });
        assert!(crate::bars::icon_of_function(20001).is_none());
        assert!(function_picture(none.as_ref(), 1.0).is_none());
        let other: Box<dyn Action> = Box::new(Nothing);
        assert!(function_picture(other.as_ref(), 1.0).is_none());
    }

    #[test]
    fn a_menu_uses_the_window_below_it() {
        // 700 px window, the menu opens 30 px from the top: the rest less the margin.
        assert_eq!(popup_max_height(px(700.), px(30.)), px(700. - 30. - 10.));
        // Longer than gpui-component's 450 px and half of the window.
        assert!(popup_max_height(px(1000.), px(30.)) > px(450.));
        assert!(popup_max_height(px(700.), px(30.)) > px(350.));
    }

    #[test]
    fn a_tiny_window_still_gets_a_usable_menu() {
        assert_eq!(popup_max_height(px(100.), px(30.)), px(MIN_MENU_HEIGHT));
    }

    struct Root {
        menu_bar: Entity<MenuBar>,
        first_focus: FocusHandle,
    }

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(div().id("first").track_focus(&self.first_focus))
                .child(div().h(px(30.)).child(self.menu_bar.clone()))
        }
    }

    fn long_menu() -> Menu {
        let items = (0..40).map(|i| MenuItem::action(format!("item {i}"), Nothing));
        Menu::new("long").items(items)
    }

    fn open_menu(cx: &mut TestAppContext) -> (Entity<MenuBar>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            GlobalState::global_mut(cx).set_app_menus(vec![long_menu().owned()]);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let first_focus = cx.focus_handle();
            first_focus.focus(window, cx);
            Root {
                menu_bar: MenuBar::new(cx),
                first_focus,
            }
        });
        let menu_bar = root.read_with(cx, |root, _| root.menu_bar.clone());
        menu_bar.update_in(cx, |bar, window, cx| {
            bar.set_selected_index(Some(0), window, cx);
        });
        cx.run_until_parked();
        (menu_bar, cx)
    }

    #[gpui::test]
    fn a_long_menu_is_not_limited_to_half_of_the_window(cx: &mut TestAppContext) {
        let (menu_bar, cx) = open_menu(cx);
        let window_height = menu_bar.update_in(cx, |_, window, _| window.viewport_size().height);
        let popup = cx
            .debug_bounds(POPUP_SELECTOR)
            .expect("the open menu is painted");
        // 40 items do not fit: the menu is as tall as the window allows, from its top down to
        // the bottom of the window; gpui-component would stop at half of it.
        assert!(
            popup.size.height > window_height * 0.5,
            "{popup:?} in {window_height:?}"
        );
        assert!(
            popup.bottom() <= window_height,
            "{popup:?} in {window_height:?}"
        );
        assert!(popup.bottom() > window_height - px(40.), "{popup:?}");
    }

    #[gpui::test]
    fn an_open_menu_follows_a_smaller_window(cx: &mut TestAppContext) {
        let (_menu_bar, cx) = open_menu(cx);
        cx.simulate_resize(gpui::size(px(800.), px(500.)));
        cx.run_until_parked();
        let popup = cx
            .debug_bounds(POPUP_SELECTOR)
            .expect("the open menu is painted");
        assert!(popup.bottom() <= px(500.), "{popup:?}");
    }

    const LONG_PATH: &str = r"1 \\wsl.localhost\Ubuntu\home\naomasa\project\tmp\a-very-long-folder-name\another-long-folder-name\EDGE_SUBMISSION.md  [UTF-8]";

    fn open_menu_of(
        items: Vec<MenuItem>,
        cx: &mut TestAppContext,
    ) -> (Entity<MenuBar>, &mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            GlobalState::global_mut(cx).set_app_menus(vec![Menu::new("m").items(items).owned()]);
        });
        let (root, cx) = cx.add_window_view(|window, cx| {
            let first_focus = cx.focus_handle();
            first_focus.focus(window, cx);
            Root {
                menu_bar: MenuBar::new(cx),
                first_focus,
            }
        });
        let menu_bar = root.read_with(cx, |root, _| root.menu_bar.clone());
        menu_bar.update_in(cx, |bar, window, cx| {
            bar.set_selected_index(Some(0), window, cx);
        });
        cx.run_until_parked();
        (menu_bar, cx)
    }

    fn names_of(items: &[OwnedMenuItem]) -> Vec<String> {
        items
            .iter()
            .filter_map(|item| match item {
                OwnedMenuItem::Action { name, .. } => Some(name.clone()),
                OwnedMenuItem::Submenu(menu) => Some(menu.name.to_string()),
                _ => None,
            })
            .collect()
    }

    #[gpui::test]
    fn a_menu_is_as_wide_as_its_longest_item(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(
            vec![
                MenuItem::action("short", Nothing),
                MenuItem::action(LONG_PATH, Nothing),
            ],
            cx,
        );
        let window_width = cx.update(|window, _| window.viewport_size().width);
        let popup = cx
            .debug_bounds(POPUP_SELECTOR)
            .expect("the menu is painted");
        let text = cx.update(|window, _| {
            let style = window.text_style();
            let run = style.to_run(LONG_PATH.len());
            window
                .text_system()
                .layout_line(LONG_PATH, window.rem_size() * 0.875, &[run], None)
                .width
        });
        // gpui-component alone stops at 500 px and the text runs out of the box.
        assert!(text > px(500.), "the path is long: {text:?}");
        assert!(popup.size.width >= text, "{popup:?} for a text of {text:?}");
        assert!(
            popup.size.width <= window_width,
            "{popup:?} in {window_width:?}"
        );
    }

    #[gpui::test]
    fn an_item_wider_than_the_window_is_cut_in_the_middle(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action(LONG_PATH, Nothing)], cx);
        // A narrow window: the path cannot fit.
        cx.simulate_resize(gpui::size(px(500.), px(400.)));
        cx.run_until_parked();
        let popup = cx
            .debug_bounds(POPUP_SELECTOR)
            .expect("the menu is painted");
        assert!(popup.size.width <= px(500.), "{popup:?}");
        let (names, _) = cx.update(|window, _| {
            fit_width(
                vec![MenuItem::action(LONG_PATH, Nothing).owned()],
                window,
                Place::at(window, px(0.)),
                None,
            )
        });
        let shown = &names_of(&names)[0];
        assert!(shown.contains('…'), "{shown}");
        assert!(shown.starts_with(r"1 \\wsl."), "{shown}");
        assert!(shown.ends_with(r"\EDGE_SUBMISSION.md  [UTF-8]"), "{shown}");
    }

    fn at(window: &Window, left: f32) -> Place {
        Place::at(window, px(left))
    }

    #[gpui::test]
    fn a_short_menu_keeps_its_width_and_its_submenu_opens_on_the_right(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        let sub = MenuItem::submenu(Menu::new("sub").items([MenuItem::action("an item", Nothing)]));
        let (items, opening) = cx.update(|window, _| {
            fit_width(
                vec![sub.owned(), MenuItem::action("another", Nothing).owned()],
                window,
                at(window, 0.),
                None,
            )
        });
        // gpui-component's own limit stays: a submenu opens on the right whenever the menu's
        // left edge plus 500 px is inside the window, as before.
        assert_eq!(opening.max_width, px(500.));
        assert!(opening.child.left > px(0.), "{opening:?}");
        assert_eq!(names_of(&items), ["sub", "another"]);
        let popup = cx
            .debug_bounds(POPUP_SELECTOR)
            .expect("the menu is painted");
        assert!(popup.size.width < px(200.), "{popup:?}");
    }

    #[gpui::test]
    fn a_submenu_with_a_long_item_gets_the_room_to_the_edge_of_the_window(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        let sub = MenuItem::submenu(Menu::new("sub").items([MenuItem::action(LONG_PATH, Nothing)]));
        let (width, opening) = cx.update(|window, _| {
            let (_, opening) = fit_width(vec![sub.owned()], window, at(window, 0.), None);
            (window.viewport_size().width, opening)
        });
        // Opens on the right, and the room is what is left of the window.
        assert!(opening.child.left > px(0.), "{opening:?}");
        let end = opening.child.left + opening.child.room;
        assert!(
            (end - (width - px(4.))).abs() < px(1.),
            "{opening:?} in {width:?}"
        );
    }

    #[gpui::test]
    fn a_long_heading_of_a_submenu_in_a_submenu_decides_the_side(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        let heading = |name: &str| {
            let inner = MenuItem::submenu(
                Menu::new(name.to_owned()).items([MenuItem::action("x", Nothing)]),
            );
            MenuItem::submenu(Menu::new("sub").items([inner])).owned()
        };
        let (short, long) = cx.update(|window, _| {
            // A menu 400 px from the right edge: little room on the right.
            let place = Place::at(window, window.viewport_size().width - px(400.));
            (
                fit_width(vec![heading("x")], window, place, None).1,
                fit_width(vec![heading(LONG_PATH)], window, place, None).1,
            )
        });
        // A short heading fits on the right; a long one is measured too, and has more room on
        // the left.
        assert!(short.child.left > long.child.left, "{short:?} {long:?}");
        assert!(long.child.room > short.child.room, "{short:?} {long:?}");
    }

    #[gpui::test]
    fn a_long_path_in_a_submenu_is_cut_to_the_room_where_the_submenu_opens(
        cx: &mut TestAppContext,
    ) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        cx.simulate_resize(gpui::size(px(700.), px(400.)));
        cx.run_until_parked();
        let path = r"2 C:\work\files\a-very-long-folder-name-inside\another-long-folder-name-inside\first-long-file-name.txt  [UTF-8]";
        // The parent at the left edge: the submenu opens on the right; the parent near the right
        // edge: on the left. Either way the shortened label fits where the submenu opens and keeps
        // the file name.
        for left in [0., 500.] {
            let sub =
                MenuItem::submenu(Menu::new("recent").items([MenuItem::action(path, Nothing)]));
            let (shown, room, width_of_shown) = cx.update(|window, _| {
                let (_, opening) = fit_width(vec![sub.owned()], window, at(window, left), None);
                let (items, _) = fit_width(
                    vec![MenuItem::action(path, Nothing).owned()],
                    window,
                    opening.child,
                    None,
                );
                let shown = names_of(&items).remove(0);
                let width = crate::menu_width::Fit::new(window, opening.child, Layout::default())
                    .width_of(&shown, Row::Action);
                (shown, opening.child.room, width)
            });
            assert!(shown.contains('…'), "{left}: {shown}");
            assert!(
                shown.ends_with(r"\first-long-file-name.txt  [UTF-8]"),
                "{left}: {shown}"
            );
            assert!(
                width_of_shown <= room,
                "{left}: {shown} needs {width_of_shown:?} of {room:?}"
            );
        }
    }

    /// The labels of the submenu of a File-like menu at the left edge of a window 584 px wide
    /// (the width the Sandbox check used), as `fill` shortens them.
    fn recent_files_shown(cx: &mut gpui::VisualTestContext, paths: &[&str]) -> Vec<String> {
        cx.simulate_resize(gpui::size(px(584.), px(400.)));
        cx.run_until_parked();
        let items: Vec<_> = paths
            .iter()
            .map(|path| MenuItem::action(*path, Nothing).owned())
            .collect();
        let parent = vec![
            MenuItem::action("新規作成(N)", Nothing).owned(),
            MenuItem::submenu(
                Menu::new("最近使ったファイル(F)")
                    .items(paths.iter().map(|path| MenuItem::action(*path, Nothing))),
            )
            .owned(),
        ];
        cx.update(|window, _| {
            let (_, opening) = fit_width(parent, window, at(window, 0.), None);
            // The room is what is left of the window, from where the submenu opens.
            assert!(opening.child.left > px(0.), "{opening:?}");
            let end = opening.child.left + opening.child.room;
            assert!((end - (window.viewport_size().width - px(4.))).abs() < px(1.));
            names_of(&fit_width(items, window, opening.child, None).0)
        })
    }

    #[gpui::test]
    fn a_long_path_keeps_its_file_name_in_the_room_of_a_narrow_window(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        let shown = recent_files_shown(
            cx,
            &[
                r"1 C:\work\files\a-very-long-folder-name-inside\another-long-folder-name-inside\second-long-file-name.md  [UTF-8]",
            ],
        );
        // The start and `…` and the file name with the encoding stay; the end is not cut.
        assert!(shown[0].starts_with(r"1 C:"), "{}", shown[0]);
        assert!(shown[0].contains('…'), "{}", shown[0]);
        assert!(
            shown[0].ends_with(r"\second-long-file-name.md  [UTF-8]"),
            "{}",
            shown[0]
        );
    }

    #[gpui::test]
    fn a_menu_opened_near_the_right_edge_gives_its_submenu_the_room_beside_where_it_is_drawn(
        cx: &mut TestAppContext,
    ) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        cx.simulate_resize(gpui::size(px(584.), px(400.)));
        cx.run_until_parked();
        let sub = MenuItem::submenu(Menu::new("recent").items([MenuItem::action(
            r"1 C:\work\files\a-very-long-folder-name-inside\second-long-file-name.md  [UTF-8]",
            Nothing,
        )]));
        let (opening, drawn) = cx.update(|window, _| {
            // A right-click 20 px from the right edge: the popup is moved in by `anchored`.
            let asked = window.viewport_size().width - px(20.);
            let (_, opening) = fit_width(vec![sub.owned()], window, Place::at(window, asked), None);
            (opening, asked)
        });
        // The menu is drawn at least 8 px inside the window, and the submenu on its left ends 16 px
        // before it: the room is smaller than the room beside where the click was.
        assert!(
            opening.child.left + opening.child.room <= drawn - px(16.),
            "{opening:?}"
        );
        assert!(
            opening.child.left + opening.child.room < drawn - px(16.) - px(1.),
            "{opening:?}"
        );
    }

    #[gpui::test]
    fn each_submenu_on_the_left_starts_where_its_own_width_puts_it(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        cx.simulate_resize(gpui::size(px(584.), px(400.)));
        cx.run_until_parked();
        let wide = MenuItem::submenu(Menu::new("wide").items([MenuItem::action(
            r"1 C:\work\files\a-very-long-folder-name-inside\second-long-file-name.md  [UTF-8]",
            Nothing,
        )]));
        let narrow = MenuItem::submenu(Menu::new("narrow").items([MenuItem::action("x", Nothing)]));
        let (opening, widths) = cx.update(|window, _| {
            // A menu near the right edge: both submenus open on the left.
            let place = Place::at(window, window.viewport_size().width - px(20.));
            let (_, opening, widths) =
                fit_widths(vec![wide.owned(), narrow.owned()], window, place, None);
            (opening, widths)
        });
        let (wide_at, narrow_at) = (opening.child_for(widths[0]), opening.child_for(widths[1]));
        assert!(widths[0] > widths[1], "{widths:?}");
        // They end at the same edge, 16 px before the parent, so the narrow one starts farther
        // right: its own nested submenus are placed from there.
        assert!(narrow_at.left > wide_at.left, "{wide_at:?} {narrow_at:?}");
        assert_eq!(
            narrow_at.left + widths[1],
            wide_at.left + widths[0].min(wide_at.room)
        );
    }

    #[gpui::test]
    fn a_submenu_with_little_room_is_cut_to_it_down_to_the_least_width(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        let (rem, narrow, least) = cx.update(|window, _| {
            let rem = window.rem_size();
            let path =
                r"1 C:\work\files\a-very-long-folder-name-inside\second-long-file-name.md  [UTF-8]";
            let width_for = |room: Pixels| {
                let place = Place {
                    left: px(300.),
                    room,
                    margin: px(4.),
                };
                let fit = crate::menu_width::Fit::new(window, place, Layout::default());
                fit.width_of(&fit.owned(path.to_owned(), Row::Action), Row::Action)
            };
            // 160 px is less than the 200 px the room used to be raised to, and more than 8 rem.
            (rem, width_for(px(160.)), width_for(px(60.)))
        });
        // The labels are cut to the room that is really there...
        assert!(narrow <= px(160.), "{narrow:?}");
        // ...and a room below 8 rem is raised to 8 rem, which gpui-component draws anyway.
        assert!(least <= rem * 8.0, "{least:?}");
        assert!(least > px(60.), "{least:?}");
    }

    #[gpui::test]
    fn a_path_that_fits_the_room_is_not_shortened(cx: &mut TestAppContext) {
        let (_bar, cx) = open_menu_of(vec![MenuItem::action("short", Nothing)], cx);
        let path = r"3 C:\work\files\p473.txt  [UTF-8]";
        let shown = recent_files_shown(cx, &[path]);
        assert_eq!(shown, [path]);
    }

    #[gpui::test]
    fn the_menu_closes_and_the_focus_goes_back(cx: &mut TestAppContext) {
        let (menu_bar, cx) = open_menu(cx);
        menu_bar.update_in(cx, |bar, window, cx| {
            bar.set_selected_index(None, window, cx);
        });
        cx.run_until_parked();
        menu_bar.read_with(cx, |bar, cx| {
            assert!(bar.action_context.is_none());
            assert!(bar.menus[0].read(cx).popup_menu.is_none());
        });
    }
}

#[cfg(test)]
mod key_tests;
