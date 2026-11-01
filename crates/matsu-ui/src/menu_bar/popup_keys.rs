//! The access keys of the popup menus that MATSU opens by itself: the right-click menu and the
//! other custom menus (the tab menu, the menu of the status bar, the drop-down of the open button),
//! the outline pane's menu and the menu of a macro's `CreateMenu`.
//!
//! The menus of the menu bar have their own handling (`BarMenu::on_key_down`); here the popup is
//! not inside a bar, so the letters come through the keystroke interceptor of the window
//! (`Workspace::watch_menu_keys`), which runs before the key bindings and reacts to nothing while
//! the IME composes. The rules are those of the bar, which are the rules of Win32
//! (`CMenuDrawer::OnMenuChar` in `sakura_core/uiparts/CMenuDrawer.cpp`):
//!
//! * a letter that no item has is ignored,
//! * the item that has it runs (a submenu opens),
//! * with several, the selection goes to the next of them, round the end, and nothing runs.
//!
//! Once a submenu has the focus its own letters are left alone, as in the bar.
//!
//! gpui-component's `PopupMenu` does not tell its selection, so the registry follows the Up and
//! Down keys and its own moves. If the mouse moved since (it selects what it points at), the
//! selection is not known and the popup is built again, with nothing selected, before the letter
//! is used.

use std::rc::Rc;

use gpui::{
    App, Context, Entity, Focusable as _, Global, Keystroke, OwnedMenuItem, WeakEntity, Window,
};
use gpui_base::actions::{Confirm, SelectDown, SelectLeft, SelectRight};
use gpui_component::menu::PopupMenu;

use super::access::{
    Clickable, after_down, after_up, as_in_popup, is_clickable, item_key, key_of, next_match,
    presses_to_reach,
};

/// How a popup is filled, to fill it again.
pub(crate) type Rebuild = Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

/// One row of a popup as far as the letters are concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyItem {
    key: Option<char>,
    clickable: bool,
    submenu: bool,
}

impl Clickable for KeyItem {
    fn clickable(&self) -> bool {
        self.clickable
    }
}

/// The rows of a popup, numbered as the popup numbers its items: a separator in front or right
/// after another is not there (`PopupMenu::separator` refuses it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyList {
    items: Vec<KeyItem>,
    /// The last row is a separator (or there is none yet).
    separated: bool,
}

impl KeyList {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            separated: true,
        }
    }

    /// The rows of owned menu items.
    pub(crate) fn of_owned(items: Vec<OwnedMenuItem>) -> Self {
        let mut list = Self::new();
        for item in as_in_popup(items) {
            match item {
                OwnedMenuItem::Separator => list.separator(),
                OwnedMenuItem::Submenu(_) => list.submenu(item_key(&item)),
                OwnedMenuItem::Action { .. } => list.item(item_key(&item), is_clickable(&item)),
                OwnedMenuItem::SystemMenu(_) => {}
            }
        }
        list
    }

    /// An item that runs something; `clickable` is false for a gray one.
    pub(crate) fn item(&mut self, key: Option<char>, clickable: bool) {
        self.push(key, clickable, false);
    }

    /// An item that opens a submenu.
    pub(crate) fn submenu(&mut self, key: Option<char>) {
        self.push(key, true, true);
    }

    fn push(&mut self, key: Option<char>, clickable: bool, submenu: bool) {
        self.separated = false;
        self.items.push(KeyItem {
            key: key.map(|k| k.to_ascii_uppercase()),
            clickable,
            submenu,
        });
    }

    pub(crate) fn separator(&mut self) {
        if !self.separated {
            self.separated = true;
            self.items.push(KeyItem {
                key: None,
                clickable: false,
                submenu: false,
            });
        }
    }
}

/// What a letter does in a popup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    /// No item has the letter: it is not for the popup.
    Ignore,
    /// The letter is taken and nothing happens (the only item with it is gray, or all are).
    Taken,
    /// The selection goes to this item.
    Select(usize),
    /// The selection goes to this item and it runs (a submenu opens).
    Run { target: usize, submenu: bool },
}

impl KeyList {
    /// What `key` does when the selection is at `pos`.
    pub(crate) fn plan(&self, key: char, pos: Option<usize>) -> Plan {
        let key = key.to_ascii_uppercase();
        let with_key: Vec<usize> = (0..self.items.len())
            .filter(|&i| self.items[i].key == Some(key))
            .collect();
        match with_key.as_slice() {
            [] => Plan::Ignore,
            &[only] => {
                if self.items[only].clickable {
                    Plan::Run {
                        target: only,
                        submenu: self.items[only].submenu,
                    }
                } else {
                    Plan::Taken
                }
            }
            _ => {
                let reachable: Vec<usize> = with_key
                    .into_iter()
                    .filter(|&i| self.items[i].clickable)
                    .collect();
                next_match(&reachable, pos).map_or(Plan::Taken, Plan::Select)
            }
        }
    }
}

struct Entry {
    popup: WeakEntity<PopupMenu>,
    keys: KeyList,
    rebuild: Rebuild,
    /// The selection of the popup, as far as it is known.
    pos: Option<usize>,
    /// The count of mouse moves (`Registry::moves`) when `pos` was last right.
    moves: u64,
}

/// The popups that are open (or were; the dead ones are dropped when another one registers).
#[derive(Default)]
struct Registry {
    entries: Vec<Entry>,
    /// How many mouse moves the window has seen. A move can change the selection (the popup selects
    /// what the pointer is on) and the pointer can come back to where it was, so the position of the
    /// pointer cannot tell.
    moves: u64,
}

impl Global for Registry {}

/// Makes the letters work in `popup`, which was filled by `rebuild` with the rows `keys`.
pub(crate) fn register(popup: &Entity<PopupMenu>, keys: KeyList, rebuild: Rebuild, cx: &mut App) {
    let registry = cx.default_global::<Registry>();
    registry.entries.retain(|e| e.popup.upgrade().is_some());
    let moves = registry.moves;
    registry.entries.push(Entry {
        popup: popup.downgrade(),
        keys,
        rebuild,
        pos: None,
        moves,
    });
}

/// An element that counts the mouse moves of the window, to be a child of the window's root (it
/// is drawn every frame and listens before the popups, which occlude what is under them).
pub(crate) fn watch_mouse() -> impl gpui::IntoElement {
    use gpui::Styled as _;
    gpui::canvas(
        |_, _, _| (),
        |_, (), window, _| {
            window.on_mouse_event(|_: &gpui::MouseMoveEvent, phase, _, cx| {
                if phase == gpui::DispatchPhase::Capture {
                    cx.default_global::<Registry>().moves += 1;
                }
            });
        },
    )
    .absolute()
    .size_0()
}

/// How many mouse moves the window has seen.
#[cfg(test)]
pub(crate) fn moves(cx: &App) -> u64 {
    cx.try_global::<Registry>().map_or(0, |r| r.moves)
}

/// A keystroke arrived before the key bindings see it. Returns whether the popup took it.
pub(crate) fn key(keystroke: &Keystroke, window: &mut Window, cx: &mut App) -> bool {
    if cx
        .try_global::<Registry>()
        .is_none_or(|r| r.entries.is_empty())
        || keystroke.is_ime_in_progress()
    {
        return false;
    }
    let m = keystroke.modifiers;
    if m.control || m.alt || m.platform || m.function {
        return false;
    }
    // The popup that has the focus itself (not one of its submenus: those keep their letters).
    let focused = cx.global::<Registry>().entries.iter().position(|e| {
        e.popup
            .upgrade()
            .is_some_and(|p| p.read(cx).focus_handle(cx).is_focused(window))
    });
    let Some(at) = focused else {
        return false;
    };
    let registry = cx.global_mut::<Registry>();
    let now = registry.moves;
    let entry = &mut registry.entries[at];
    // Only the plain arrows are bound to the popup's Up and Down.
    match keystroke.key.as_str() {
        "down" | "up" if m.shift => return false,
        "down" => {
            entry.pos = Some(after_down(&entry.keys.items, entry.pos));
            return false;
        }
        "up" => {
            entry.pos = Some(after_up(&entry.keys.items, entry.pos));
            return false;
        }
        _ => {}
    }
    let Some(letter) = key_of(&keystroke.key) else {
        return false;
    };
    let plan = entry.keys.plan(letter, entry.pos);
    if plan == Plan::Ignore {
        return false;
    }
    let (Some(popup), keys, rebuild, mut pos, seen) = (
        entry.popup.upgrade(),
        entry.keys.clone(),
        entry.rebuild.clone(),
        entry.pos,
        entry.moves,
    ) else {
        return false;
    };
    // The mouse may have selected something: start again from nothing selected.
    let mut plan = plan;
    if now != seen {
        popup.update(cx, |menu, cx| {
            menu.rebuild(window, cx, |menu, window, cx| rebuild(menu, window, cx));
        });
        pos = None;
        plan = keys.plan(letter, pos);
    }
    if let Plan::Select(target) | Plan::Run { target, .. } = plan {
        let presses = if pos == Some(target) {
            0
        } else {
            presses_to_reach(&keys.items, pos, target)
        };
        for _ in 0..presses {
            window.dispatch_action(Box::new(SelectDown), cx);
            pos = Some(after_down(&keys.items, pos));
        }
        if let Plan::Run { submenu, .. } = plan {
            if submenu {
                open_submenu(&popup, window);
            } else {
                window.dispatch_action(Box::new(Confirm { secondary: false }), cx);
            }
        }
    }
    let id = popup.downgrade();
    let registry = cx.global_mut::<Registry>();
    let moves = registry.moves;
    if let Some(e) = registry.entries.iter_mut().find(|e| e.popup == id) {
        e.pos = pos;
        e.moves = moves;
    }
    true
}

/// Opens the submenu that is selected. The popup places its submenu while it draws, so the Right
/// key waits for a frame; near the right edge the submenu is on the left, where the Left key goes
/// in and the Right key does not.
fn open_submenu(popup: &Entity<PopupMenu>, window: &mut Window) {
    let popup = popup.clone();
    window.on_next_frame(move |window, cx| {
        window.dispatch_action(Box::new(SelectRight), cx);
        window.on_next_frame(move |window, cx| {
            if popup.read(cx).focus_handle(cx).is_focused(window) {
                window.dispatch_action(Box::new(SelectLeft), cx);
            }
        });
    });
    window.refresh();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A, separator, A, B, gray C, a submenu D.
    fn list() -> KeyList {
        let mut keys = KeyList::new();
        keys.item(Some('a'), true);
        keys.separator();
        keys.item(Some('A'), true);
        keys.item(Some('B'), true);
        keys.item(Some('C'), false);
        keys.submenu(Some('D'));
        keys
    }

    #[test]
    fn a_letter_that_one_item_has_runs_it() {
        let keys = list();
        assert_eq!(
            keys.plan('b', None),
            Plan::Run {
                target: 3,
                submenu: false
            }
        );
        // Case does not matter.
        assert_eq!(keys.plan('B', Some(0)), keys.plan('b', None));
        assert_eq!(
            keys.plan('d', None),
            Plan::Run {
                target: 5,
                submenu: true
            }
        );
    }

    #[test]
    fn a_letter_that_several_items_have_goes_round_them() {
        let keys = list();
        assert_eq!(keys.plan('a', None), Plan::Select(0));
        assert_eq!(keys.plan('a', Some(0)), Plan::Select(2));
        assert_eq!(keys.plan('a', Some(1)), Plan::Select(2));
        assert_eq!(keys.plan('a', Some(2)), Plan::Select(0));
        assert_eq!(keys.plan('a', Some(5)), Plan::Select(0));
    }

    #[test]
    fn a_letter_without_an_item_is_not_taken_and_a_gray_item_takes_it_for_nothing() {
        let keys = list();
        assert_eq!(keys.plan('z', None), Plan::Ignore);
        assert_eq!(keys.plan('c', None), Plan::Taken);
    }

    #[test]
    fn the_separators_the_popup_refuses_are_not_counted() {
        let mut keys = KeyList::new();
        keys.separator();
        keys.item(Some('x'), true);
        keys.separator();
        keys.separator();
        keys.item(Some('y'), true);
        // x is 0, the separator 1, y 2.
        assert_eq!(
            keys.plan('y', None),
            Plan::Run {
                target: 2,
                submenu: false
            }
        );
    }
}
