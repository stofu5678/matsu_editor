//! The roles and names of the tab bar for assistive technology: a tab list of tabs named by their
//! captions (the active one selected), and the buttons at the right end named by what they do.

use gpui::{Div, Role, Stateful, StatefulInteractiveElement as _};

/// The name of the tab bar.
pub(super) const BAR_NAME: &str = "タブ";
/// The name of the ▼ button (its tooltip says more: the captions on a left click, the paths on a
/// right click).
pub(super) const LIST_BUTTON_NAME: &str = "タブの一覧";

pub(super) fn bar(element: Stateful<Div>) -> Stateful<Div> {
    element.role(Role::TabList).aria_label(BAR_NAME)
}

/// A tab with `caption`, number `position` (from 1) of `count`.
pub(super) fn tab(
    element: Stateful<Div>,
    caption: &str,
    active: bool,
    position: usize,
    count: usize,
) -> Stateful<Div> {
    element
        .role(Role::Tab)
        .aria_label(caption.to_owned())
        .aria_selected(active)
        .aria_position_in_set(position)
        .aria_size_of_set(count)
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
        let bar = node_of(&bar(div().id("bar"))).unwrap();
        assert_eq!((bar.role(), bar.label()), (Role::TabList, Some(BAR_NAME)));

        let active = node_of(&tab(div().id("t0"), "main.rs", true, 1, 2)).unwrap();
        let other = node_of(&tab(div().id("t1"), "名前のない文書", false, 2, 2)).unwrap();
        assert_eq!(
            (active.role(), active.label()),
            (Role::Tab, Some("main.rs"))
        );
        assert_eq!(active.is_selected(), Some(true));
        assert_eq!(other.label(), Some("名前のない文書"));
        assert_eq!(other.is_selected(), Some(false));
        assert_eq!(other.position_in_set(), Some(2));
        assert_eq!(other.size_of_set(), Some(2));
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
