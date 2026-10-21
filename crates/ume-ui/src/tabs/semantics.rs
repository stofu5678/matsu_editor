//! The roles and names of the tab bar for assistive technology: a tab list of tabs named by their
//! captions (the active one selected), and the buttons at the right end named by what they do.

use gpui::{Div, Role, Stateful, StatefulInteractiveElement as _};

/// The name of the tab bar.
pub(super) const BAR_NAME: &str = "タブ";
/// The name of the ▼ button (its tooltip says more: the captions on a left click, the paths on a
/// right click).
pub(super) const LIST_BUTTON_NAME: &str = "タブの一覧";

// AccessKit counts `position_in_set` from 0 (accesskit_windows 0.34.0 `node.rs` 681-686 adds 1 for
// UIA's `PositionInSet`) and takes the size of the set from the container, never from the item
// (`size_of_set_from_container`, accesskit_consumer 0.38.0 `node.rs` 612-624, and node.rs 688-692 of
// accesskit_windows): so the item has the zero-based position and the list has the count.
/// The tab list, with `count` tabs.
pub(super) fn bar(element: Stateful<Div>, count: usize) -> Stateful<Div> {
    element
        .role(Role::TabList)
        .aria_label(BAR_NAME)
        .aria_size_of_set(count)
}

/// A tab with `caption`, at `index` (from 0) in the tab list.
pub(super) fn tab(
    element: Stateful<Div>,
    caption: &str,
    active: bool,
    index: usize,
) -> Stateful<Div> {
    element
        .role(Role::Tab)
        .aria_label(caption.to_owned())
        .aria_selected(active)
        .aria_position_in_set(index)
}

/// The close button of the tab with `caption`: `タブを閉じる: caption`.
pub(super) fn tab_close_button(element: Stateful<Div>, caption: &str) -> Stateful<Div> {
    element.role(Role::Button).aria_label(format!(
        "{}: {caption}",
        super::bar::close_tooltip("F_WINCLOSE")
    ))
}

/// A button of the right end, named `name`.
pub(super) fn button(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element.role(Role::Button).aria_label(name.to_owned())
}

#[cfg(test)]
mod tests {
    use gpui::{InteractiveElement as _, div};

    use super::*;
    use crate::a11y::node_of;

    #[test]
    fn the_bar_is_a_tab_list_and_the_active_tab_is_selected() {
        let bar = node_of(&bar(div().id("bar"), 2)).unwrap();
        assert_eq!((bar.role(), bar.label()), (Role::TabList, Some(BAR_NAME)));
        // What UIA reads: the size from the container, the zero-based position from the item
        // (accesskit_windows 0.34.0 node.rs 681-692; accesskit_consumer 0.38.0 node.rs 612-624).
        assert_eq!(bar.size_of_set(), Some(2));

        let active = node_of(&tab(div().id("t0"), "main.rs", true, 0)).unwrap();
        let other = node_of(&tab(div().id("t1"), "名前のない文書", false, 1)).unwrap();
        assert_eq!(
            (active.role(), active.label()),
            (Role::Tab, Some("main.rs"))
        );
        assert_eq!(active.is_selected(), Some(true));
        assert_eq!(other.label(), Some("名前のない文書"));
        assert_eq!(other.is_selected(), Some(false));
        assert_eq!(active.position_in_set(), Some(0));
        assert_eq!(other.position_in_set(), Some(1));
        assert_eq!(other.size_of_set(), None);
    }

    #[test]
    fn the_buttons_are_named_by_what_they_do() {
        let close = node_of(&tab_close_button(div().id("c"), "main.rs")).unwrap();
        assert_eq!(close.role(), Role::Button);
        assert_eq!(close.label(), Some("タブを閉じる: main.rs"));
        let list = node_of(&button(div().id("l"), LIST_BUTTON_NAME)).unwrap();
        assert_eq!(list.label(), Some("タブの一覧"));
    }
}
