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

use crate::app::Workspace;
use crate::menu_bar::access::{AltTap, item_key, key_of, next_match, presses_to_reach};
use crate::tabs::content::MenuKind;
use gpui::{
    Action, App, AppContext as _, Bounds, ClickEvent, Context, DismissEvent, Entity, FocusHandle,
    Focusable, InteractiveElement as _, IntoElement, KeyDownEvent, MouseButton, OwnedMenu,
    OwnedMenuItem, ParentElement, Pixels, Render, Role, SharedString, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, anchored, deferred, div, prelude::FluentBuilder, px,
};
use gpui_base::ElementExt as _;
use gpui_base::actions::{Cancel, SelectDown, SelectLeft, SelectRight};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::PopupMenu;
use gpui_component::{GlobalState, InteractiveElementExt as _, Selectable, Sizable, h_flex};

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
        // A kind of tab that has no state of the editor leaves the items as they are made.
        let Some(states) = workspace
            .read(cx)
            .menu_states(topmost, workspace.entity_id(), cx)
        else {
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
            .iter()
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
        if self.selected_index.is_none() && ix.is_some() {
            self.action_context = window.focused(cx);
        } else if ix.is_none() {
            if let Some(action_context) = self.action_context.as_ref() {
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

pub(crate) fn fill(
    mut menu: PopupMenu,
    items: Vec<OwnedMenuItem>,
    action_context: Option<&FocusHandle>,
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
                menu.menu_element_with_disabled(
                    Action::boxed_clone(action.as_ref()),
                    disabled,
                    move |_, _| {
                        crate::menu_icons::item_element(
                            SharedString::from(name.clone()),
                            picture.clone(),
                            checked,
                            disabled,
                            scale,
                        )
                    },
                )
            }
            OwnedMenuItem::Action {
                name,
                action,
                checked,
                disabled,
                ..
            } => menu.menu_with_check_and_disabled(
                name,
                checked,
                Action::boxed_clone(action.as_ref()),
                disabled,
            ),
            OwnedMenuItem::Separator => menu.separator(),
            OwnedMenuItem::Submenu(submenu) => {
                let context = action_context.cloned();
                let icon = with_icons.then(|| {
                    gpui_component::Icon::empty().mr(px(crate::menu_icons::submenu_padding(scale)))
                });
                menu.submenu_with_icon(icon, submenu.name, window, cx, move |sub, window, cx| {
                    let sub = match &context {
                        Some(handle) => sub.action_context(handle.clone()),
                        None => sub,
                    };
                    // As gpui-component does: a long submenu scrolls within its default height.
                    let scrollable = submenu.items.len() > 20;
                    fill(sub, submenu.items.clone(), context.as_ref(), window, cx)
                        .scrollable(scrollable)
                })
            }
            OwnedMenuItem::SystemMenu(_) => menu,
        };
    }
    menu
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
    /// The height of the window that `popup_menu` was sized for.
    popup_window_height: Pixels,
    /// Where the button was at the last paint, to size the popup below it.
    bounds: Bounds<Pixels>,
    /// The item that the access keys selected last in the open popup (the popup does not tell
    /// which item is selected, so the keys move the selection from here).
    key_pos: Option<usize>,
    _subscription: Option<Subscription>,
}

impl BarMenu {
    fn new(ix: usize, menu: &OwnedMenu, menu_bar: Entity<MenuBar>, cx: &mut App) -> Entity<Self> {
        let name = menu.name.clone();
        cx.new(|_| Self {
            ix,
            menu_bar,
            name,
            menu: menu.clone(),
            popup_menu: None,
            popup_window_height: Pixels::ZERO,
            bounds: Bounds::default(),
            key_pos: None,
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
                let max_height = popup_max_height(
                    window.viewport_size().height,
                    self.bounds.bottom() + px(POPUP_OFFSET),
                );
                let popup_menu = PopupMenu::build(window, cx, |menu, window, cx| {
                    let menu = match &action_context {
                        Some(handle) => menu.action_context(handle.clone()),
                        None => menu,
                    };
                    fill(menu, items, action_context.as_ref(), window, cx)
                        .scrollable(true)
                        .max_h(max_height)
                });
                self._subscription =
                    Some(cx.subscribe_in(&popup_menu, window, Self::handle_dismiss));
                self.popup_menu = Some(popup_menu.clone());
                self.key_pos = None;
                self.popup_window_height = window.viewport_size().height;
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
        _: &Entity<PopupMenu>,
        _: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self._subscription.take();
        self.popup_menu.take();
        self.menu_bar.update(cx, |state, cx| {
            state.on_cancel(&Cancel, window, cx);
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
    /// not matter. Only the menu itself is handled; once a submenu has the focus, its letters are
    /// left alone.
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
        let (Some(key), Some(popup)) = (key_of(&keystroke.key), self.popup_menu.clone()) else {
            return;
        };
        if !popup.read(cx).focus_handle(cx).is_focused(window) {
            return;
        }
        let mut items = self.menu.items.clone();
        MenuBar::mark_items(&self.menu_bar, &mut items, window, cx);
        let matches: Vec<usize> = (0..items.len())
            .filter(|&i| item_key(&items[i]) == Some(key))
            .collect();
        let Some(&first) = matches.first() else {
            return;
        };
        cx.stop_propagation();
        if matches.len() > 1 {
            let next = next_match(&matches, self.key_pos).unwrap_or(first);
            self.select_item(&items, next, window, cx);
            return;
        }
        match &items[first] {
            OwnedMenuItem::Action {
                disabled: false,
                action,
                ..
            } => {
                let action = Action::boxed_clone(action.as_ref());
                // The menu closes and the focus goes back, then the item runs where it was.
                self.menu_bar.update(cx, |bar, cx| {
                    bar.set_selected_index(None, window, cx);
                });
                window.dispatch_action(action, cx);
            }
            OwnedMenuItem::Submenu(_) => {
                self.select_item(&items, first, window, cx);
                window.dispatch_action(Box::new(SelectRight), cx);
            }
            _ => {}
        }
    }

    /// Moves the selection of the popup to item `target` with the Down key, from where the access
    /// keys left it.
    fn select_item(
        &mut self,
        items: &[OwnedMenuItem],
        target: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for _ in 0..presses_to_reach(items, self.key_pos, target) {
            window.dispatch_action(Box::new(SelectDown), cx);
        }
        self.key_pos = Some(target);
    }

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
        if is_open && self.popup_window_height != window.viewport_size().height {
            // The window was resized while the menu is open: size the popup again.
            self.popup_menu = None;
            self._subscription = None;
        }
        let this = cx.entity();

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
            .on_key_down(cx.listener(Self::on_key_down))
            .when(is_open, |this| {
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
                                .child(self.build_popup_menu(window, cx)),
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
