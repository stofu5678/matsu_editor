//! `CreateMenu`: the pop-up menu of a macro.
//!
//! The menu string is parsed by `matsu_plugin::functions::menu`; this shows it in the popup of the
//! custom menus (`crate::custom_menu`) and answers the number of the item the user chose, or 0.
//!
//! Sakura: `sakura_core/macro/CMacro.cpp`, `F_CREATEMENU`: `TrackPopupMenu` at the caret (if it is
//! inside the text area, else at the pointer) or at the pointer, left and top aligned, kept inside
//! the work area of the monitor.

use std::future::Future;
use std::rc::Rc;

use gpui::{Context, DismissEvent, FocusHandle, Focusable as _, Pixels, Point, Window, point, px};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use matsu_plugin::functions::menu::{MenuNode, item_count};

use crate::common_settings::model::parse_label;
use crate::menu_bar::popup_keys::{self, KeyList, Rebuild};
use crate::menu_bar::popup_max_height;
use crate::menu_width::{Fit, Opening, Place};
use crate::view::EditorView;

/// The menu text as it is drawn: `&A` is `A` (GPUI's menus have no access keys) and `&&` is `&`.
fn plain_label(label: &str) -> String {
    parse_label(label).0
}

/// The text of an item of the menu: the label without its access key, cut if it is wider than
/// the window (`menu_width`).
fn shown(label: &str, window: &Window, place: Place) -> String {
    Fit::new(window, place).owned(plain_label(label))
}

/// How the menu of `nodes` at `place` opens: its `max_w` (as wide as its longest label) and the
/// place of its submenus.
fn opening_of(nodes: &[MenuNode], window: &Window, place: Place) -> Opening {
    let fit = Fit::new(window, place);
    let width_of = |label: &str| fit.width_of(&shown(label, window, place));
    let natural = |label: &str| fit.natural(&plain_label(label));
    let (mut own, mut submenu) = (Pixels::ZERO, Pixels::ZERO);
    for node in nodes {
        match node {
            MenuNode::Item { label, .. } => own = own.max(width_of(label)),
            MenuNode::Submenu {
                label, children, ..
            } => {
                own = own.max(width_of(label));
                // The items and the headings of the submenus in it.
                for child in children {
                    if let MenuNode::Item { label, .. } | MenuNode::Submenu { label, .. } = child {
                        submenu = submenu.max(natural(label));
                    }
                }
            }
            MenuNode::Separator => {}
        }
    }
    fit.opening(own, submenu)
}

/// The rows of `nodes` as the letters see them: the key is the one after `&` in the label.
fn key_list(nodes: &[MenuNode]) -> KeyList {
    let mut keys = KeyList::new();
    for node in nodes {
        match node {
            MenuNode::Item {
                label, disabled, ..
            } => keys.item(parse_label(label).1, !disabled),
            MenuNode::Separator => keys.separator(),
            // Shown as a gray item (see `fill_nodes`).
            MenuNode::Submenu {
                label,
                disabled,
                children,
                ..
            } if *disabled || item_count(children) == 0 => keys.item(parse_label(label).1, false),
            MenuNode::Submenu { label, .. } => keys.submenu(parse_label(label).1),
        }
    }
    keys
}

/// Adds `nodes` to `menu`. Choosing an item sends its number on `answer`.
fn fill_nodes(
    mut menu: PopupMenu,
    nodes: &[MenuNode],
    answer: &async_channel::Sender<i64>,
    focus: &FocusHandle,
    place: Place,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let opening = opening_of(nodes, window, place);
    for node in nodes {
        menu = match node {
            MenuNode::Item {
                label,
                number,
                checked,
                disabled,
                radio,
            } => {
                let answer = answer.clone();
                let number = *number;
                // `CheckMenuRadioItem(i, i, i)`: an item with the flag R is the checked one of its
                // own one-item group.
                let item = PopupMenuItem::new(shown(label, window, place))
                    .checked(*checked || *radio)
                    .disabled(*disabled);
                // The menu of GPUI can confirm a disabled item with the keys; it gives no number.
                menu.item(if *disabled {
                    item
                } else {
                    item.on_click(move |_, _, _| {
                        let _ = answer.try_send(number);
                    })
                })
            }
            MenuNode::Separator => menu.separator(),
            // A submenu that is disabled cannot be opened, and an empty one has nothing to show.
            MenuNode::Submenu {
                label,
                disabled,
                children,
                ..
            } if *disabled || item_count(children) == 0 => {
                menu.item(PopupMenuItem::new(shown(label, window, place)).disabled(true))
            }
            MenuNode::Submenu {
                label, children, ..
            } => {
                // The submenu takes the letters too, as the one of `menu_bar::fill` does.
                let keys = key_list(children);
                let child = opening.child;
                let (children, answer, focus) = (children.clone(), answer.clone(), focus.clone());
                let fill_again: Rebuild = Rc::new(move |sub, window, cx| {
                    let sub = sub.action_context(focus.clone());
                    fill_nodes(sub, &children, &answer, &focus, child, window, cx)
                });
                menu.submenu(
                    shown(label, window, place),
                    window,
                    cx,
                    move |sub, window, cx| {
                        popup_keys::register(&cx.entity(), keys.clone(), fill_again.clone(), cx);
                        fill_again(sub, window, cx)
                    },
                )
            }
        };
    }
    menu.max_w(opening.max_width)
}

impl EditorView {
    /// Where `CreateMenu` shows its menu: at the caret if `at_caret` and the caret is inside the
    /// text area, else at the pointer (`F_CREATEMENU`: `CalcCaretDrawPos` and `GetCursorPos`).
    fn macro_menu_position(
        &self,
        at_caret: bool,
        window: &Window,
        cx: &gpui::App,
    ) -> Point<Pixels> {
        if at_caret
            && let Some(paint) = self.paint
            && let Some(at) = self.point_of(self.cursor.caret(), cx)
        {
            let left = px(paint.origin_x);
            let top = px(paint.origin_y);
            let right = left + px(self.metrics.view_width);
            let bottom = top + px(self.metrics.view_height);
            if left <= at.x && top <= at.y && at.x < right && at.y < bottom {
                return at;
            }
        }
        let pointer = window.mouse_position();
        point(pointer.x, pointer.y)
    }

    /// Shows the menu of `nodes` and returns the answer to wait for: the number of the item
    /// chosen, or 0 when the menu is closed without a choice or has no item.
    pub(crate) fn open_macro_menu(
        &mut self,
        at_caret: bool,
        nodes: &[MenuNode],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl Future<Output = i64> + use<> {
        let (sender, receiver) = async_channel::bounded(1);
        if item_count(nodes) > 0 {
            let at = self.macro_menu_position(at_caret, window, cx);
            let focus = self.focus_handle.clone();
            let max_height = popup_max_height(window.viewport_size().height, at.y);
            let place = Place::at(window, at.x);
            let keys = key_list(nodes);
            let (nodes, answer) = (nodes.to_vec(), sender.clone());
            let fill_again: Rebuild = Rc::new(move |menu, window, cx| {
                let menu = menu.action_context(focus.clone());
                fill_nodes(menu, &nodes, &answer, &focus, place, window, cx)
                    .scrollable(true)
                    .max_h(max_height)
            });
            let build = fill_again.clone();
            let popup =
                PopupMenu::build(window, cx, move |menu, window, cx| build(menu, window, cx));
            popup_keys::register(&popup, keys, fill_again, cx);
            // An item sends its number before the menu is dismissed, and the first answer wins.
            let closed = sender;
            let dismiss = cx.subscribe_in(
                &popup,
                window,
                move |this, popup, _: &DismissEvent, window, cx| {
                    let _ = closed.try_send(0);
                    let moved = crate::menu_bar::focus_left_popup(popup, window, cx);
                    this.close_custom_menu_keeping_focus(moved, window, cx);
                },
            );
            popup.read(cx).focus_handle(cx).focus(window, cx);
            self.custom_menu
                .show(popup, at, window.viewport_size(), dismiss);
            cx.notify();
        }
        // With no item there is no sender left: the wait ends at once with 0.
        async move { receiver.recv().await.unwrap_or(0) }
    }
}
