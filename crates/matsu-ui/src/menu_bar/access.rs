//! The logic of the access keys of the menu bar, apart from GPUI: which item a letter means, how
//! far the selection of a popup has to move, and when a tap of Alt counts.
//!
//! Sakura: `CMenuDrawer::OnMenuChar` (`sakura_core/uiparts/CMenuDrawer.cpp`) answers `WM_MENUCHAR`
//! for the letters typed in an open menu: no item with the key is ignored, one is executed, several
//! are selected one after the other (the one after the highlighted item, round the end).

use crate::a11y::key_hint;
use gpui::{Modifiers, OwnedMenuItem};

/// The access key of an item, upper-cased, read from the `(K)` mark of its name.
pub(super) fn item_key(item: &OwnedMenuItem) -> Option<char> {
    match item {
        OwnedMenuItem::Action { name, .. } => key_hint(name),
        OwnedMenuItem::Submenu(submenu) => key_hint(&submenu.name),
        OwnedMenuItem::Separator | OwnedMenuItem::SystemMenu(_) => None,
    }
}

/// The items as the popup menu holds them, so that an index here is an index there: it drops what
/// `fill` does not add (system menus) and what `PopupMenu::separator` refuses (a separator first or
/// right after another).
pub(super) fn as_in_popup(items: Vec<OwnedMenuItem>) -> Vec<OwnedMenuItem> {
    let mut out: Vec<OwnedMenuItem> = Vec::with_capacity(items.len());
    for item in items {
        match item {
            OwnedMenuItem::SystemMenu(_) => {}
            OwnedMenuItem::Separator
                if out
                    .last()
                    .is_none_or(|last| matches!(last, OwnedMenuItem::Separator)) => {}
            item => out.push(item),
        }
    }
    out
}

/// The character of a key that can be an access key (a letter or a digit), upper-cased.
pub(super) fn key_of(name: &str) -> Option<char> {
    let mut chars = name.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    c.is_ascii_alphanumeric().then(|| c.to_ascii_uppercase())
}

/// The item to go to for a letter that several items share: the first after `current`, else the
/// first one (`OnMenuChar` returns `MNC_SELECT` for the item after the highlighted one).
pub(super) fn next_match(matches: &[usize], current: Option<usize>) -> Option<usize> {
    let after = current.and_then(|c| matches.iter().copied().find(|&m| m > c));
    after.or_else(|| matches.first().copied())
}

/// Whether the popup menu lets the selection stop on this item (gpui-component skips separators and
/// gray items).
pub(super) fn is_clickable(item: &OwnedMenuItem) -> bool {
    match item {
        OwnedMenuItem::Action { disabled, .. } => !disabled,
        OwnedMenuItem::Submenu(_) => true,
        OwnedMenuItem::Separator | OwnedMenuItem::SystemMenu(_) => false,
    }
}

/// Where the selection of a popup menu goes on the Down key (gpui-component's `select_down`): the
/// first press selects the first item, the others the next clickable item, round the end.
pub(super) fn after_down(items: &[OwnedMenuItem], at: Option<usize>) -> usize {
    match at {
        None => 0,
        Some(ix) => (ix + 1..items.len())
            .find(|&i| is_clickable(&items[i]))
            .unwrap_or(0),
    }
}

/// Where the selection goes on the Up key (`select_up`): the previous clickable item, round the
/// start to the last one.
pub(super) fn after_up(items: &[OwnedMenuItem], at: Option<usize>) -> usize {
    let ix = at.unwrap_or(0);
    (0..ix)
        .rev()
        .find(|&i| is_clickable(&items[i]))
        .or_else(|| (0..items.len()).rev().find(|&i| is_clickable(&items[i])))
        .unwrap_or(0)
}

/// How many times the Down key has to be pressed in a popup menu whose selection is at `from`
/// (`None`: nothing selected) to reach `target`; 0 if `target` cannot be reached.
pub(super) fn presses_to_reach(
    items: &[OwnedMenuItem],
    from: Option<usize>,
    target: usize,
) -> usize {
    let mut at = from;
    for presses in 1..=items.len() + 1 {
        let next = after_down(items, at);
        at = Some(next);
        if next == target {
            return presses;
        }
    }
    0
}

/// Tells a tap of Alt (press and release with nothing else in between) from Alt used with another
/// key or with the mouse. A lone Alt marks the menu bar, as in Windows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum AltTap {
    #[default]
    Idle,
    /// Alt is down; `clean` while nothing else happened.
    Held { clean: bool },
}

impl AltTap {
    /// The modifiers changed. Returns true when this is the release of a tap.
    pub(super) fn changed(&mut self, modifiers: Modifiers) -> bool {
        let others =
            modifiers.control || modifiers.shift || modifiers.platform || modifiers.function;
        if modifiers.alt {
            // Alt with another modifier is not a lone Alt, and stays so until Alt is released.
            *self = match (*self, others) {
                (_, true) => Self::Held { clean: false },
                (Self::Idle, false) => Self::Held { clean: true },
                (held, false) => held,
            };
            return false;
        }
        let before = std::mem::take(self);
        !others && before == (Self::Held { clean: true })
    }

    /// A key or a mouse button went down: this is not a lone Alt any more.
    pub(super) fn other_input(&mut self) {
        if let Self::Held { clean } = self {
            *clean = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Menu, MenuItem, actions};

    actions!(access_tests, [Nothing]);

    fn alt() -> Modifiers {
        Modifiers {
            alt: true,
            ..Modifiers::default()
        }
    }

    #[test]
    fn a_letter_or_a_digit_is_a_key_and_nothing_else() {
        assert_eq!(key_of("t"), Some('T'));
        assert_eq!(key_of("7"), Some('7'));
        assert_eq!(key_of("enter"), None);
        assert_eq!(key_of("-"), None);
        assert_eq!(key_of(""), None);
    }

    #[test]
    fn several_items_with_a_key_are_visited_in_turn() {
        let matches = [2, 5, 9];
        assert_eq!(next_match(&matches, None), Some(2));
        assert_eq!(next_match(&matches, Some(2)), Some(5));
        assert_eq!(next_match(&matches, Some(4)), Some(5));
        assert_eq!(next_match(&matches, Some(9)), Some(2));
        assert_eq!(next_match(&[], Some(1)), None);
    }

    fn items() -> Vec<OwnedMenuItem> {
        Menu::new("m")
            .items([
                MenuItem::action("a(A)", Nothing),
                MenuItem::separator(),
                MenuItem::action("b(B)", Nothing),
                MenuItem::submenu(Menu::new("c(C)").items([MenuItem::action("x", Nothing)])),
            ])
            .owned()
            .items
    }

    #[test]
    fn the_items_are_listed_as_the_popup_holds_them() {
        let items = Menu::new("m")
            .items([
                MenuItem::separator(),
                MenuItem::action("a(A)", Nothing),
                MenuItem::separator(),
                MenuItem::separator(),
                MenuItem::action("b(B)", Nothing),
            ])
            .owned()
            .items;
        let listed = as_in_popup(items);
        assert_eq!(listed.len(), 3);
        assert_eq!(item_key(&listed[0]), Some('A'));
        assert_eq!(item_key(&listed[2]), Some('B'));
    }

    #[test]
    fn the_selection_moves_over_separators() {
        let items = items();
        assert_eq!(item_key(&items[0]), Some('A'));
        assert_eq!(item_key(&items[1]), None);
        assert_eq!(item_key(&items[3]), Some('C'));
        assert_eq!(presses_to_reach(&items, None, 0), 1);
        assert_eq!(presses_to_reach(&items, None, 2), 2);
        assert_eq!(presses_to_reach(&items, Some(0), 2), 1);
        assert_eq!(presses_to_reach(&items, Some(2), 3), 1);
        // Round the end.
        assert_eq!(presses_to_reach(&items, Some(3), 0), 1);
        assert_eq!(presses_to_reach(&items, Some(3), 2), 2);
        assert_eq!(presses_to_reach(&items, Some(0), 1), 0);
        assert_eq!(after_down(&items, None), 0);
        assert_eq!(after_up(&items, None), 3);
        assert_eq!(after_up(&items, Some(2)), 0);
        assert_eq!(after_up(&items, Some(3)), 2);
    }

    #[test]
    fn a_lone_alt_is_a_tap_and_alt_with_something_is_not() {
        let mut tap = AltTap::default();
        assert!(!tap.changed(alt()));
        assert!(tap.changed(Modifiers::default()));
        // Used with a key (or the mouse) in between.
        assert!(!tap.changed(alt()));
        tap.other_input();
        assert!(!tap.changed(Modifiers::default()));
        // Alt and Shift together (the layout switch) is not a tap either.
        assert!(!tap.changed(alt()));
        assert!(!tap.changed(Modifiers {
            alt: true,
            shift: true,
            ..Modifiers::default()
        }));
        assert!(!tap.changed(Modifiers::default()));
        // Shift let go first: the Alt that is left is still not a lone one.
        assert!(!tap.changed(alt()));
        assert!(!tap.changed(Modifiers {
            alt: true,
            shift: true,
            ..Modifiers::default()
        }));
        assert!(!tap.changed(alt()));
        assert!(!tap.changed(Modifiers::default()));
        // A release without a press is nothing.
        assert!(!tap.changed(Modifiers::default()));
    }
}
